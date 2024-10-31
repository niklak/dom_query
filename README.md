
# DOM_QUERY: A Flexible Rust Crate for DOM Querying and Manipulation

[![Crates.io version](https://img.shields.io/crates/v/dom_query.svg?style=flat)](https://crates.io/crates/dom_query)
[![Download](https://img.shields.io/crates/d/dom_query.svg?style=flat)](https://crates.io/crates/dom_query)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat)](https://docs.rs/dom_query)
[![codecov](https://codecov.io/gh/niklak/dom_query/graph/badge.svg?token=CFAVOIE61O)](https://codecov.io/gh/niklak/dom_query)

[![Build Status](https://github.com/niklak/dom_query/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/niklak/dom_query/actions/workflows/rust.yml)
[![Raspberry Pi 5 CI](https://github.com/niklak/dom_query/actions/workflows/raspberrypi5.yml/badge.svg)](https://github.com/niklak/dom_query/actions/workflows/raspberrypi5.yml)
[![wasm ci](https://github.com/niklak/dom_query/actions/workflows/wasm.yml/badge.svg)](https://github.com/niklak/dom_query/actions/workflows/wasm.yml)


DOM_QUERY is a flexible Rust crate that simplifies HTML parsing, DOM querying and manipulation by providing a high-level jQuery-like API. It uses the `html5ever` crate for HTML parsing and the `selectors` crate for efficient DOM traversal and element selection.

## Features

- Parse HTML documents and fragments
- Query DOM elements using CSS selectors
- Traverse the DOM tree (ancestors, parents, children, siblings)
- Manipulate elements and their attributes:
  - Add/remove/modify attributes
  - Change element content
  - Add/remove elements
  - Rename elements
  - Move elements within the DOM tree

> [!NOTE]
> This crate is a significantly enhanced fork of [nipper](https://crates.io/crates/nipper),
> featuring expanded CSS selector support, enhanced DOM traversal  and improved DOM manipulation capabilities.


## Examples


<details>
<summary><b>Parsing a document</b></summary>

```rust
use dom_query::Document;
use tendril::StrTendril;
// Document may consume &str, String, StrTendril
let contents_str = r#"<!DOCTYPE html>
<html><head><title>Test Page</title></head><body></body></html>"#;
let doc = Document::from(contents_str);

let contents_string = contents_str.to_string();
let doc = Document::from(contents_string);

let contents_tendril = StrTendril::from(contents_str);
let doc = Document::from(contents_tendril);

// The root element for the `Document` is a Document
assert!(doc.root().is_document());

// if the source has DocType, then the Document will also have one
// as a first child.
assert!(doc.root().first_child().unwrap().is_doctype());

//both of them are not elements.
```
</details>


<details>
<summary><b>Parsing a fragment</b></summary>

```rust
use dom_query::Document;
use tendril::StrTendril;
// fragment can be created with Document::fragment(), which accepts &str, String, StrTendril
let contents_str = r#"<!DOCTYPE html>
<html><head><title>Test Page</title></head><body></body></html>"#;
let fragment = Document::fragment(contents_str);

let contents_string = contents_str.to_string();
let fragment = Document::fragment(contents_string);

let contents_tendril = StrTendril::from(contents_str);
let fragment = Document::fragment(contents_tendril);

// The root element for the  fragment is not a Document but a Fragment
assert!(!fragment.root().is_document());
assert!(fragment.root().is_fragment());

// and when it parses a fragment, it drops Doctype
assert!(!fragment.root().first_child().unwrap().is_doctype());
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

// there is also `try_select` which returns an Option
let no_sel = document.try_select("p");
assert!(no_sel.is_none());

```
</details>


<details>
<summary><b>Selecting a single match and multiple matches</b></summary>

```rust
use dom_query::Document;
let doc: Document = r#"<!DOCTYPE html>
<html lang="en">
<head></head>
<body>
    <ul class="list">
        <li>1</li><li>2</li><li>3</li>
    </ul>
    <ul class="list">
        <li>4</li><li>5</li><li>6</li>
    </ul>
</body>
</html>"#
    .into();
// if you need to select only the first, single match, you can use following:
let single_selection = doc.select_single(".list");

// access is only for the first matching:
assert_eq!(single_selection.length(), 1);
assert_eq!(single_selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");

// simple selection contains all matches:
let selection = doc.select(".list");
assert_eq!(selection.length(), 2);

// but if you call inner_html() on it, you will get the inner_html of the first match:
assert_eq!(selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");

//this approach is using the first node from nodes vec and `select_single` consumes one iteration instead.
let first_selection = doc.select(".list").first();
assert_eq!(first_selection.length(), 1);
assert_eq!(first_selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");

// this approach is consuming all nodes into vec at first, and then you can call `iter().next()` to get the first one.
let next_selection = doc.select(".list").iter().next().unwrap();
assert_eq!(next_selection.length(), 1);
assert_eq!(next_selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");

// currently, to get data from all matches you need to iterate over them, either:
let all_matched: String = selection.iter().map(|s| s.inner_html().trim().to_string()).collect();
assert_eq!(
all_matched,
"<li>1</li><li>2</li><li>3</li><li>4</li><li>5</li><li>6</li>"
);

// or:
let all_matched: String = selection.nodes().iter().map(|s| s.inner_html().trim().to_string()).collect();
/ which is more efficient.
assert_eq!(
all_matched,
"<li>1</li><li>2</li><li>3</li><li>4</li><li>5</li><li>6</li>"
);
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
    <summary><b>Selecting ancestors</b></summary>


```rust
use dom_query::Document;

let doc: Document = r#"<!DOCTYPE html>
<html>
    <head>Test</head>
    <body>
        <div id="great-ancestor">
            <div id="grand-parent">
                <div id="parent">
                    <div id="child">Child</div>
                </div>
            </div>
        </div>
    </body>
</html>
"#.into();

// selecting an element
let child_sel = doc.select("#child");
assert!(child_sel.exists());

let child_node = child_sel.nodes().first().unwrap();

// getting all ancestors
let ancestors = child_node.ancestors(None);

let ancestor_sel = Selection::from(ancestors);

// in this case ancestors includes all ancestral nodes including html

// the root html element is presented in the ancestor selection
assert!(ancestor_sel.is("html"));

// also the direct parent of our starting node is presented
assert!(ancestor_sel.is("#parent"));

// `Selection::is` matches only the current selection without descending down the tree,
// so it won't match the #child node.
assert!(!ancestor_sel.is("#child"));


// if you don't require all ancestors, you can specify a number of ancestors you need -- `max_limit`
let ancestors = child_node.ancestors(Some(2));
let ancestor_sel = Selection::from(ancestors);

// in this case ancestors includes only two ancestral nodes: #grand-parent and #parent
assert!(ancestor_sel.is("#grand-parent #parent"));

assert!(!ancestor_sel.is("#great-ancestor"));

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
// selecting a single match
let title_single = doc1.select_single_matcher(&title_matcher);
assert_eq!(title_single.text(), "Test Page 1".into());
```
</details>


<details>
<summary><b>Selecting with pseudo-classes (:has, :has-text, :contains)</b></summary>

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

//since `th` contains text "Influenced by" without sibling tags, we can use `:has-text` pseudo class
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

// there is also `try_html()` method, which returns an `Option<StrTendril>`,
// and if there is no matching selection it returns None
let opt_no_content = doc.select("div.no-content").try_html();
assert_eq!(opt_no_content, None);

//`html()` method will return an empty `StrTendril` if there is no matching selection
let no_content = doc.select("div.no-content").html();
assert_eq!(no_content, "".into());

//Same things works for `inner_html()` and `try_inner_html()` method.
assert_eq!(doc.select("div.no-content").try_inner_html(), None);
assert_eq!(doc.select("div.no-content").inner_html(), "".into());
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
<summary><b>Accessing immediate text</b></summary>

```rust
use dom_query::Document;

let html = r#"<!DOCTYPE html>
<html>
    <head><title>Test</title></head>
    <body><div><h1>Test <span>Page</span></h1></div></body>
</html>"#;

let doc = Document::from(html);

let body_selection = doc.select("body div h1").first();
// accessing immediate text without descendants
let text = body_selection.immediate_text();
assert_eq!(text.to_string(), "Test ");

```

</details>



<details>
<summary><b>Manipulating the attribute of an HTML element</b></summary>

```rust
use dom_query::Document;
let html = r#"<!DOCTYPE html>
<html>
    <head><title>Test</title></head>
    <body><input hidden="" id="k" class="important" type="hidden" name="k" data-k="100"></body>
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

// remove a list of attributes from the element
input_selection.remove_attrs(&["id", "class"]);
// set a attribute "data-k" with value "200"
input_selection.set_attr("data-k", "200");

assert_eq!(input_selection.html(), r#"<input hidden="" type="hidden" name="k" data-k="200">"#.into());

// check if attribute "hidden" exists on the element
let is_hidden = input_selection.has_attr("hidden");
assert!(is_hidden);
let has_title = input_selection.has_attr("title");
assert!(!has_title);


// remove all attributes from the element
input_selection.remove_all_attrs();
assert_eq!(input_selection.html(), r#"<input>"#.into());

```
</details>

<details>
    <summary><b>Manipulating the DOM</b></summary>

```rust
use dom_query::Document;
let html_contents = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="content">
                <p>9,8,7</p>
            </div>
            <div class="remove-it">
                Remove me
            </div>
            <div class="replace-it">
                <div>Replace me</div>
            </div>
        </body>
    </html>"#;;

let doc = Document::from(html_contents);

let mut content_selection = doc.select("body .content");
// append a new html node to the selection
content_selection.append_html(r#"<div class="inner">inner block</div>"#);
assert!(doc.select("body .content .inner").exists());

// set a new content to the selection, replacing existing content
let mut set_selection = doc.select(".inner");
set_selection.set_html(r#"<p>1,2,3</p>"#);
assert_eq!(doc.select(".inner").html(), r#"<div class="inner"><p>1,2,3</p></div>"#.into());

// remove the selection
doc.select(".remove-it").remove();
assert!(!doc.select(".remove-it").exists());

// replace the selection with a new html, current selection will not change.
let mut replace_selection = doc.select(".replace-it");
replace_selection.replace_with_html(r#"<div class="replaced">Replaced</div>"#);
assert_eq!(replace_selection.text().trim(), "Replace me");

//but the document will change
assert_eq!(doc.select(".replaced").text(),"Replaced".into());
```
</details>


<details>
    <summary><b>Node manipulations: Creating an empty element, adding a single element to a single node</b></summary>

```rust
use dom_query::Document;

let doc: Document = r#"<!DOCTYPE html>
<html lang="en">
<head></head>
<body>
    <div id="main">
        <p id="first">It's</p>
    <div>
</body>
</html>"#.into();

// selecting a node we want to attach a new element
let main_sel = doc.select_single("#main");
let main_node = main_sel.nodes().first().unwrap();

// if you need just to create an empty element, then you can use the following:
let el = doc.tree.new_element("p");
// you still able to deal with element's attributes:
el.set_attr("id", "second");
main_node.append_child(&el.id);
assert!(doc.select("#main #second").exists());
// because this method doesn't parse anything it is much more cheaper than following approaches.

// if you need to add a more complex element, you can use `node.append_html`,
// which is much more convenient, then previous approach:

main_node.append_html(r#"<p id="third">Wonderful</p>"#);
assert_eq!(doc.select("#main #third").text().as_ref(), "Wonderful");
assert!(doc.select("#first").exists());

// if we need to replace existing element content with a new one, then use `node.set_html`:
main_node.set_html(r#"<p id="the-only">Wonderful</p>"#);
assert_eq!(doc.select("#main #the-only").text().as_ref(), "Wonderful");
assert!(!doc.select("#first").exists());
```
</details>


<details>
    <summary><b>Renaming selected elements without changing the contents</b></summary>


```rust
use dom_query::Document;

let doc: Document = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <div class="content">
        <div>1</div>
        <div>2</div>
        <div>3</div>
        <span>4</span>
    </div>
<body>
</html>"#
.into();
let mut sel = doc.select("div.content > div, div.content > span");
// before renaming, there are 3 `div` and 1 `span`
assert_eq!(sel.length(), 4);

sel.rename("p");

// after renaming, there are no `div` and `span` elements
assert_eq!(doc.select("div.content > div, div.content > span").length(), 0);
// but there are three `p` elements
assert_eq!(doc.select("div.content > p").length(), 4);
```
</details>

- **[more examples](./examples/)**
- **[dom_query by example](https://github.com/niklak/dom_query/wiki/dom_query-by-Example)**



## Related projects

* [html5ever](https://crates.io/crates/html5ever)
* [selectors](https://crates.io/crates/selectors)
* [select.rs](https://crates.io/crates/select)
* [goquery](https://godoc.org/github.com/PuerkitoBio/goquery)
* [dom_finder](https://crates.io/crates/dom_finder)


## Crate features

- `hashbrown` — optional, standard hashmaps and hashsets will be replaced `hashbrown` hashmaps and hashsets;

## Possible issues
* [wasm32 compilation](https://github.com/niklak/dom_query/wiki/WASM32-compilation)


## Changelog
[Changelog](./CHANGELOG.md)

## License

Licensed under MIT ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


## Contribution

Any contribution intentionally submitted for inclusion in the work by you, shall be
licensed with MIT license, without any additional terms or conditions.
