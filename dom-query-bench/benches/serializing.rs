use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::{hint::black_box, time::Duration};

use dom_query::Document;

fn bench_serializing(c: &mut Criterion) {
    let contents = include_str!("../test-pages/rustwiki_2024.html");
    let doc = Document::from(contents);

    let mut group = c.benchmark_group("dom_query");
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));

    group.bench_with_input(BenchmarkId::new("serializing", "html"), &doc, |b, doc| {
        b.iter(|| {
            let html = doc.html();
            black_box(html)
        })
    });
    group.finish();
}
criterion_group!(benches, bench_serializing);
criterion_main!(benches);
