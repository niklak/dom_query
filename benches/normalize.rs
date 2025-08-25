use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use dom_query::Document;

fn normalize_spaces(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut iter = text.split_whitespace();

    if let Some(first) = iter.next() {
        result.push_str(first);
        for word in iter {
            result.push(' ');
            result.push_str(word);
        }
    }
    result
}

fn normalized_char_count(text: &str) -> usize {
    let mut result = 0;
    let mut iter = text.split_whitespace();

    if let Some(first) = iter.next() {
        result = first.chars().count();
        for word in iter {
            result += 1;
            result += word.chars().count();
        }
    }
    result
}

fn bench_normalized_char_count(c: &mut Criterion) {
    let html = include_str!("../test-pages/rustwiki_2024.html");
    let doc = Document::from(html);
    let body_node = doc.body().unwrap();
    c.bench_function("normalized_char_count(node.text())", |b| {
        b.iter(|| {
            let count = normalized_char_count(black_box(&body_node.text()));
            black_box(count);
        })
    });

    c.bench_function("normalize_spaces(node.text()).count", |b| {
        b.iter(|| {
            let count = normalize_spaces(black_box(&body_node.text()))
                .chars()
                .count();
            black_box(count);
        })
    });

    c.bench_function("node.normalized_char_count", |b| {
        b.iter(|| {
            let count = body_node.normalized_char_count();
            black_box(count);
        })
    });
}

criterion_group!(benches, bench_normalized_char_count);
criterion_main!(benches);
