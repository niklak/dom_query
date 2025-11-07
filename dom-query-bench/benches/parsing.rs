use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{hint::black_box, time::Duration};

use dom_query::Document;

fn bench_parsing(c: &mut Criterion) {
    let contents = include_str!("../test-pages/rustwiki_2024.html");

    let mut group = c.benchmark_group("dom_query");
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));

    group.bench_with_input(
        BenchmarkId::new("parsing", "simple"),
        contents,
        |b, contents| {
            b.iter(|| {
                let document = Document::from(contents);
                black_box(document)
            })
        },
    );
    group.finish();
}
criterion_group!(benches, bench_parsing);
criterion_main!(benches);
