use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{hint::black_box, time::Duration};

use dom_query::Document;

fn bench_markdown(c: &mut Criterion) {
    let contents = include_str!("../test-pages/rustwiki.html");

    let mut group = c.benchmark_group("dom_query");
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));
    let doc = Document::from(contents);

    group.bench_with_input(
        BenchmarkId::new("markdown", "simple"),
        &doc,
        |b, doc| {
            b.iter(|| {
                let md = doc.md(None);
                black_box(md);
            })
        },
    );
    group.finish();
}
criterion_group!(benches, bench_markdown);
criterion_main!(benches);