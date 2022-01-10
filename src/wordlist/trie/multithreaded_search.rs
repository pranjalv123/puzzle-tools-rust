use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Debug;
use std::ops::{Deref};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use maplit::hashmap;

use rayon::{scope, Scope};
use crate::regex::nfa::graph::NfaGraph;
use crate::regex::nfa::state::NfaStateKind::Accept;

use crate::wordlist::trie::node::{ImmutableTrieNode};
use crate::wordlist::trie::searchconfig::SearchConfig;
use crate::wordlist::trie::trie::{ImmutableTrie, Trie};


//#[derive(Debug)]
struct QItem<'q, State: Send + Debug + 'q>(OrderedTrieNode<'q>, SearchState<'q, State>, State);

impl<State: Send + Debug> PartialEq<Self> for QItem<'_, State> {
    fn eq(&self, other: &Self) -> bool {
        self.0.val == other.0.val
    }
}

impl<State: Send + Debug> Eq for QItem<'_, State> {}

impl<State: Send + Debug> PartialOrd<Self> for QItem<'_, State> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<State: Send + Debug> Ord for QItem<'_, State> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

type PriorityQueue<'q, State: Send + Debug> = BinaryHeap<QItem<'q, State>>;

struct SearchState<'a, State>
    where State: Send + Debug + 'a {
    accept: for<'r> fn(&'r State) -> bool,
    keep_going: for<'r> fn(&'r State, char) -> Option<State>,
    score: for<'r> fn(&'r ImmutableTrieNode) -> isize,

    num_spaces: usize,
    current_word_len: usize,
    prev_words: Vec<&'a ImmutableTrieNode<'a>>,
    prev_penalty: isize,
}

impl<'a, State> Clone for SearchState<'a, State>
    where State: Send + Debug + 'a {
    fn clone(&self) -> Self {
        SearchState {
            accept: self.accept,
            keep_going: self.keep_going,
            score: self.score,
            num_spaces: self.num_spaces,
            current_word_len: self.current_word_len,
            prev_words: self.prev_words.clone(),
            prev_penalty: self.prev_penalty,
        }
    }
}

impl<'a, State> SearchState<'a, State>
    where State: Send + Debug {
    fn new_word(&self, node: &'a ImmutableTrieNode<'a>, config: &SearchConfig) -> SearchState<'a, State> {
        let mut new = self.clone();
        new.num_spaces += 1;
        new.current_word_len = 0;
        new.prev_words.push(node);
        new.prev_penalty += config.space_penalty.unwrap() as isize - node.freq as isize;
        new
    }
    fn same_word(&self) -> SearchState<'a, State> {
        let mut new = self.clone();
        new.current_word_len += 1;
        new
    }
}

