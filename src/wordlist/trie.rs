

use std::collections::BinaryHeap;
use std::fmt::{Debug, Formatter};


use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;

use crate::regex::nfa::graph::{NfaGraph, NfaResult};
use crate::regex::nfa::state::NfaStateKind::Accept;
use crate::regex::nfa::state::NfaStatePtr;


use crate::wordlist::index::Index;
use crate::wordlist::trienode::{OrderedTrieNode, TrieNodeRef};

pub struct Trie {
    root: TrieNodeRef,
}

impl Index for Trie {
    fn add(&mut self, word: &str) {
        self.root.insert(word);
    }

    fn contains(&self, word: &str) -> bool {
        return self.get_node(word, Some(self.root.clone()))
            .map(|x| x.is_terminal()).unwrap_or(false);
    }


    fn add_all<'f, I>(&mut self, items: I)
        where I: Iterator<Item=&'f str> {
        self.add_all_(items);
        self.decorate()
    }
}


impl Trie {
    fn new() -> Trie {
        Trie {
            root: Default::default()
        }
    }


    fn decorate(&mut self) {
        self.root.traverse_postfix(&mut |node:TrieNodeRef| {
            node.set_weight(
                node.map_child(&mut |x| x.weight())
                    .iter()
                    .fold(1, |x, y| x + y));
            });
    }

    fn query_regex(&self, regex: &str) -> Vec<String> {
        self.query_regex_graph_weighted(&NfaGraph::from_regex(regex), |x| x.weight())
    }

    fn query_regex_graph_weighted<'f, T: 'f>(&'f self,
                                             regex: &NfaGraph,
                                             weight: fn(TrieNodeRef) -> T) -> Vec<String>
        where T: Ord, T: Debug {
        #[derive(Ord, PartialOrd, Eq, PartialEq)]
        struct QItem<T: Ord + Debug> (OrderedTrieNode<T>, Vec<NfaStatePtr>);
        type PriorityQueue<T> = BinaryHeap<QItem<T>>;

        let mut pq = PriorityQueue::new();
        let mut results :Vec<String>= vec![];

        fn add_successful_children<T>(pq: &mut PriorityQueue<T>,
                                          weight: fn(TrieNodeRef) -> T,
                                          child: TrieNodeRef,
                                          result: NfaResult)
            where T: Ord + Debug {
            if !result.states.is_empty() {
                pq.push(QItem(child.order::<T>(weight), result.states));
            }
        }

        for child in self.root.clone() {
            let lstring = &*child.letter().to_string();
            let result = regex.apply(lstring);
            add_successful_children(&mut pq, weight, child, result);
        }

        while !pq.is_empty() {
            let QItem(node, states) = pq.pop().unwrap();

            println!("Popping {:?}, {:?}", node, states);
            if node.is_terminal() {
                if states.iter().any(|x| x.kind_is(&Accept)) {
                    results.push(node.path().to_string());
                }
            }

            for child in node.node.clone() {
                let lstring = &*child.letter().to_string();
                let result = regex.apply_with_start(lstring, &states);

                add_successful_children(&mut pq, weight, child, result);
            }
        }
        results
    }

    fn get_node<'f>(&self, word: &str, node: Option<TrieNodeRef>) -> Option<TrieNodeRef>
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
        let mut seq = serializer.serialize_seq(Some(self.root.weight()))?;
        self.root.traverse_prefix(&mut |x| seq.serialize_element(&x));
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Trie {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_seq(DeserializeTrieVisitor {})
    }
}

struct DeserializeTrieVisitor {}

impl<'de> Visitor<'de> for DeserializeTrieVisitor {
    type Value = Trie;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a TrieNode")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let trie = Trie::new();
        let mut stack: Vec<TrieNodeRef> = vec![trie.root.clone()];

        seq.next_element::<TrieNodeRef>(); //discard root; we already have it

        while let Some(thing) = seq.next_element()? {
            {
                //println!("{:?}, {:?}", thing, stack);
            }

            let node: TrieNodeRef = thing;
            {
                while node.depth() < stack.len() {
                    stack.pop();
                }
            }
            let _letter = node.letter();


            let mut parent = stack.pop().unwrap();
            parent.set_child(node.clone());
            stack.push(parent);
            stack.push(node);
        }
        Ok(trie)
    }
}


impl Debug for Trie {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        self.root.traverse_prefix(&mut |x| { l.entries(x); });
        l.finish()
    }
}


#[cfg(test)]
mod tests {
    
    use crate::wordlist::index::Index;
    use crate::wordlist::trie::Trie;

    #[test]
    fn finds_words_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let mut trie = Trie::new();
        trie.add_all((&words).iter().map(|x| *x));
        (&words).iter().for_each(|word| assert!(trie.contains(&word)));
    }

    #[test]
    fn doesnt_finds_words_not_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let bad_words = vec!["HE", "H", "LOL", "BANANA"];
        let mut trie = Trie::new();
        trie.add_all((&words).iter().map(|x| *x));
        (&bad_words).iter().for_each(|word| assert!(!trie.contains(&word)));
    }


    #[test]
    fn query_words_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let mut trie = Trie::new();
        trie.add_all(words.into_iter());

        let mut result = trie.query_regex("H.L*(O|P)");
        result.sort();

        assert_eq!(result, vec!["HELLO", "HELP"])
    }

    #[test]
    fn test_serialize_deserialize() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let mut trie = Trie::new();
        trie.add_all(words.clone().into_iter());

        let serialized = serde_json::to_string(&trie).unwrap();
        let new_trie = serde_json::from_str::<Trie>(&serialized).unwrap();

        println!("{:#?}", new_trie);

        (&words).iter().for_each(|word| assert!(new_trie.contains(&word)));
    }
}