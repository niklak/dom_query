//! HTML manipulation with CSS selectors.
//!
//! # Features
//!
//! * Iteration
//! * Manipulation
//! * Property
//! * Query
//! * Traversal
//!
//! # Examples
//!
//! ## Parsing a document
//! ```
//! use dom_query::Document;
//! use tendril::StrTendril;
//! // Document may consume `&str`, `String`, `StrTendril`
//! let contents_str = r#"<!DOCTYPE html>
//! <html><head><title>Test Page</title></head><body></body></html>"#;
//! let doc = Document::from(contents_str);
//!
//! let contents_string = contents_str.to_string();
//! let doc = Document::from(contents_string);
//!
//! let contents_tendril = StrTendril::from(contents_str);
//! let doc = Document::from(contents_tendril);
//!
//! // The root element for the `Document` is a Document
//! assert!(doc.root().is_document());
//!
//! // if the source has DocType, then the Document will also have one
//! // as a first child.
//! assert!(doc.root().first_child().unwrap().is_doctype());
//!
//! //both of them are not elements.
//! ```
//!
//! ## Parsing a fragment
//! ```
//! use dom_query::Document;
//! use tendril::StrTendril;
//! // fragment can be created with `Document::fragment()`, which accepts` &str`, `String`, `StrTendril`
//! let contents_str = r#"<!DOCTYPE html>
//! <html><head><title>Test Page</title></head><body></body></html>"#;
//! let fragment = Document::fragment(contents_str);
//!
//! let contents_string = contents_str.to_string();
//! let fragment = Document::fragment(contents_string);
//!
//! let contents_tendril = StrTendril::from(contents_str);
//! let fragment = Document::fragment(contents_tendril);
//!
//! // The root element for the  fragment is not a Document but a Fragment
//! assert!(!fragment.root().is_document());
//! assert!(fragment.root().is_fragment());
//!
//! // and when it parses a fragment, it drops Doctype
//! assert!(!fragment.root().first_child().unwrap().is_doctype());
//!
//! ```
//!
//! ## Selecting elements
//! ```
//! use dom_query::Document;
//!
//! let html = r#"<!DOCTYPE html>
//! <html>
//!     <head>
//!         <meta charset="utf-8">
//!         <title>Test Page</title>
//!     </head>
//!     <body>
//!         <h1>Test Page</h1>
//!         <ul>
//!             <li>One</li>
//!             <li><a href="/2">Two</a></li>
//!             <li><a href="/3">Three</a></li>
//!         </ul>
//!     </body>
//! </html>"#;
//! let document = Document::from(html);
//! // select a single element
//! let a = document.select("ul li:nth-child(2)");
//! let text = a.text().to_string();
//! assert!(text == "Two");
//!
//! // selecting multiple elements
//! document.select("ul > li:has(a)").iter().for_each(|el| {
//!     assert!(el.is("li"));
//! });
//!
//! // there is also `try_select` which returns an Option
//! let no_sel = document.try_select("p");
//! assert!(no_sel.is_none());
//!
//! ```
//!
//! ## Selecting a single match and multiple matches
//!
//! ```
//! use dom_query::Document;
//!
//! let doc: Document = r#"<!DOCTYPE html>
//! <html lang="en">
//! <head></head>
//! <body>
//!     <ul class="list">
//!         <li>1</li><li>2</li><li>3</li>
//!     </ul>
//!     <ul class="list">
//!         <li>4</li><li>5</li><li>6</li>
//!     </ul>
//! </body>
//! </html>"#
//! .into();
//! // if you need to select only the first, single match, you can use following:
//! let single_selection = doc.select_single(".list");
//! // access is only for the first matching:
//! assert_eq!(single_selection.length(), 1);
//! assert_eq!(single_selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");
//! // simple selection contain all matches:
//! let selection = doc.select(".list");
//! assert_eq!(selection.length(), 2);
//! // but if you call inner_html() on it, you will get the inner_html of the first match:
//! assert_eq!(selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");
//!
//! //this approach is using the first node from nodes vec and `select_single` consumes one iteration instead.
//! let first_selection = doc.select(".list").first();
//! assert_eq!(first_selection.length(), 1);
//! assert_eq!(first_selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");
//!
//! // this approach is consuming all nodes into vec at first, and then you can call `iter().next()` to get the first one.
//! let next_selection = doc.select(".list").iter().next().unwrap();
//! assert_eq!(next_selection.length(), 1);
//! assert_eq!(next_selection.inner_html().to_string().trim(), "<li>1</li><li>2</li><li>3</li>");
//!
//! // currently, to get data from all matches you need to iterate over them:
//! let all_matched: String = selection
//! .iter()
//! .map(|s| s.inner_html().trim().to_string())
//! .collect();
//!
//! assert_eq!(
//! all_matched,
//! "<li>1</li><li>2</li><li>3</li><li>4</li><li>5</li><li>6</li>"
//! );
//!
//! ```
//!
//! ## Selecting descendent elements
//! ```
//! use dom_query::Document;
//!
//! let html = r#"<!DOCTYPE html>
//! <html>
//!     <head>
//!         <meta charset="utf-8">
//!         <title>Test Page</title>
//!     </head>
//!     <body>
//!         <h1>Test Page</h1>
//!         <ul class="list-a">
//!             <li>One</li>
//!             <li><a href="/2">Two</a></li>
//!             <li><a href="/3">Three</a></li>
//!         </ul>
//!         <ul class="list-b">
//!             <li><a href="/4">Four</a></li>
//!         </ul>
//!     </body>
//! </html>"#;
//! let document = Document::from(html);
//! // select a parent element
//! let ul = document.select("ul");
//!
//! // selecting multiple elements
//! ul.select("li").iter().for_each(|el| {
//!     assert!(el.is("li"));
//! });
//!
//! // also descendant selector may be specified starting from the parent elements
//! let el = ul.select("body ul.list-b li").first();
//! let text = el.text();
//! assert_eq!("Four", text.to_string());
//! ```
//!
//!
//! ## Selecting with precompiled matchers (for reuse)
//!
//! ```
//! use dom_query::{Document, Matcher};
//!
//!
//! let html1 = r#"<!DOCTYPE html><html><head><title>Test Page 1</title></head><body></body></html>"#;
//! let html2 = r#"<!DOCTYPE html><html><head><title>Test Page 2</title></head><body></body></html>"#;
//! let doc1 = Document::from(html1);
//! let doc2 = Document::from(html2);
//!
//! // create a matcher once, reuse on different documents
//! let title_matcher = Matcher::new("title").unwrap();
//!
//! let title_el1 = doc1.select_matcher(&title_matcher);
//! assert_eq!(title_el1.text(), "Test Page 1".into());
//!
//! let title_el2 = doc2.select_matcher(&title_matcher);
//! assert_eq!(title_el2.text(), "Test Page 2".into());
//!
//! let title_single = doc1.select_single_matcher(&title_matcher);
//! assert_eq!(title_single.text(), "Test Page 1".into());
//!
//! ```
//! ## Manipulating the attribute of an HTML element
//!
//! ```
//! use dom_query::Document;
//!
//! let html = r#"<!DOCTYPE html>
//! <html>
//!     <head><title>Test</title></head>
//!     <body><input type="hidden" name="k" data-k="100"/></body>
//! </html>"#;
//!
//! let doc = Document::from(html);
//! let mut input_selection = doc.select("input[name=k]");
//!
//! // get the value of attribute "data-k"
//! let val = input_selection.attr("data-k").unwrap();
//! assert_eq!(val.to_string(), "100");
//!
//! // remove the attribute "data-k" from the element
//! input_selection.remove_attr("data-k");
//! // get the value of attribute "data-k", if missing, return default value
//! let val_or = input_selection.attr_or("data-k", "0");
//! assert_eq!(val_or.to_string(), "0");
//!
//! // set a attribute "data-k" with value "200"
//! input_selection.set_attr("data-k", "200");
//!
//! assert_eq!(input_selection.html(), r#"<input type="hidden" name="k" data-k="200">"#.into());
//! ```
//!
//! ## Serializing to HTML
//!
//! ```
//! use dom_query::Document;
//!
//! let html = r#"<!DOCTYPE html>
//! <html>
//!     <head><title>Test</title></head>
//!     <body><div class="content"><h1>Test Page</h1></div></body>
//! </html>"#;
//!
//! let doc = Document::from(html);
//! let heading_selector = doc.select("div.content");
//!
//! // serializing including the outer html tag
//! let content = heading_selector.html();
//! assert_eq!(content.to_string(), r#"<div class="content"><h1>Test Page</h1></div>"#);
//! // serializing without the outer html tag
//! let inner_content = heading_selector.inner_html();
//! assert_eq!(inner_content.to_string(), "<h1>Test Page</h1>");
//!
//! // there is also `try_html()` method, which returns an `Option<StrTendril>`,
//! // and if there is no matching selection it returns None
//! let opt_no_content = doc.select("div.no-content").try_html();
//! assert_eq!(opt_no_content, None);
//!
//! //Unlike` html()` method with return an empty `StrTendril`
//! let no_content = doc.select("div.no-content").html();
//! assert_eq!(no_content, "".into());
//!
//! //Same things works for `inner_html()` and `try_inner_html()` method.
//! assert_eq!(doc.select("div.no-content").try_inner_html(), None);
//! assert_eq!(doc.select("div.no-content").inner_html(), "".into());
//! ```
//!
//! ## Accessing descendent text
//!
//! ```
//! use dom_query::Document;
//!
//! let html = r#"<!DOCTYPE html>
//! <html>
//!     <head><title>Test</title></head>
//!     <body><div><h1>Test <span>Page</span></h1></div></body>
//! </html>"#;
//!
//! let doc = Document::from(html);
//!
//! let body_selection = doc.select("body div").first();
//! let text = body_selection.text();
//! assert_eq!(text.to_string(), "Test Page");
//!
//! ```
//!
//! ## Manipulating the DOM
//!
//! ```
//! use dom_query::Document;
//! let html_contents = r#"<!DOCTYPE html>
//! <html>
//!     <head><title>Test</title></head>
//!     <body>
//!         <div class="content">
//!             <p>9,8,7</p>
//!         </div>
//!         <div class="remove-it">
//!             Remove me
//!         </div>
//!     </body>
//! </html>"#;
//!
//! let doc = Document::from(html_contents);
//!
//! let mut content_selection = doc.select("body .content");
//! // append a new html node to the selection
//! content_selection.append_html(r#"<div class="inner">inner block</div>"#);
//! assert!(doc.select("body .content .inner").exists());
//!
//! // set a new content to the selection, replacing existing content
//! content_selection.set_html(r#"<div class="inner">1,2,3</div>"#);
//! assert_eq!(doc.select(".inner").text(), "1,2,3".into());
//!
//! // remove the selection
//! doc.select(".remove-it").remove();
//! assert!(!doc.select(".remove-it").exists());
//!
//! // replace the selection with a new html, current selection will not change.
//! let mut replace_selection = doc.select(".inner");
//! replace_selection.replace_with_html(r#"<div class="replaced">Replaced</div>"#);
//! assert_eq!(replace_selection.text(), "1,2,3".into());
//!
//! //but the dom will change
//! assert_eq!(doc.select(".replaced").text(),"Replaced".into());
//! assert!(!doc.select(".inner").exists());
//!
//! ```

// #![deny(missing_docs)]
extern crate html5ever;

mod css;
mod document;
mod dom_tree;
mod element;
mod entities;
mod fragment;
mod manipulation;
mod matcher;
mod property;
mod query;
mod selection;
mod traversal;

pub use document::Document;
#[doc(hidden)]
pub use dom_tree::SerializableNodeRef;
pub use dom_tree::{Node, NodeData, NodeRef};
#[doc(hidden)]
pub use entities::NodeId;
pub use matcher::Matcher;
pub use selection::Selection;
