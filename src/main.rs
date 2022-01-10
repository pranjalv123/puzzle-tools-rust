pub mod wordlist;
mod ciphers;
mod alphabet;
mod regex;



use std::time::Instant;
use structopt::StructOpt;

use crate::wordlist::wordlist::{FileFormat, Wordlist};


/// Search for a pattern in a file and display the lines that contain it.
#[derive(StructOpt)]
struct Cli {
    /// The path to the file to read
    #[structopt(parse(from_os_str))]
    path: std::path::PathBuf,
    pattern: String,
}

fn main() {
    let args = Cli::from_args();

    let wl = Wordlist::new();
    wl.load_file(args.path.as_path().to_str().unwrap(),
                                 FileFormat::builder().build());

    {
        let start = Instant::now();
        let l = wl.search(&args.pattern).len();
        println!("{} in {:#?}s", l, start.elapsed().as_secs_f64());
    }
    {
        let start = Instant::now();
        let l = wl.search_multithreaded(&args.pattern).len();
        println!("{} in {:#?}s", l, start.elapsed().as_secs_f64());
    }
    //println!("${:#?}", wl.search(&args.pattern));
}
