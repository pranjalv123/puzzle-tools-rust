use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use crate::normalize;
use crate::wordlist::trie::Trie;
use crate::wordlist::index::Index;

pub struct Wordlist {
    trie: Trie
}
impl Wordlist {

    pub fn from_file(filename: &str) -> Wordlist {

        println!("Reading words from {:#?}", &filename);

        let file = File::open(filename).unwrap();
        let buf_reader = BufReader::new(file);

        let mut trie = Trie::builder();
        let mut count: usize = 0;
        let start = Instant::now();
        buf_reader.lines().for_each(|x| match x {
            Ok(word) => {
                trie.add(&*normalize(&word));
                count += 1;
                if count % 100000 == 0 {
                    println!("{} {}", count, normalize(&word));
                }
            }
            Err(e) => { panic!("{}", e); }
        });
        let elapsed = start.elapsed();
        println!("Read {} words in {}s ({} kwps)", count, (elapsed.as_millis() as f64)/1000.0, (count as f64)/(elapsed.as_millis() as f64));

        Wordlist{trie: trie.build()}

    }

    pub fn contains(&self, word: &str) -> bool {
        self.trie.contains(word)
    }
    pub fn search(&self, regex: &str) -> Vec<String> {
        self.trie.query_regex(regex)
    }

    pub fn anagram(&self, anagram: &str) -> Vec<String> {
        self.trie.query_anagram(anagram)
    }
}