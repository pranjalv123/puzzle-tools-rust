use std::cell::Cell;
use std::sync::Arc;
use crate::wordlist::trie::multithreaded_search::ResultCallback;

#[derive(Default)]
pub struct SearchConfig {
    pub max_results: Option<usize>,
    pub max_length: Option<usize>,
    pub space_penalty: Option<usize>,
    pub spaces_allowed: usize,
    pub min_word_len: usize,
}



impl SearchConfig {
    pub fn new() -> SearchConfig {
        SearchConfig {
            max_results: None,
            max_length: None,
            space_penalty: None,
            spaces_allowed: 0,
            min_word_len: 3,
        }
    }
}