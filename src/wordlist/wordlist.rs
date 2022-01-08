use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

use serde_json::from_str;
use crate::normalize;
use crate::wordlist::trie::Trie;
use crate::wordlist::index::Index;
use typed_builder::TypedBuilder;

pub struct Wordlist {
    trie: Trie,
}

#[derive(TypedBuilder)]
pub struct FileFormat {
    #[builder(default, setter(strip_option))]
    delimiter: Option<char>,
    #[builder(default, setter(strip_option))]
    word_column: Option<usize>,
    #[builder(default, setter(strip_option))]
    freq_column: Option<usize>,
}

impl FileFormat {
    fn parse_line<'a>(&self, line: &'a str) -> (&'a str, isize) {
        if self.delimiter.is_none() {
            (line, 1)
        } else {
            let columns = line.split(self.delimiter.unwrap()).collect::<Vec<_>>();
            let word_idx = self.word_column.unwrap_or(0);
            let freq_idx = self.freq_column.unwrap_or(1);
            (columns.get(word_idx).unwrap(),
             from_str::<isize>(columns.get(freq_idx).unwrap()).unwrap())
        }
    }
}

impl Wordlist {
    pub fn from_file(filename: &str, format: FileFormat) -> Wordlist {
        println!("Reading words from {:#?}", &filename);

        let file = File::open(filename).unwrap();
        let buf_reader = BufReader::new(file);

        let mut trie = Trie::builder();
        let mut count: usize = 0;
        let mut failures: usize = 0;

        let mut start = Instant::now();

        let lines: Vec<Result<String, _>> = buf_reader.lines().collect();

        println!("Reading took {}", start.elapsed().as_secs_f64());

        start = Instant::now();
        //buf_reader.lines()
        lines.iter().for_each(|x| match x {
            Ok(line) => {
                if line.len() > 0 {
                    let (word, freq) = format.parse_line(&line);
                    trie.add_with_freq(&*normalize(&word), freq.try_into().unwrap());
                    count += 1;
                    if count % 100000 == 0 {
                        println!("{} {}", count, normalize(&word));
                    }
                }
            }
            Err(_e) => {
                //eprintln!("Line #{} - {}", count, e);
                failures += 1;
            }
        });
        let elapsed = start.elapsed();
        println!("Read {} words in {}s ({} kwps) [{} failures ({:.2}%)]",
                 count, (elapsed.as_millis() as f64) / 1000.0, (count as f64) / (elapsed.as_millis() as f64),
                 failures, 100.0 * (failures as f64) / (count as f64));

        let start_build = Instant::now();
        let built_trie = trie.build();
        println!("Built tree in {}", start_build.elapsed().as_millis() as f64 / 1000.0);
        Wordlist { trie: built_trie }
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