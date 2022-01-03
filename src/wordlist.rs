mod trie;
mod index;
mod trienode;

trait Wordlist : Iterator {
    fn addWord(&self, word: &str);
    fn contains(&self, word: &str) -> bool;
    fn search(&self, pattern: &str) -> Vec<&str>;
}

