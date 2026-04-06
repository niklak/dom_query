//! A mini CSS selector parser and matcher.
//! 
//! This module provides a minimal CSS selector parser and matcher that supports a subset of the CSS selector syntax.

mod extension;
mod parser;
mod selector;

pub use parser::{parse_mini_selector, parse_selector_list};
pub use selector::MiniSelector;
