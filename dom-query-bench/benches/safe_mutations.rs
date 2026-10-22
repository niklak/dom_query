use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::fmt::Write;
use std::hint::black_box;
use std::time::Duration;

use dom_query::Document;

const MAX_DEPTH: usize = 10_000;
const TARGET_DEPTHS: [usize; 4] = [10, 100, 1_000, 10_000];
const RUN: usize = 1_000;

fn deep_run_doc() -> String {
    let mut html = String::with_capacity(MAX_DEPTH * 11 + RUN * 13 + 64);
    html.push_str(r#"<body><div id="run">"#);
    for _ in 0..RUN {
        html.push_str("<span></span>");
    }
    html.push_str("</div>");
    for depth in 1..=MAX_DEPTH {
        if TARGET_DEPTHS.contains(&depth) {
            let _ = write!(html, r#"<div id="target-{depth}">"#);
        } else {
            html.push_str("<div>");
        }
    }
    for _ in 0..MAX_DEPTH {
        html.push_str("</div>");
    }
    html.push_str("</body>");
    html
}

fn bench_append_children(c: &mut Criterion) {
    let contents = deep_run_doc();
    let doc = Document::from(contents.as_str());
    assert_eq!(doc.tree.len(), 11_005);
    assert_eq!(doc.select("#run > span").length(), RUN);

    let run_start_id = doc.select("#run > span").nodes()[0].id;
    let targets = TARGET_DEPTHS.map(|depth| {
        let target_id = doc.select_single(&format!("#target-{depth}")).nodes()[0].id;
        assert!(doc.tree.ancestor_ids_of_it(&target_id, None).count() >= depth);
        (depth, target_id)
    });

    let mut group = c.benchmark_group("dom_query");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));
    let doc_ref = &doc;
    for (depth, target_id) in targets {
        group.bench_function(BenchmarkId::new("append_children/depth", depth), move |b| {
            b.iter_batched_ref(
                || doc_ref.clone(),
                |doc| {
                    doc.tree
                        .get_unchecked(&target_id)
                        .append_children(&run_start_id);
                    black_box(doc);
                },
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_append_children);
criterion_main!(benches);