impl<'a, 'scope> ImmutableTrie<'a> {
    fn best_first_search<'f, State>
    (&'f self,
     accept: fn(&State) -> bool,
     keep_going: fn(&State, char) -> Option<State>,
     score: for<'r> fn(&'r ImmutableTrieNode) -> isize,
     starting_state: State,
     config: &'f SearchConfig,
    ) -> Vec<String>
        where State: Send + Debug + 'f
    {
        let pq: Arc<Mutex<PriorityQueue<'f, _>>> = Arc::new(Mutex::new(PriorityQueue::new()));
        let results: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));

        let search_state = SearchState {
            num_spaces: 0,
            current_word_len: 0,
            prev_words: vec![],
            prev_penalty: 0,
            accept,
            score,
            keep_going,
        };
        let item = QItem(self.root.get().unwrap().order(score), search_state, starting_state);
        let root = self.root.get().unwrap();

        scope(|scope| {
            let r = results.clone();
            let pq = pq.clone();
            let done = Arc::new(AtomicBool::new(false));

            let mut pql = pq.lock().unwrap();
            pql.push(item);
            drop(pql);
            scope.spawn(move |s| {
                Self::worker(
                    pq, r, config, done, root, s);
            })
        });

        let x = results.deref().lock().unwrap().clone();
        x
    }


    fn worker<'f, State>(pq: Arc<Mutex<PriorityQueue<'f, State>>>,
                         results: Arc<Mutex<Vec<String>>>,
                         config: &'f SearchConfig,
                         done: Arc<AtomicBool>,
                         root: &'f ImmutableTrieNode,
                         scope: &Scope<'scope>,
    )
        where State: Send + Debug + 'f,
              'f: 'scope
    {
        if config.max_results.is_some() && done.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let mut pq_l = pq.lock().unwrap();
        let QItem(node, search_state, state) = pq_l.pop().unwrap();
        drop(pq_l);
        if node.is_terminal {
            if (search_state.accept)(&state) && search_state.current_word_len >= config.min_word_len {
                let mut r = results.lock().unwrap();
                if config.max_length.is_some() && r.len() >= config.max_length.unwrap() {
                    pq.lock().unwrap().clear();
                    done.store(false, std::sync::atomic::Ordering::Relaxed);
                    return;
                }
                let mut new_search_state = search_state.clone();
                new_search_state.prev_words.push(node.node);
                r.push(new_search_state.prev_words.iter()
                    .map(|x| x.path.clone()).collect::<Vec<_>>().join(" "));
                println!("{:?}, {:?}", r.last(), node.val);
                drop(r);
            }
        }

        for child in node.node {
            if let Some(new_state) = (search_state.keep_going)(&state, child.letter) {
                let results = results.clone();
                let pq = pq.clone();

                let done = done.clone();
                let new_search_state = search_state.same_word();
                let item = QItem(
                    child.order(|x| (search_state.score)(x) - search_state.prev_penalty),
                    new_search_state,
                    new_state);
                let mut pql = pq.lock().unwrap();
                pql.push(item);
                drop(pql);
                scope.spawn(move |s| {
                    Self::worker(pq, results, config, done, root,
                                 s);
                })
            }
        }
        if search_state.num_spaces < config.spaces_allowed && search_state.current_word_len >= config.min_word_len {
            if let Some(penalty) = config.space_penalty {
                scope.spawn(move |s| {
                    let mut ordered = root.order(|x| (search_state.score)(x) - search_state.prev_penalty);
                    ordered.val -= penalty as isize;
                    let item = QItem(ordered,
                                     search_state.new_word(node.node, config),
                                     state);
                    let mut pql = pq.lock().unwrap();
                    pql.push(item);
                    drop(pql);
                    Self::worker(pq, results, config, done.clone(), root, s);
                })
            }
        }
    }


    pub fn query_regex_multithreaded(&self, regex: &str, config: &SearchConfig) -> Vec<String> {
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
                               |x| x.weight as isize,
                               (&nfa, nfa.starting_states()),
                               config)
    }

    fn get_counts(word: &str) -> HashMap<char, usize> {
        let mut counts = hashmap! {};

        word.chars().for_each(|c| {
            *counts.entry(c).or_insert(0) += 1;
        });
        counts
    }

    pub fn query_anagram_multithreaded(&self, word: &str, config: &SearchConfig) -> Vec<String> {
        self.best_first_search(|counts: &HashMap<char, usize>| counts.values().all(|x| *x == 0),
                               |counts: &HashMap<char, usize>, c: char| {
                                   if *counts.get(&c).unwrap_or(&0) > 0 {
                                       let mut new_counts = counts.clone();
                                       *new_counts.get_mut(&c).unwrap() -= 1;
                                       Some(new_counts)
                                   } else { None }
                               },
                               |x| x.weight as isize,
                               Self::get_counts(word),
                               config,
        )
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct OrderedTrieNode<'a> {
    val: isize,
    node: &'a ImmutableTrieNode<'a>,
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
    let mut mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let mut trie = ImmutableTrie::new();
    mut_trie.build(&trie);

    let default_config = SearchConfig::new();

    assert_eq!(trie.query_anagram_multithreaded("OLEHL", &default_config), vec!["HELLO"]);
    assert!(trie.query_anagram_multithreaded("LEHL", &default_config).is_empty());
    assert!(trie.query_anagram_multithreaded("LELO", &default_config).is_empty());
    assert!(trie.query_anagram_multithreaded("DOG", &default_config).is_empty());
    assert_eq!(trie.query_anagram_multithreaded("OOGD", &default_config), vec!["GOOD"]);
}

#[test]
fn query_words_in_trie() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
    let mut mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let immut = ImmutableTrie::new();
    mut_trie.build(&immut);
    let default_config = SearchConfig::new();
    let mut result = immut.query_regex_multithreaded("H.L*(O|P)", &default_config);
    result.sort();

    assert_eq!(result, vec!["HELLO", "HELP"])
}


#[test]
fn query_words_in_trie_space_penalty() {
    let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD", "BYE"];
    let mut mut_trie = Trie::new();
    mut_trie.add_all((&words).iter().map(|x| *x));
    let immut = ImmutableTrie::new();
    mut_trie.build(&immut);
    let mut default_config = SearchConfig::new();
    default_config.space_penalty = Some(50);
    default_config.spaces_allowed = 3;
    {
        let mut result = immut.query_regex_multithreaded("H.L*(O|P)", &default_config);
        result.sort();

        assert_eq!(result, vec!["HELLO", "HELP"])
    }
    {
        let mut result = immut.query_regex_multithreaded("GOODBYE", &default_config);
        result.sort();

        assert_eq!(result, vec!["GOOD BYE", "GOODBYE"])
    }
}
