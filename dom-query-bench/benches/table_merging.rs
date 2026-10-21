use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::fmt::Write;
use std::hint::black_box;

use dom_query::Document;

fn table_contents(rows: usize, cols: usize) -> String {
    let mut html = String::from("<table><tbody>");
    for row in 0..rows {
        html.push_str("<tr>");
        for col in 0..cols {
            let _ = write!(html, "<td>{row}:{col}</td>");
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table>");
    html
}

fn bench_table_replace(c: &mut Criterion) {
    let mut group = c.benchmark_group("dom_query");
    let contents = table_contents(200, 10);
    let doc = Document::from(contents);

    group.bench_with_input(
        BenchmarkId::new("replace_with_html", "table_cells"),
        &doc,
        |b, doc| {
            b.iter(|| {
                let d = doc.clone();
                let cells = d.select("td");
                cells.replace_with_html(r#"<td class="updated">updated</td>"#);
                black_box(cells);
                black_box(d);
            })
        },
    );

    group.finish();
}

fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::new(5, 0))
        .warm_up_time(std::time::Duration::new(2, 0))
}

criterion_group! {name = benches; config = configure_criterion();
    targets = bench_table_replace,
}
criterion_main!(benches);
