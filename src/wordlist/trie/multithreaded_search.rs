use std::cmp::Ordering;
use std::collections::{HashMap};
use std::fmt::Debug;

use std::ops::{Deref};
use std::sync::{Arc};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool};

use maplit::hashmap;


use rayon::{scope, Scope};

use crate::regex::nfa::graph::NfaGraph;
use crate::regex::nfa::state::NfaStateKind::Accept;
use crate::regex::nfa::state::NfaStatePtr;
use crate::wordlist::trie::concurrent_pq;
use crate::wordlist::trie::concurrent_pq::ConcurrentPQ;

use crate::wordlist::trie::node::{ImmutableTrieNode};
use crate::wordlist::trie::searchconfig::SearchConfig;
use crate::wordlist::trie::trie::{ImmutableTrie, Trie};


trait StateT: Send + Debug {}

pub trait ResultCallback: FnMut(String, &SearchConfig) -> bool + Sync + Send {}

impl<F: FnMut(String, &SearchConfig) -> bool + Sync + Send> ResultCallback for F {}

//#[derive(Debug)]
struct QItem<'q, State: StateT + 'q>(OrderedTrieNode<'q>, SearchState<'q>, State);

impl<'q, State: StateT> concurrent_pq::QItem for QItem<'q, State> {}

impl<State: StateT> PartialEq<Self> for QItem<'_, State> {
    fn eq(&self, other: &Self) -> bool {
        self.0.val == other.0.val
    }
}

impl<State: StateT> Eq for QItem<'_, State> {}

impl<State: StateT> PartialOrd<Self> for QItem<'_, State> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<State: StateT> Ord for QItem<'_, State> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[derive(Copy, Clone)]
struct SearchParams<State: StateT> {
    accept: for<'r> fn(&'r State) -> bool,
    keep_going: for<'r> fn(&'r State, char) -> Option<State>,
    score: for<'r> fn(&'r ImmutableTrieNode, &SearchState) -> isize,
}

struct SearchState<'a> {
    num_spaces: usize,
    current_word_len: usize,
    prev_words: Vec<&'a ImmutableTrieNode<'a>>,
    total_len: usize,
    prev_penalty: isize,
}

impl<'a> Clone for SearchState<'a> {
    fn clone(&self) -> Self {
        SearchState {
            num_spaces: self.num_spaces,
            current_word_len: self.current_word_len,
            prev_words: self.prev_words.clone(),
            total_len: self.total_len,
            prev_penalty: self.prev_penalty,
        }
    }
}

impl<'a> SearchState<'a> {
    fn new_word(&self, node: &'a ImmutableTrieNode<'a>, config: &SearchConfig) -> SearchState<'a> {
        let mut new = self.clone();
        new.num_spaces += 1;
        new.current_word_len = 0;
        new.prev_words.push(node);
        new.prev_penalty += config.space_penalty.unwrap() as isize - node.freq as isize;
        new
    }
    fn same_word(&self) -> SearchState<'a> {
        let mut new = self.clone();
        new.current_word_len += 1;
        new.total_len += 1;
        new
    }
}


