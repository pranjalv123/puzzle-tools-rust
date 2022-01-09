use std::path::PathBuf;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use puzzle_tools::wordlist::wordlist::{FileFormat, Wordlist};


fn criterion_benchmark(c: &mut Criterion) {

    let mut data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_dir.push("data/allwords2.txt");
    let wl = Wordlist::new();
    wl.load_file(data_dir.to_str().unwrap(),
                 FileFormat::builder().build());

    c.bench_function("len 3", |b| b.iter(|| wl.search("...")));

    { let mut group = c.benchmark_group("10s");
        group.sample_size(10);
        group.bench_function("len 5", |b| b.iter(|| wl.search(".....")));
     //   group.bench_function("len 7", |b| b.iter(|| wl.search(".......")));
    }

    { let mut group = c.benchmark_group("10s");
        group.sample_size(10);
        group.bench_function("len 5", |b| b.iter(|| wl.search(".....")));
     //   group.bench_function("len 7", |b| b.iter(|| wl.search(".......")));
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);