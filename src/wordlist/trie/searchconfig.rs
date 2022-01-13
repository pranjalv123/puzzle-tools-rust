



#[derive(Default)]
pub struct SearchConfig {
    pub max_results: Option<usize>,
    pub max_length: Option<usize>,
    pub space_penalty: Option<usize>,
    pub spaces_allowed: usize,
    pub min_word_len: usize,
    pub prune_freq: usize,
}



impl SearchConfig {
    pub fn new() -> SearchConfig {
        SearchConfig {
            max_results: None,
            max_length: None,
            space_penalty: None,
            spaces_allowed: 0,
            min_word_len: 3,
            prune_freq: 0
        }
    }
}