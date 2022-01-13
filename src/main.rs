pub mod wordlist;
mod ciphers;
mod alphabet;
mod regex;


use std::time::Instant;
use structopt::StructOpt;
use crate::wordlist::trie::searchconfig::SearchConfig;

use crate::wordlist::wordlist::{FileFormat, Wordlist};


/// Search for a pattern in a file and display the lines that contain it.
#[derive(StructOpt)]
struct Cli {
    /// The path to the file to read
    #[structopt(parse(from_os_str))]
    path: Option<std::path::PathBuf>,

    #[structopt(long)]
    web: bool,

    #[structopt(long)]
    anagram: Option<String>,

    #[structopt(long)]
    regex: Option<String>,
}


fn run_web() {}

fn main() {
    let args = Cli::from_args();

    let wl = Wordlist::new();
    if args.path.is_some() {
        wl.load_file(args.path.unwrap().as_path().to_str().unwrap(),
                     FileFormat::builder().build());
    } else {
        //wl.load_file("/usr/share/dict/words",
        //             FileFormat::builder().build());
        wl.load_file("data/with_freqs",
                     FileFormat::builder().delimiter(' ').word_column(1).freq_column(0).build())
    }
    let mut default_config = SearchConfig::new();
    default_config.space_penalty = Some(6187267);
    default_config.spaces_allowed = 2;
    default_config.max_length = Some(100);

    if args.anagram.is_some() {
        //let results = wl.anagram(&args.anagram.unwrap());
        let results = wl.anagram_multithreaded(&args.anagram.unwrap(), &default_config);
        for r in results {
            println!("{}", r);
        }
    }


    if let Some(regex) = args.regex {
        //let results = wl.anagram(&args.anagram.unwrap());
        println!("Searching for {}", regex);
        let results = wl.search_multithreaded(&regex, &default_config);
        for r in results {
            println!("{}", r);
        }
    }


    // {
    //     let start = Instant::now();
    //     let l = wl.search(&args.pattern).len();
    //     println!("{} in {:#?}s", l, start.elapsed().as_secs_f64());
    // }
    // {
    //     let start = Instant::now();
    //     let l = wl.search_multithreaded(&args.pattern).len();
    //     println!("{} in {:#?}s", l, start.elapsed().as_secs_f64());
    // }
    //println!("${:#?}", wl.search(&args.pattern));
}
