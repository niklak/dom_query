use std::cell::{Cell, RefCell};

use html5ever::tree_builder;
use html5ever::ParseOpts;
use html5ever::QualName;
use html5ever::{local_name, namespace_url, ns};

use tendril::StrTendril;
use tendril::TendrilSink;

use crate::dom_tree::{NodeData, Tree};
use crate::Document;

impl Document {
    /// Create a new html document fragment
    pub fn fragment<T: Into<StrTendril>>(html: T) -> Self {
        html5ever::parse_fragment(
            Document::fragment_sink(),
            ParseOpts {
                tokenizer: Default::default(),
                tree_builder: tree_builder::TreeBuilderOpts {
                    drop_doctype: true,
                    ..Default::default()
                },
            },
            QualName::new(None, ns!(html), local_name!("body")),
            Vec::new(),
        )
        .one(html)
    }
    /// Create a new sink for a html document fragment
    pub fn fragment_sink() -> Self {
        Self {
            tree: Tree::new(NodeData::Fragment),
            errors: RefCell::new(vec![]),
            quirks_mode: Cell::new(tree_builder::NoQuirks),
        }
    }
}
