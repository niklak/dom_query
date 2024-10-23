#![allow(dead_code)]
use dom_query::Document;

pub fn doc() -> Document {
    include_str!("../test-pages/page.html").into()
}

pub fn doc_wiki() -> Document {
    include_str!("../test-pages/rustwiki.html").into()
}

pub fn doc_with_siblings() -> Document {
    include_str!("../test-pages/tests_with_siblings.html").into()
}
