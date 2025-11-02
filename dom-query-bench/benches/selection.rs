use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

use dom_query::{Document, Matcher};


fn bench_dom_query_select(c: &mut Criterion) {
    let mut group = c.benchmark_group("dom_query");
    let contents = include_str!("../test-pages/hacker_news.html");
    let doc = Document::from(contents);

    // simple selection
    group.bench_with_input(BenchmarkId::new("selection", "simple"), &doc, |b, doc| {
        b.iter(|| {
            let links = doc.select("body td.title a[href]");
            let res = links.nodes().len();
            black_box(res)
        })
    });

    // selecting with matcher
    group.bench_with_input(BenchmarkId::new("selection", "with_matcher"), &doc, |b, doc| {
        let matcher = Matcher::new(r"body td.title a[href]").unwrap();
        b.iter(|| {
            let links = doc.select_matcher(&matcher);
            let res = links.nodes().len();
            black_box(res)
        })
    });

    // selecting with matcher_iter
    group.bench_with_input(BenchmarkId::new("selection", "with_matcher_iter"), &doc, |b, doc| {
        let matcher = Matcher::new(r"body td.title a[href]").unwrap();
        b.iter(|| {
            let body = dom_query::Selection::from(doc.body().unwrap());
            let res = body.select_matcher_iter(&matcher).count();
            black_box(res)
        })
    });

    // serial selection (each selection is a root for the descendant selections)
    group.bench_with_input(BenchmarkId::new("selection", "serial"), &doc, |b, doc| {
        b.iter(|| {
            let links = doc
                .select_single("body")
                .select("td.title")
                .select("a[href]");
            let res = links.nodes().len();
            black_box(res)
        })
    });

    // finding elements with `NodeRef::find` -- supports only elements names, and accepts a slice of element names as path.
    group.bench_with_input(BenchmarkId::new("selection", "find"), &doc, |b, doc| {
        b.iter(|| {
            let root = doc.root();
            let links = root.find(&["body", "td", "a"]);
            let res = links.len();
            black_box(res)
        })
    });

    // finding elements with `NodeRef::find_descendants` -- It does descendants matching
    // and has limited css selection support. It does not support pseudo-classes and ',' combinator.
    group.bench_with_input(BenchmarkId::new("selection", "find_descendants"), &doc, |b, doc| {
        b.iter(|| {
            let root = doc.root();
            let links = root.find_descendants(r#"body td.title a[href]"#);
            let res = links.len();
            black_box(res)
        })
    });

    group.finish();
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(5000)
        .measurement_time(std::time::Duration::new(15, 0))
        .warm_up_time(std::time::Duration::new(5, 0))
}

criterion_group! {name = benches; config = configure_criterion();
    targets = bench_dom_query_select,
}
criterion_main!(benches);
