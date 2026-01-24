use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

use dom_query::Document;

fn bench_dom_query_merging(c: &mut Criterion) {
    let mut group = c.benchmark_group("dom_query");
    let contents = include_str!("../test-pages/hacker_news.html");
    let doc = Document::from(contents);

    // simple selection
    group.bench_with_input(BenchmarkId::new("merging", "simple"), &doc, |b, doc| {
        b.iter(|| {
            let d = doc.clone();
            let sel = d.select("body td.title a[href]");
            sel.prepend_html(r#"<span class="indent"></span><i class="awesome-icon"></i>"#);
            black_box(sel);
            black_box(d);
        })
    });

    group.finish();
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(5000)
        .measurement_time(std::time::Duration::new(10, 0))
        .warm_up_time(std::time::Duration::new(5, 0))
}

criterion_group! {name = benches; config = configure_criterion();
    targets = bench_dom_query_merging,
}
criterion_main!(benches);
