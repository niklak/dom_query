
# DOM_QUERY

> A crate for HTML querying and manipulations with CSS selectors.

[![Crates.io version](https://img.shields.io/crates/v/dom_query.svg?style=flat)](https://crates.io/crates/dom_query)
[![Download](https://img.shields.io/crates/d/dom_query.svg?style=flat)](https://crates.io/crates/dom_query)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat)](https://docs.rs/dom_query)
[![Build Status](https://github.com/niklak/dom_query/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/niklak/dom_query/actions/workflows/rust.yml)

DOM_QUERY is based on HTML crate html5ever and the CSS selector crate selectors. You can use the jQuery-like syntax to query and manipulate an HTML document quickly. **With its help you can query dom and modify it.**.

It is a fork of [nipper](https://crates.io/crates/nipper), with some updates. Also this fork supports `:has`, `:has-text`, `:contains` pseudo-classes, and some others.

## Examples


<details>
<summary><b>Parsing a document</b></summary>

```rust
use dom_query::Document;
let html = r#"<!DOCTYPE html>
<html><head><title>Test Page</title></head><body></body></html>"#;
let document = Document::from(html);
```
</details>


<details>
<summary><b>Selecting elements</b></summary>

```rust
use dom_query::Document;
let html = r#"<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>Test Page</title>
    </head>
    <body>
        <h1>Test Page</h1>
        <ul>
            <li>One</li>
            <li><a href="/2">Two</a></li>
            <li><a href="/3">Three</a></li>
        </ul>
    </body>
</html>"#;
let document = Document::from(html);
// select a single element
let a = document.select("ul li:nth-child(2)");
let text = a.text().to_string();
assert!(text == "Two");
// selecting multiple elements
document.select("ul > li:has(a)").iter().for_each(|el| {
    assert!(el.is("li"));
})
```
</details>

<details>
<summary><b>Selecting descendent elements</b></summary>

```rust
 use dom_query::Document;

 let html = r#"<!DOCTYPE html>
 <html>
     <head>
         <meta charset="utf-8">
         <title>Test Page</title>
     </head>
     <body>
         <h1>Test Page</h1>
         <ul class="list-a">
             <li>One</li>
             <li><a href="/2">Two</a></li>
             <li><a href="/3">Three</a></li>
         </ul>
         <ul class="list-b">
             <li><a href="/4">Four</a></li>
         </ul>
     </body>
 </html>"#;
 let document = Document::from(html);
 // select a parent element
 let ul = document.select("ul");

 // selecting multiple elements
 ul.select("li").iter().for_each(|el| {
     assert!(el.is("li"));
 });

 // also descendant selector may be specified starting from the parent elements
 let el = ul.select("body ul.list-b li").first();
 let text = el.text();
 assert_eq!("Four", text.to_string());

```
</details>

<details>
<summary><b>Selecting with precompiled matchers (for reuse)</b></summary>

```rust
 use dom_query::{Document, Matcher};


 let html1 = r#"<!DOCTYPE html><html><head><title>Test Page 1</title></head><body></body></html>"#;
 let html2 = r#"<!DOCTYPE html><html><head><title>Test Page 2</title></head><body></body></html>"#;
 let doc1 = Document::from(html1);
 let doc2 = Document::from(html2);

 // create a matcher once, reuse on different documents
 let title_matcher = Matcher::new("title").unwrap();

 let title_el1 = doc1.select_matcher(&title_matcher);
 assert_eq!(title_el1.text(), "Test Page 1".into());

 let title_el2 = doc2.select_matcher(&title_matcher);
 assert_eq!(title_el2.text(), "Test Page 2".into());
```
</details>

<details>
<summary><b>Manipulating the attribute of an HTML element</b></summary>

```rust
let html = r#"<!DOCTYPE html>
<html>
    <head><title>Test</title></head>
    <body><input type="hidden" name="k" data-k="100"/></body>
</html>"#;
let doc = Document::from(html);
let mut input_selection = doc.select("input[name=k]");

// get the value of attribute "data-k"
let val = input_selection.attr("data-k").unwrap();
assert_eq!(val.to_string(), "100");

// remove the attribute "data-k" from the element
input_selection.remove_attr("data-k");

// get the value of attribute "data-k", if missing, return default value
let val_or = input_selection.attr_or("data-k", "0");
assert_eq!(val_or.to_string(), "0");

// set a attribute "data-k" with value "200"
input_selection.set_attr("data-k", "200");
assert_eq!(input_selection.html(), r#"<input type="hidden" name="k" data-k="200">"#.into());
```
</details>


<details>
<summary><b>Serializing to HTML</b></summary>

```rust
 use dom_query::Document;

 let html = r#"<!DOCTYPE html>
 <html>
     <head><title>Test</title></head>
     <body><div class="content"><h1>Test Page</h1></div></body>
 </html>"#;

 let doc = Document::from(html);
 let heading_selector = doc.select("div.content");

 // serializing including the outer html tag
 let content = heading_selector.html();
 assert_eq!(content.to_string(), r#"<div class="content"><h1>Test Page</h1></div>"#);
 // serializing without the outer html tag
 let inner_content = heading_selector.inner_html();
 assert_eq!(inner_content.to_string(), "<h1>Test Page</h1>");
```
</details>



<details>
<summary><b>Accessing descendent text</b></summary>

```rust
use dom_query::Document;

let html = r#"<!DOCTYPE html>
<html>
    <head><title>Test</title></head>
    <body><div><h1>Test <span>Page</span></h1></div></body>
</html>"#;
let doc = Document::from(html);
let body_selection = doc.select("body div").first();
let text = body_selection.text();
assert_eq!(text.to_string(), "Test Page");
```
</details>


<details>
<summary><b>Extract data with pseudo-classes (:has, :has-text, :contains)</b></summary>

```rust
use dom_query::Document;

let html = include_str!("../test-pages/rustwiki_2024.html");
let doc = Document::from(html);

// searching list items inside a `tr` element which has a `a` element with title="Programming paradigm"
let paradigm_selection = doc.select(r#"table tr:has(a[title="Programming paradigm"]) td.infobox-data ul > li"#); 

println!("Rust programming paradigms:");
for item in paradigm_selection.iter() {
    println!(" {}", item.text());
}
println!("{:-<50}", "");

//since `th` contains text "Paradigms" without sibling tags, we can use `:has-text` pseudo class
let influenced_by_selection = doc.select(r#"table tr:has-text("Influenced by") + tr td  ul > li > a"#);

println!("Rust influenced by:");
for item in influenced_by_selection.iter() {
    println!(" {}", item.text());
}
println!("{:-<50}", "");

// Extract all links from the block that contains certain text. 
// Since `foreign function interface` located in its own tag,
// we have to use `:contains` pseudo class
let links_selection = doc.select(r#"p:contains("Rust has a foreign function interface") a[href^="/"]"#);

println!("Links in the FFI block:");
for item in links_selection.iter() {
    println!(" {}", item.attr("href").unwrap());
}
println!("{:-<50}", "");
```

</details>



## Related projects

* [html5ever](https://crates.io/crates/html5ever)
* [selectors](https://crates.io/crates/selectors)
* [select.rs](https://crates.io/crates/select)
* [goquery](https://godoc.org/github.com/PuerkitoBio/goquery)


## Features

- `hashbrown` -- optional, standard hashmaps and hashsets will be replaced `hashbrown` hashmaps and hashsets;

## Changelog
[Changelog](./CHANGELOG.md)

## License

Licensed under MIT ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


## Contribution

Any contribution intentionally submitted for inclusion in the work by you, shall be
licensed with MIT license, without any additional terms or conditions.
