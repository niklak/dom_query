use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use dom_query::Document;

fn nested_doc(depth: usize) -> Document {
    let mut html = String::with_capacity(depth * 11 + 64);
    html.push_str("<body>");
    for _ in 0..depth {
        html.push_str("<div>");
    }
    html.push_str(r#"<span id="leaf"></span>"#);
    for _ in 0..depth {
        html.push_str("</div>");
    }
    html.push_str("</body>");
    Document::from(html.as_str())
}

fn bench_ancestor_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("dom_query");

    for (name, depth) in [("small", 100), ("medium", 1000), ("big", 10_000)] {
        let doc = nested_doc(depth);
        let ancestor_id = doc.body().unwrap().first_element_child().unwrap().id;
        let target_id = doc.select_single("#leaf").nodes()[0].id;

        assert!(
            doc.tree
                .ancestor_ids_of_it(&target_id, None)
                .any(|id| id == ancestor_id)
        );

        group.bench_with_input(BenchmarkId::new("ancestors/found", name), &doc, |b, doc| {
            b.iter(|| {
                let found = doc
                    .tree
                    .ancestor_ids_of_it(black_box(&target_id), None)
                    .any(|id| id == ancestor_id);
                black_box(found);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_ancestor_check);
criterion_main!(benches);