impl<'a, 'scope> ImmutableTrie<'a> {
    fn best_first_search<'f, State: StateT, F>
    (&'f self,
     starting_state: State,
     params: &SearchParams<State>,
     config: &'f SearchConfig,
     result_callback: Arc<Mutex<F>>,
    )
        where F: ResultCallback
    {
        let search_state = SearchState {
            num_spaces: 0,
            current_word_len: 0,
            prev_words: vec![],
            total_len: 0,
            prev_penalty: 0,
        };
        let root = self.root.get().unwrap();
        let item = QItem(root.order(|_| 0),
                         search_state, starting_state);

        scope(|scope| {
            let pq = ConcurrentPQ::<QItem<State>>::new();
            let done = Arc::new(AtomicBool::new(false));

            pq.push(item);
            scope.spawn(move |s| {
                Self::worker(
                    pq, config, params, done, root, s, result_callback);
            })
        });
    }


    fn worker<'f, State: StateT, F>(mut pq: ConcurrentPQ<QItem<'f, State>>,
                                    config: &'f SearchConfig,
                                    params: &'f SearchParams<State>,
                                    done: Arc<AtomicBool>,
                                    root: &'f ImmutableTrieNode,
                                    scope: &Scope<'scope>,
                                    result_callback: Arc<Mutex<F>>,
    )
        where State: StateT + 'f, F: ResultCallback + 'scope,
              'f: 'scope
    {
        if config.max_results.is_some() && done.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let item;
        match pq.try_pop() {
            Some(x) => item = x,
            None => return
        }
        let QItem(node, search_state, state, ..) = item;

        if node.is_terminal && node.freq > config.prune_freq {
            if (params.accept)(&state) && search_state.current_word_len >= config.min_word_len {
                let mut new_search_state = search_state.clone();
                new_search_state.prev_words.push(node.node);
                let result = new_search_state.prev_words.iter()
                    .map(|x| x.path.clone()).collect::<Vec<_>>().join(" ");

                let mut callback = result_callback.lock().unwrap();
                if callback(result, config) {
                    done.store(true, std::sync::atomic::Ordering::Relaxed);
                    pq.clear();
                    return;
                }
            }
        }

        for child in node.node {
            if (config.prune_freq > child.weight) {
                continue
            }
            if let Some(new_state) = (params.keep_going)(&state, child.letter) {
                let pq = pq.clone();
                let done = done.clone();
                let new_search_state = search_state.same_word();
                let item = QItem(
                    child.order(|x|
                        (params.score)(x, &search_state) - search_state.prev_penalty),
                    new_search_state,
                    new_state);
                let result_callback = result_callback.clone();

                pq.push(item);
                scope.spawn(move |s| {
                    Self::worker(pq, config, params, done, root, s, result_callback);
                })
            }
        }
        if search_state.num_spaces < config.spaces_allowed && search_state.current_word_len >= config.min_word_len {
            if let Some(penalty) = config.space_penalty {
                scope.spawn(move |s| {
                    let mut ordered =
                        root.order(|x| (params.score)(x, &search_state) - search_state.prev_penalty);
                    ordered.val -= penalty as isize;
                    let item = QItem(ordered,
                                     search_state.new_word(node.node, config),
                                     state);

                    pq.push(item);
                    Self::worker(pq, config, params, done.clone(), root, s, result_callback.clone());
                })
            }
        }
    }


    pub fn query_regex_multithreaded<F>(&self, regex: &str, config: &SearchConfig, result_callback: F)
        where F: ResultCallback {
        let nfa = NfaGraph::from_regex(regex);

        let params = SearchParams::<(&NfaGraph, Vec<NfaStatePtr>)> {
            keep_going: |state, c: char| {
                let lstring = c.to_string();
                let result = state.0.apply_with_start(&lstring, &state.1);
                if result.states.is_empty() {
                    None
                } else {
                    Some((state.0, result.states))
                }
            },
            score: |x, search_state| (search_state.total_len as isize)  * (x.weight as isize),
            accept: |state| state.1.iter().any(|x| x.kind_is(&Accept)),
        };

        self.best_first_search((&nfa, nfa.starting_states()),
                               &params,
                               config,
                               Arc::new(Mutex::new(result_callback)));
    }

    fn get_counts(word: &str) -> HashMap<char, usize> {
        let mut counts = hashmap! {};

        word.chars().for_each(|c| {
            *counts.entry(c).or_insert(0) += 1;
        });
        counts
    }

    pub fn query_anagram_multithreaded<F>(&self, word: &str, config: &SearchConfig, result_callback: F)
        where F: ResultCallback {
        let params = SearchParams {
            keep_going: |counts: &HashMap<char, usize>, c: char| {
                if *counts.get(&c).unwrap_or(&0) > 0 {
                    let mut new_counts = counts.clone();
                    *new_counts.get_mut(&c).unwrap() -= 1;
                    Some(new_counts)
                } else { None }
            },
            score: |x, search_state|  (search_state.total_len as isize)  * (x.weight as isize),
            accept: |counts: &HashMap<char, usize>| counts.values().all(|x| *x == 0),
        };
        self.best_first_search(
            Self::get_counts(word),
            &params,
            config,
            Arc::new(Mutex::new(result_callback)),
        );
    }

    pub fn query_anagram_results(&self, word: &str, config: &SearchConfig) -> Vec<String> {
        let results = Mutex::new(vec![]);
        let callback = |result, config: &SearchConfig| {
            let mut r = results.lock().unwrap();
            if r.len() >= config.max_results.unwrap_or(usize::MAX) {
                return true;
            }
            r.push(result);
            false
        };
        self.query_anagram_multithreaded(word, config, callback);
        let x = results.lock().unwrap().clone();
        x
    }

    pub fn query_regex_results(&self, word: &str, config: &SearchConfig) -> Vec<String> {
        let results = Mutex::new(vec![]);
        let callback = |result, config: &SearchConfig| {
            let mut r = results.lock().unwrap();
            if r.len() >= config.max_results.unwrap_or(usize::MAX) {
                return true;
            }
            r.push(result);
            false
        };
        self.query_regex_multithreaded(word, config, callback);
        let x = results.lock().unwrap().clone();
        x
    }
}

