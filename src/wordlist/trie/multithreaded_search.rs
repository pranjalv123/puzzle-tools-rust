use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Debug;
use std::sync::Mutex;
use maplit::hashmap;
use rayon::{scope, Scope};
use crate::regex::nfa::graph::NfaGraph;
use crate::regex::nfa::state::NfaStateKind::Accept;
use crate::regex::nfa::state::NfaStatePtr;
use crate::wordlist::trie::node::{OrderedTrieNode, TrieNode};
use crate::wordlist::trie::trie::Trie;

#[derive(Eq, PartialEq)]
struct QItem<'q, Score: Ord + Debug, State> (OrderedTrieNode<'q, Score>, State);

impl<State, Score> PartialOrd<Self> for QItem<'_, Score, State>
    where Score: Debug + Eq + Ord + PartialEq + PartialOrd, State: Eq + PartialEq {
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

impl<'a> Trie<'a> {
    fn best_first_search<State, Score, Accept, KeepGoing>
    (&'a self,
     accept: fn(&State) -> bool,
     keep_going: fn(&State, char) -> Option<State>,
     score: for<'r> fn(&'r TrieNode) -> Score,
     starting_state: State,
    ) -> Vec<String>
        where Score: Ord + PartialEq + Eq + PartialOrd + Debug + Send,
              State: Eq + PartialEq + Send
    {
        assert!(self.built.get());

        let mut pq = Mutex::new(PriorityQueue::new());
        let mut results: Mutex<Vec<String>> = Mutex::new(vec![]);

        pq.lock().unwrap().push(QItem(self.root.order::<Score>(score), starting_state));

        scope(|s| {
            s.spawn(|s| Self::worker(&pq, &results, s, accept, keep_going, score))
        });

        results.into_inner().unwrap()
    }

    fn worker<State, Score, Accept, KeepGoing>(pq: &Mutex<PriorityQueue<Score, State>>, result: &Mutex<Vec<String>>, s: &Scope,
                                               accept: fn(&State) -> bool,
                                               keep_going: fn(&State, char) -> Option<State>,
                                               score: for<'r> fn(&'r TrieNode) -> Score)
        where Score: Ord + PartialEq + Eq + PartialOrd + Debug + Send,
              Accept: Fn(&State) -> bool + Sync,
              KeepGoing: Fn(&State, char) -> Option<State> + Sync,
              State: Eq + PartialEq + Send {
        let QItem(node, state) = pq.lock().unwrap().pop().unwrap();

        if node.is_terminal.get() {
            if accept(&state) {
                result.lock().unwrap().push(node.path.to_string());
            }
        }
        let mut locked = pq.lock().unwrap();
        for child in node.node {
            if let Some(new_state) = keep_going(&state, child.letter) {
                locked.push(QItem(child.order::<Score>(score), new_state));
                s.spawn(|s| Self::worker(pq, result, s, accept, keep_going, score));
            }
        }
    }


    pub fn query_regex_multithreaded(&'a self, regex: &str) -> Vec<String> {
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

    pub fn query_anagram_multithreaded(&'a self, word: &str) -> Vec<String> {
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
}