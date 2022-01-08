
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::time::Instant;
use maplit::hashmap;


use serde::{Deserialize, Deserializer, Serialize, Serializer};

use serde::ser::SerializeSeq;


use crate::regex::nfa::graph::{NfaGraph};
use crate::regex::nfa::state::NfaStateKind::Accept;
use crate::regex::nfa::state::NfaStatePtr;



use trie_builder::TrieBuilder;
use crate::wordlist::trie::haschildren::HasChildren;
use crate::wordlist::trie::trienode::{OrderedTrieNode, TrieNode};

mod trie_builder;
mod trienode;
mod mutablenode;
mod haschildren;

pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn builder() -> TrieBuilder {
        TrieBuilder::new()
    }

    pub(crate) fn new(root: TrieNode) -> Trie {
        Trie { root }
    }

    pub(crate) fn contains(&self, word: &str) -> bool {
        return self.get_node(word, Some(&self.root))
            .map(|x| x.is_terminal).unwrap_or(false);
    }


    fn best_first_search<State, Score, Accept, KeepGoing>
    (&self, accept: Accept, keep_going: KeepGoing,
     score: for<'r> fn(&'r TrieNode) -> Score, starting_state: State,
    ) -> Vec<String>
        where Score: Ord + PartialEq + Eq + PartialOrd + Debug,
              Accept: Fn(&State) -> bool,
              KeepGoing: Fn(&State, char) -> Option<State>,
              State: Eq + PartialEq
    {
        #[derive(Eq, PartialEq)]
        struct QItem<'q, Score: Ord + Debug, State> (OrderedTrieNode<'q, Score>, State);

        impl<Score, State> PartialOrd<Self> for QItem<'_, Score, State>
            where Score: Eq + PartialEq + Ord + Debug, State: PartialEq + Eq {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl<Score, State> Ord for QItem<'_, Score, State>
            where Score: Eq + PartialEq + Ord + Debug, State: Eq {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0.cmp(&other.0)
            }
        }

        type PriorityQueue<'q, T, State> = BinaryHeap<QItem<'q, T, State>>;

        let mut pq = PriorityQueue::new();
        let mut results: Vec<String> = vec![];

        pq.push(QItem(self.root.order::<Score>(score), starting_state));

        while !pq.is_empty() {
            let QItem(node, state) = pq.pop().unwrap();

            if node.is_terminal {
                if accept(&state) {
                    results.push(node.path.to_string());
                }
            }

            for child in node.node {
                if let Some(new_state) = keep_going(&state, child.letter) {
                    pq.push(QItem(child.order::<Score>(score), new_state))
                }
            }
        }
        results
    }


    pub fn query_regex(&self, regex: &str) -> Vec<String> {
        let nfa = &NfaGraph::from_regex(regex);

        self.best_first_search(|state: &Vec<NfaStatePtr>| state.iter().any(|x| x.kind_is(&Accept)),
                               |state: &Vec<NfaStatePtr>, c: char| {
                                   let lstring = c.to_string();
                                   let result = nfa.apply_with_start(&lstring, &state);
                                   if result.states.is_empty() {
                                       None
                                   } else {
                                       Some(result.states)
                                   }
                               },
                               |x| x.weight,
                               nfa.starting_states())
    }

    fn get_counts(word: &str) -> HashMap<char, usize> {
        let mut counts = hashmap! {};

        word.chars().for_each(|c| {
            *counts.entry(c).or_insert(0) += 1;
        });
        counts
    }

    pub fn query_anagram(&self, word: &str) -> Vec<String> {
        self.best_first_search(|counts: &HashMap<char, usize>| counts.values().all(|x| *x == 0),
                               |counts: &HashMap<char, usize>, c: char| {
                                   if *counts.get(&c).unwrap_or(&0) > 0 {
                                       let mut new_counts = counts.clone();
                                       *new_counts.get_mut(&c).unwrap() -= 1;
                                       Some(new_counts)
                                   } else { None }
                               },
                               |x| x.weight,
                               Self::get_counts(word),
        )
    }


    fn get_node<'f>(&self, word: &str, node: Option<&'f TrieNode>) -> Option<&'f TrieNode>
    {
        if word.is_empty() {
            return node;
        }
        if node.is_none() {
            return None;
        }
        let fst = word.chars().nth(0).unwrap();
        return self.get_node(&word[1..],
                             node.unwrap().get_child(fst));
    }
}

impl Serialize for Trie {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.root.weight))?;
        self.root.traverse_prefix(&mut |x| seq.serialize_element(&x));
        seq.end()
    }
}


impl Debug for Trie {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        self.root.traverse_prefix(&mut |x| { l.entries(x); });
        l.finish()
    }
}

impl From<&mut TrieBuilder> for Trie {
    fn from(builder: &mut TrieBuilder) -> Self {
        let start_decorate = Instant::now();
        builder.decorate();
        println!("Decoration took {}", start_decorate.elapsed().as_secs_f64());
        Trie::new((&builder.root).into())
    }
}

impl<'de> Deserialize<'de> for Trie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(TrieBuilder::deserialize(deserializer)?.build())
    }
}

#[cfg(test)]
mod tests {
    use crate::wordlist::index::Index;
    use crate::wordlist::trie::Trie;

    #[test]
    fn finds_words_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::builder()
            .add_all((&words).iter().map(|x| *x))
            .build();
        (&words).iter().for_each(|word| assert!(trie.contains(&word)));
    }

    #[test]
    fn doesnt_finds_words_not_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let bad_words = vec!["HE", "H", "LOL", "BANANA"];
        let trie = Trie::builder().add_all(words).build();
        (&bad_words).iter().for_each(|word| assert!(!trie.contains(&word)));
    }


    #[test]
    fn query_words_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::builder().add_all(words).build();

        let mut result = trie.query_regex("H.L*(O|P)");
        result.sort();

        assert_eq!(result, vec!["HELLO", "HELP"])
    }

    #[test]
    fn test_serialize_deserialize() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::builder().add_all(words.clone()).build();
        let serialized = serde_json::to_string(&trie).unwrap();
        let new_trie = serde_json::from_str::<Trie>(&serialized).unwrap();

        (&words).iter().for_each(|word| assert!(new_trie.contains(&word)));
    }

    #[test]
    fn test_anagram() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::builder().add_all(words.clone()).build();

        assert_eq!(trie.query_anagram("OLEHL"), vec!["HELLO"]);
        assert!(trie.query_anagram("LEHL").is_empty());
        assert!(trie.query_anagram("LELO").is_empty());
        assert!(trie.query_anagram("DOG").is_empty());
        assert_eq!(trie.query_anagram("OOGD"), vec!["GOOD"]);

    }
}
