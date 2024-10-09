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
                    exact_errors: false,
                    scripting_enabled: true,
                    iframe_srcdoc: false,
                    drop_doctype: true,
                    ignore_missing_rules: false,
                    quirks_mode: tree_builder::NoQuirks,
                },
            },
            QualName::new(None, ns!(html), local_name!("")),
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
