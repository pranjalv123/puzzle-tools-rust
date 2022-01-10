#[derive(Default)]
pub struct SearchConfig{
    pub max_results: Option<usize>,
    pub max_length: Option<usize>,
    pub space_penalty: Option<usize>,
    pub spaces_allowed: usize,
    pub min_word_len: usize
}

impl SearchConfig {
    pub fn new() -> SearchConfig{
        let mut s: SearchConfig = Default::default();
        s.min_word_len = 3;
        s
    }
}