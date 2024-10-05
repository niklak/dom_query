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
//! let html = r#"<DOCTYPE html>
//! <html><head><title>Test Page</title></head><body></body></html>"#;
//! let document = Document::from(html);

//! ```
//!
//! ## Selecting elements
//! ```
//! use dom_query::Document;
//!
//! let html = r#"<DOCTYPE html>
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
//! let html = r#"<DOCTYPE html>
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
//! let html1 = r#"<DOCTYPE html><html><head><title>Test Page 1</title></head><body></body></html>"#;
//! let html2 = r#"<DOCTYPE html><html><head><title>Test Page 2</title></head><body></body></html>"#;
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
//! ## Accessing element's attribute
//! 
//! ```
//! use dom_query::Document;
//! 
//! let html = r#"<DOCTYPE html>
//! <html>
//!     <head><title>Test</title></head>
//!     <body><input type="hidden" name="k" value="test"/></body>
//! </html>"#;
//! 
//! let val = Document::from(html).select("input[name=k]").first().attr("value").unwrap();
//! assert_eq!(val.to_string(), "test");
//! 
//! ```
//! 
//! ## Serializing to HTML
//! 
//! ```
//! use dom_query::Document;
//! 
//! let html = r#"<DOCTYPE html>
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
//! ```
//! 
//! ## Accessing descendent text
//! 
//! ```
//! use dom_query::Document;
//! 
//! let html = r#"<DOCTYPE html>
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
//! 

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
