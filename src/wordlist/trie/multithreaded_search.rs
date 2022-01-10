use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Debug;
use std::ops::{Deref};
use std::sync::{Arc, Mutex};
use maplit::hashmap;

use rayon::{scope, Scope};
use crate::regex::nfa::graph::NfaGraph;
use crate::regex::nfa::state::NfaStateKind::Accept;

use crate::wordlist::trie::node::{ImmutableTrieNode};
use crate::wordlist::trie::trie::{ImmutableTrie, Trie};


struct QItem<'q, Score: Ord + Debug, State> (OrderedTrieNode<'q, Score>, State);

impl<State, Score> Eq for QItem<'_, Score, State> where Score: Debug + Eq + Ord + PartialEq + PartialOrd {}

impl<State, Score> PartialEq<Self> for QItem<'_, Score, State> where Score: Debug + Eq + Ord + PartialEq + PartialOrd {
    fn eq(&self, other: &Self) -> bool {
        self.0.val == other.0.val
    }
}

impl<State, Score> PartialOrd<Self> for QItem<'_, Score, State>
    where Score: Debug + Eq + Ord + PartialEq + PartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Score, State> Ord for QItem<'_, Score, State>
    where Score: Eq + PartialEq + Ord + Debug {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

type PriorityQueue<'q, Score, State> = BinaryHeap<QItem<'q, Score, State>>;

impl<'a, 'scope> ImmutableTrie<'a> {
    fn best_first_search<'f, State, Score>
    (&'f self,
     accept: fn(&State) -> bool,
     keep_going: fn(&State, char) -> Option<State>,
     score: for<'r> fn(&'r ImmutableTrieNode) -> Score,
     starting_state: State,
    ) -> Vec<String>
        where Score: Ord + PartialEq + Eq + PartialOrd + Debug + Send + 'f,
              State: Send + 'f
    {
        let pq: Arc<Mutex<PriorityQueue<'f, _, _>>> = Arc::new(Mutex::new(PriorityQueue::new()));
        let results: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let item = QItem(self.root.get().unwrap().order(score), starting_state);
        scope(|scope| {
            let r = results.clone();
            let pq = pq.clone();
            scope.spawn(move |s| {
                Self::worker(item,
                             pq, r, accept, keep_going, score, s);
            })
        });

        let x = results.deref().lock().unwrap().clone();
        x
    }

    fn worker<'f, State, Score>(item: QItem<'f, Score, State>,
        pq: Arc<Mutex<PriorityQueue<'f, Score, State>>>, results: Arc<Mutex<Vec<String>>>,
                                        accept: for<'r> fn(&'r State) -> bool,
                                        keep_going: for<'r> fn(&'r State, char) -> Option<State>,
                                        score: for<'r> fn(&'r ImmutableTrieNode) -> Score,
                                        scope: &Scope<'scope>,
    )
        where Score: Ord + PartialEq + Eq + PartialOrd + Debug + Send + 'f,
              State: Send + 'f,
              'f: 'scope
    {
        let mut pq_l = pq.lock().unwrap();
        pq_l.push(item);
        let QItem(node, state) = pq_l.pop().unwrap();
        drop(pq_l);
        if node.is_terminal {
            if accept(&state) {
                let mut r = results.lock().unwrap();
                r.push(node.path.to_string());
                drop(r);
            }
        }
        let mut count = 0;
        let mut to_add = vec![];
        for child in node.node {
            if let Some(new_state) = keep_going(&state, child.letter) {
                to_add.push(QItem(child.order::<Score>(score), new_state));
                count += 1
            }
        }

        for add in to_add.into_iter() {
            let results = results.clone();
            let pq = pq.clone();
            scope.spawn(move |s| {
                Self::worker(add, pq, results, accept, keep_going, score, s);
            })
        }
    }


    pub fn query_regex_multithreaded(&self, regex: &str) -> Vec<String> {
        let nfa = NfaGraph::from_regex(regex);


        self.best_first_search(|state| state.1.iter().any(|x| x.kind_is(&Accept)),
                               |state, c: char| {
                                   let lstring = c.to_string();
                                   let result = state.0.apply_with_start(&lstring, &state.1);
                                   if result.states.is_empty() {
                                       None
                                   } else {
                                       Some((state.0, result.states))
                                   }
                               },
                               |x| x.weight,
                               (&nfa, nfa.starting_states()))
    }

    fn get_counts(word: &str) -> HashMap<char, usize> {
        let mut counts = hashmap! {};

        word.chars().for_each(|c| {
            *counts.entry(c).or_insert(0) += 1;
        });
        counts
    }

    pub fn query_anagram_multithreaded(&self, word: &str) -> Vec<String> {
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
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    val: T,
    node: &'a ImmutableTrieNode<'a>,
}

impl ImmutableTrieNode<'_> {
    fn order<'a, T>(&'a self, f: fn(&'a ImmutableTrieNode) -> T) -> OrderedTrieNode<'a, T>
        where T: Ord, T: Debug {
        OrderedTrieNode { val: f(self), node: self }
    }
}

impl<'a, T> From<&'a ImmutableTrieNode<'a>> for OrderedTrieNode<'a, T>
    where T: Default + Ord + Debug {
    fn from(node: &'a ImmutableTrieNode) ->
    Self {
        OrderedTrieNode::<'a, T> {
            val: Default::default(),
            node,
        }
    }
}

impl<'a, T> Deref for OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    type Target = ImmutableTrieNode<'a>;

    fn deref(&self) -> &ImmutableTrieNode<'a> {
        self.node
    }
}


#[test]
fn test_anagram_multithreaded() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
    let mut mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let mut trie = ImmutableTrie::new();
    mut_trie.build(&trie);

    assert_eq!(trie.query_anagram_multithreaded("OLEHL"), vec!["HELLO"]);
    assert!(trie.query_anagram_multithreaded("LEHL").is_empty());
    assert!(trie.query_anagram_multithreaded("LELO").is_empty());
    assert!(trie.query_anagram_multithreaded("DOG").is_empty());
    assert_eq!(trie.query_anagram_multithreaded("OOGD"), vec!["GOOD"]);
}

#[test]
fn query_words_in_trie() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
    let mut mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let immut = ImmutableTrie::new();
    mut_trie.build(&immut);

    let mut result = immut.query_regex_multithreaded("H.L*(O|P)");
    result.sort();

    assert_eq!(result, vec!["HELLO", "HELP"])
}