impl StateT for HashMap<char, usize> {}

impl StateT for (&NfaGraph, Vec<NfaStatePtr>) {}

#[derive(PartialEq, Eq, Debug)]
struct OrderedTrieNode<'a> {
    val: isize,
    node: &'a ImmutableTrieNode<'a>,
}

impl PartialOrd<Self> for OrderedTrieNode<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedTrieNode<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.val.cmp(&other.val)
    }
}

impl ImmutableTrieNode<'_> {
    fn order<'a, F>(&'a self, f: F) -> OrderedTrieNode<'a>
        where F: Fn(&'a ImmutableTrieNode) -> isize {
        OrderedTrieNode { val: f(self), node: self }
    }
}

impl<'a> From<&'a ImmutableTrieNode<'a>> for OrderedTrieNode<'a> {
    fn from(node: &'a ImmutableTrieNode) ->
    Self {
        OrderedTrieNode::<'a> {
            val: 0,
            node,
        }
    }
}

impl<'a> Deref for OrderedTrieNode<'a> {
    type Target = ImmutableTrieNode<'a>;

    fn deref(&self) -> &ImmutableTrieNode<'a> {
        self.node
    }
}

#[test]
fn test_anagram_multithreaded() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
    let mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let trie = ImmutableTrie::new();
    mut_trie.build(&trie);

    let default_config = SearchConfig::new();

    assert_eq!(
        trie.query_anagram_results("OLEHL", &default_config),
        vec!["HELLO"]);
    assert_eq!(
        trie.query_anagram_results("OOGD", &default_config),
        vec!["GOOD"]);

    assert_eq!(
        trie.query_anagram_results("LEHL", &default_config),
        Vec::<String>::new());
    assert_eq!(
        trie.query_anagram_results("LELO", &default_config),
        Vec::<String>::new());
    assert_eq!(
        trie.query_anagram_results("DOG", &default_config),
        Vec::<String>::new());
}

#[test]
fn query_words_in_trie() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
    let mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let immut = ImmutableTrie::new();
    mut_trie.build(&immut);
    let default_config = SearchConfig::new();

    let mut result =
        immut.query_regex_results("H.L*(O|P)", &default_config);
    result.sort();

    assert_eq!(result, vec!["HELLO", "HELP"])
}


#[test]
fn query_words_in_trie_space_penalty() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD", "BYE"];
    let mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let immut = ImmutableTrie::new();
    mut_trie.build(&immut);
    let mut default_config = SearchConfig::new();
    default_config.space_penalty = Some(50);
    default_config.spaces_allowed = 3;

    let mut result =
        immut.query_regex_results("GOODBYE", &default_config);
    result.sort();

    assert_eq!(result, vec!["GOOD BYE", "GOODBYE"])
}
