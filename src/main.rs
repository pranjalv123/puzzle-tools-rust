pub mod wordlist;
mod ciphers;
mod alphabet;
mod regex;


use std::io::{self, Write};
use std::io::stdin;
use serde_json::from_str;
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
    default_config.space_penalty = Some(5000);//Some(6187267);
    default_config.spaces_allowed = 2;
    default_config.max_results = Some(50);
    default_config.prune_freq = 100;

    loop {
        let mut command = String::new();
        print!("> ");
        io::stdout().flush().unwrap();
        stdin().read_line(&mut command);

        if command == "exit" { return; }
        let mut split = command.splitn(2, " ");
        let command = split.next();
        let arg = split.next();
        if command.is_none() || arg.is_none() {
            println!("anagram SOMETHING, regex SOMETHING use all caps\n\
            or set [max_results,spaces_allowed,prune_freq,space_penalty] <number> (ask pranjal what these mean if unclear)");
            continue;
        }


        let arg = arg.unwrap().trim();

        if command.unwrap() == "set" {
            let mut split = arg.splitn(2, " ");
            let prop = split.next();
            let val = split.next();
            if prop.is_none() || val.is_none() {
                println!("Invalid command (should be set <prop> <val>");
                continue;
            }
            let val = from_str::<usize>(val.unwrap());
            if val.is_err() {
                println!("Couldn't parse number");
                continue;
            }
            let prop = prop.unwrap();
            let val = val.unwrap();
            match prop {
                "max_results" => default_config.max_results = Some(val),
                "spaces_allowed" => default_config.spaces_allowed = val,
                "prune_freq" => default_config.prune_freq = val,
                "space_penalty" => default_config.space_penalty = Some(val),
                _ => println!("Invalid property")
            }

        }
        if command.unwrap() == "anagram" {
            println!("Anagramming \"{}\"", arg);
            let mut counter = 0;
            wl.anagram_callback(arg,
                                &default_config, |word, config: &SearchConfig| {
                    counter += 1;
                    println!("{}", word);
                    return counter >= config.max_results.unwrap();
                });
        }

        if command.unwrap() == "regex" {
            let mut counter = 0;
            wl.search_callback(arg,
                               &default_config, |word, config: &SearchConfig| {
                    counter += 1;
                    println!("{}", word);
                    return counter >= config.max_results.unwrap();
                });
        }
    }
}
    //
    // if args.anagram.is_some() {
    //     //let results = wl.anagram(&args.anagram.unwrap());
    //     let mut counter = 0;
    //     let results = wl.anagram_callback(&args.anagram.as_ref().unwrap(),
    //                                       &default_config, |word, config: &SearchConfig| {
    //             counter += 1;
    //             println!("{}", word);
    //             return counter >= config.max_results.unwrap();
    //         });
    //     // for r in results {
    //     //     println!("{}", r);
    //     // }
    // }
    //
    //
    // if let Some(regex) = args.regex {
    //     //let results = wl.anagram(&args.anagram.unwrap());
    //     println!("Searching for {}", regex);
    //     let results = wl.search_multithreaded(&regex, &default_config);
    //     for r in results {
    //         println!("{}", r);
    //     }
    // }
    //
    //
    // // {
    // //     let start = Instant::now();
    // //     let l = wl.search(&args.pattern).len();
    // //     println!("{} in {:#?}s", l, start.elapsed().as_secs_f64());
    // // }
    // // {
    // //     let start = Instant::now();
    // //     let l = wl.search_multithreaded(&args.pattern).len();
    // //     println!("{} in {:#?}s", l, start.elapsed().as_secs_f64());
    // // }
    // //println!("${:#?}", wl.search(&args.pattern));
    //
