use std::cmp::max;
use std::fs::File;
use std::io::{BufRead, BufReader};

use std::time::Instant;

use serde_json::from_str;

use typed_builder::TypedBuilder;
use crate::alphabet::normalize;
use crate::wordlist::trie::multithreaded_search::ResultCallback;
use crate::wordlist::trie::searchconfig::SearchConfig;
use crate::wordlist::trie::trie::{ImmutableTrie, Trie};

pub struct Wordlist<'a> {
    trie: Trie<'a>,
    immut_trie: ImmutableTrie<'a>,
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
    fn parse_line<'a>(&self, line: &'a str) -> Result<(&'a str, isize), String> {
        if self.delimiter.is_none() {
            Ok((line, 1))
        } else {
            let columns = line.split(self.delimiter.unwrap()).collect::<Vec<_>>();
            let word_idx = self.word_column.unwrap_or(0);
            let freq_idx = self.freq_column.unwrap_or(1);
            if (columns.len() >= max(word_idx, freq_idx)) {
                Ok((columns.get(word_idx).unwrap(),
                    from_str::<isize>(columns.get(freq_idx).unwrap()).unwrap()))
            } else {
                Err("Not enough columns".to_string())
            }
        }
    }
}

impl<'a> Wordlist<'a> {
    pub fn new() -> Wordlist<'a> {
        Wordlist { trie: Trie::new(), immut_trie: ImmutableTrie::new() }
    }

    pub fn load_file<'f>(&'a self, filename: &str, format: FileFormat) {
        println!("Reading words from {:#?}", &filename);

        let file = File::open(filename).unwrap();
        let buf_reader = BufReader::new(file);

        let trie = &self.trie;
        let immut = &self.immut_trie;
        let mut count: usize = 0;
        let mut failures: usize = 0;

        let mut start = Instant::now();

        let lines: Vec<Result<String, _>> = buf_reader.lines().collect();

        println!("Reading took {}", start.elapsed().as_secs_f64());

        start = Instant::now();
        //buf_reader.lines()
        lines.iter().for_each(
            |x| match x {
                Ok(line) => {
                    if line.len() > 0 {
                        match format.parse_line(line) {
                            Ok((word, freq)) => {
                                trie.add_with_freq(&*normalize(&word), freq.try_into().unwrap());
                                count += 1;
                                if count % 100000 == 0 {
                                    println!("{} {}", count, normalize(&word));
                                }
                            }
                            Err(e) => {
                                failures += 1;
                            }
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
        {
            trie.build(immut);
        }
        println!("Built tree in {}", start_build.elapsed().as_millis() as f64 / 1000.0);
    }

    pub fn contains(&'a self, word: &str) -> bool {
        self.trie.contains(word)
    }
    pub fn search(&'a self, regex: &str) -> Vec<String> {
        self.trie.query_regex(regex)
    }

    pub fn search_multithreaded(&'a self, regex: &str, config: &SearchConfig) -> Vec<String> {
        self.immut_trie.query_regex_results(regex, config)
    }
    pub fn search_callback<F>(&'a self, regex: &str, config: &SearchConfig, callback: F)
        where F: ResultCallback {
        self.immut_trie.query_regex_multithreaded(regex, config, callback)
    }

    pub fn anagram(&'a self, anagram: &str) -> Vec<String> {
        self.trie.query_anagram(anagram)
    }


    pub fn anagram_multithreaded(&'a self, regex: &str, config: &SearchConfig) -> Vec<String>{
        self.immut_trie.query_anagram_results(regex, config)
    }
    pub fn anagram_callback<F>(&'a self, anagram: &str, config: &SearchConfig, callback: F)
        where F: ResultCallback {
        self.immut_trie.query_anagram_multithreaded(anagram, config, callback)
    }
}