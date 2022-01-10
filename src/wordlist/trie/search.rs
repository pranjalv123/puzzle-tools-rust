use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Debug;
use std::ops::Deref;
use maplit::hashmap;
use crate::regex::nfa::graph::NfaGraph;
use crate::regex::nfa::state::NfaStateKind::Accept;
use crate::regex::nfa::state::NfaStatePtr;
use crate::wordlist::trie::node::{TrieNode};
use crate::wordlist::trie::trie::Trie;

impl<'a> Trie<'a> {

    pub(crate) fn contains(&'a self, word: &str) -> bool {
        return self.get_node(word, Some(&self.root))
            .map(|x| x.is_terminal.get()).unwrap_or(false);
    }

    fn best_first_search<State, Score, Accept, KeepGoing>
    (&'a self, accept: Accept, keep_going: KeepGoing,
     score: for<'r> fn(&'r TrieNode) -> Score, starting_state: State,
    ) -> Vec<String>
        where Score: Ord + PartialEq + Eq + PartialOrd + Debug,
              Accept: Fn(&State) -> bool,
              KeepGoing: Fn(&State, char) -> Option<State>,
              State: Eq + PartialEq
    {
        assert!(self.built.get());
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

            if node.is_terminal.get() {
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


    pub fn query_regex(&'a self, regex: &str) -> Vec<String> {
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
                               |x| x.weight.get(),
                               nfa.starting_states())
    }

    fn get_counts(word: &str) -> HashMap<char, usize> {
        let mut counts = hashmap! {};

        word.chars().for_each(|c| {
            *counts.entry(c).or_insert(0) += 1;
        });
        counts
    }

    pub fn query_anagram(&'a self, word: &str) -> Vec<String> {
        self.best_first_search(|counts: &HashMap<char, usize>| counts.values().all(|x| *x == 0),
                               |counts: &HashMap<char, usize>, c: char| {
                                   if *counts.get(&c).unwrap_or(&0) > 0 {
                                       let mut new_counts = counts.clone();
                                       *new_counts.get_mut(&c).unwrap() -= 1;
                                       Some(new_counts)
                                   } else { None }
                               },
                               |x| x.weight.get(),
                               Self::get_counts(word),
        )
    }


    fn get_node<'f>(&self, word: &str, node: Option<&'f TrieNode<'f>>) -> Option<&'f TrieNode<'f>>
    {
        assert!(self.built.get());
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


#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    val: T,
    node: &'a TrieNode<'a>,
}

impl<'a> TrieNode<'a> {
    fn order<T>(&'a self, f: fn(&'a TrieNode) -> T) -> OrderedTrieNode<T>
        where T: Ord, T: Debug {
        OrderedTrieNode { val: f(self), node: self }
    }
}

impl<'a, T> From<&'a TrieNode<'a>> for OrderedTrieNode<'a, T>
    where T: Default + Ord + Debug {
    fn from(node: &'a TrieNode<'a>) ->
    Self {
        OrderedTrieNode::<'a, T> {
            val: Default::default(),
            node,
        }
    }
}

impl<'a, T> Deref for OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    type Target = TrieNode<'a>;

    fn deref(&self) -> &TrieNode<'a> {
        self.node
    }
}
