//! HTML manipulation with CSS selectors.
//!
//! # Features
//!
//! - Parse HTML documents and fragments
//! - Query DOM elements using CSS selectors
//! - Traverse the DOM tree (ancestors, parents, children, siblings)
//! - Manipulate elements and their attributes:
//!   - Add/remove/modify attributes
//!   - Change element content
//!   - Add/remove elements
//!   - Rename elements
//!   - Move elements within the DOM tree
//!

#![doc= include_str!("../Examples.md")]

mod css;
mod document;
mod dom_tree;
mod entities;
mod matcher;
mod mini_selector;
mod node;
mod selection;
mod serializing;

pub use document::Document;
pub use dom_tree::Tree;
pub use dom_tree::TreeNodeOps;
pub use matcher::Matcher;
pub use mini_selector::MiniSelector;
pub use mini_selector::{parse_selector_list, parse_single_selector};
#[doc(hidden)]
pub use node::SerializableNodeRef;
pub use node::{Element, Node, NodeData, NodeId, NodeIdProver, NodeRef};
pub use selection::Selection;
