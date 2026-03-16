use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

use dom_query::Document;

fn bench_add_class(c: &mut Criterion) {
    let contents = include_str!("../test-pages/hacker_news.html");
    let doc = Document::from(contents);

    let mut group = c.benchmark_group("dom_query");
    
    let test_cases = vec![
        ("small", "p-2 m-2 p-2 text-center"),
        ("medium", "p-4 m-4 text-left text-left bg-gray-100 hover:bg-gray-200 
            rounded rounded shadow"),
        ("large", "p-2 m-2 text-center bg-white hover:bg-gray-100 rounded 
            shadow border border-gray-300 font-bold text-sm text-sm 
            text-gray-700 focus:outline-none focus:ring-2 focus:ring-indigo-500"),
        ("insane", "p-2 m-2 text-center bg-white hover:bg-gray-100 rounded 
                shadow border border-gray-300 font-bold text-sm text-gray-700 
                focus:outline-none focus:ring-2 focus:ring-indigo-500 
                hover:text-indigo-600 hover:shadow-md hover:bg-gray-200 px-4 
                py-2 mb-2 mt-2 max-w-full w-full sm:w-auto md:w-1/2 lg:w-1/3 
                xl:w-1/4 flex flex-col justify-center items-center"),
        ("beyond", "p-2 m-2 text-center bg-white hover:bg-gray-100 rounded 
            shadow border border-gray-300 font-bold text-sm text-gray-700 
            focus:outline-none focus:ring-2 focus:ring-indigo-500 
            hover:text-indigo-600 hover:shadow-md hover:bg-gray-200 px-4 py-2 
            mb-2 mt-2 max-w-full w-full sm:w-auto md:w-1/2 lg:w-1/3 xl:w-1/4 
            flex flex-col justify-center items-center p-2 m-2 
            text-center bg-white hover:bg-gray-100 rounded shadow border 
            border-gray-300 font-bold text-sm text-gray-700 focus:outline-none 
            focus:ring-2 focus:ring-indigo-500")
    ];
    
    for (name, classes) in test_cases {
        group.bench_with_input(BenchmarkId::new("add_class", name), &doc, |b, doc| {
            b.iter(|| {
                let d = doc.clone();
                let sel = d.select("a");
                sel.remove_attr("class");
                sel.add_class(classes);
                black_box(sel);
            })
        });
    }

    
    group.finish();
}
criterion_group!(benches, bench_add_class);
criterion_main!(benches);
