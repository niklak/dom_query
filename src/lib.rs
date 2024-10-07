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
//! let html = r#"<!DOCTYPE html>
//! <html><head><title>Test Page</title></head><body></body></html>"#;
//! let document = Document::from(html);

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
//! })
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

//TODO: implement iterator for Selection
//TODO: parse fragment?
