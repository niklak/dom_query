use std::cell::Ref;

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{Element, NodeId};

use crate::node::{NodeData, NodeRef, SerializeOp, TreeNode, child_nodes};

use super::constants::DEFAULT_SKIP_TAGS;
use super::ctx::FormatOpts;
use super::elements::list::can_interrupt_paragraph;
use super::elements::table::add_cell_block_break;
use super::heading::{escape_closing_hashes, is_heading, md_prefix};
use super::text_utils::{
    add_linebreaks, linebreak, push_block_space, push_normalized_text, trim_space,
    trim_trailing_space,
};

pub struct MDSerializer<'a> {
    pub(super) root_node: &'a NodeRef<'a>,
    pub(super) nodes: Ref<'a, Vec<TreeNode>>,
    pub(super) skip_tags: &'a [&'a str],
}

impl<'a> MDSerializer<'a> {
    pub fn new(root_node: &'a NodeRef, skip_tags: Option<&'a [&'a str]>) -> Self {
        let skip_tags = skip_tags.unwrap_or(&DEFAULT_SKIP_TAGS);
        let nodes = root_node.tree.nodes.borrow();
        MDSerializer {
            root_node,
            nodes,
            skip_tags,
        }
    }

    pub fn serialize(&self, include_node: bool) -> StrTendril {
        let mut text = String::new();
        let opts = FormatOpts {
            include_node,
            ..Default::default()
        };
        self.write(&mut text, self.root_node.id, opts);
        text.into()
    }

    pub(super) fn write(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        let linebreak = linebreak(opts.table_cell);
        let double_br = linebreak.repeat(2);

        let mut ops = if opts.include_node {
            vec![SerializeOp::Open(root_id)]
        } else {
            child_nodes(Ref::clone(&self.nodes), &root_id, true)
                .map(SerializeOp::Open)
                .collect()
        };

        while let Some(op) = ops.pop() {
            match op {
                SerializeOp::Open(id) => {
                    let node = &self.nodes[id.value()];
                    match &node.data {
                        NodeData::Text { contents } => {
                            push_normalized_text(text, contents, opts);
                        }
                        NodeData::Element(e) => {
                            if self.skip_tags.contains(&e.name.local.as_ref()) {
                                continue;
                            }

                            // linebreaks are disabled for md blocks if md is skipped (for code);
                            // a table cell joins its blocks with `<br>`
                            if !opts.skip_md {
                                if opts.link_body {
                                    if is_link_block(&e.name) {
                                        push_block_space(text);
                                    }
                                } else if is_md_block(&e.name) {
                                    if opts.table_cell {
                                        add_cell_block_break(text, linebreak);
                                    } else {
                                        let node = NodeRef::new(id, self.root_node.tree);
                                        // a blank line before a nested list would make the parent list loose
                                        let tight = opts.list_item
                                            && is_list(&e.name)
                                            && can_interrupt_paragraph(&node);
                                        let end = if tight { linebreak } else { &double_br };
                                        add_linebreaks(text, linebreak, end);
                                    }
                                }
                                // push md prefixes only when md mode is active
                                if !opts.link_body {
                                    if let Some(prefix) = md_prefix(&e.name) {
                                        text.push_str(prefix);
                                    }
                                }
                                // delegate serialization if the element has its own custom handler
                                if self.write_element(text, e, node, opts) {
                                    continue;
                                }
                            }
                            ops.push(SerializeOp::Close(&e.name));
                            ops.extend(
                                child_nodes(Ref::clone(&self.nodes), &id, true)
                                    .map(SerializeOp::Open),
                            );
                        }
                        _ => {}
                    }
                }
                SerializeOp::Close(name) => {
                    if text.ends_with(&double_br) {
                        continue;
                    }

                    if opts.link_body && is_link_block(name) {
                        push_block_space(text);
                    } else if !opts.skip_md {
                        if is_heading(name) {
                            escape_closing_hashes(text);
                        }
                        if is_md_block(name) {
                            if opts.table_cell {
                                add_cell_block_break(text, linebreak);
                            } else {
                                add_linebreaks(text, linebreak, &double_br);
                            }
                        } else if matches!(
                            name.local,
                            local_name!("br") | local_name!("li") | local_name!("tr")
                        ) {
                            trim_trailing_space(text);
                            if opts.link_body {
                                // a hard break can leave a blank line inside `[...]`
                                text.push_str("<br>");
                            } else {
                                // normalize `<br>` as linebreak
                                // fallback for `li` and `tr` elements rendered outside their standard context
                                text.push_str("  ");
                                text.push_str(linebreak);
                            }
                        }
                    } else if name.local == local_name!("br") && !text.ends_with(' ') {
                        // normalize `<br>` as a space when Markdown output is disabled
                        text.push(' ');
                    }
                }
            }
        }

        if !opts.include_node {
            trim_space(text);
        }
    }

    pub(super) fn write_text(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        let mut ops = if opts.include_node {
            vec![root_id]
        } else {
            child_nodes(Ref::clone(&self.nodes), &root_id, true).collect()
        };

        while let Some(id) = ops.pop() {
            let node = &self.nodes[id.value()];
            if let NodeData::Text { contents } = &node.data {
                push_normalized_text(text, contents, opts);
            } else if let NodeData::Element(_e) = &node.data {
                ops.extend(child_nodes(Ref::clone(&self.nodes), &id, true));
            }
        }
    }

    fn write_element(
        &self,
        text: &mut String,
        e: &Element,
        tree_node: &TreeNode,
        opts: FormatOpts,
    ) -> bool {
        let mut matched = true;

        match e.name.local {
            // their markers cannot go in link text; their children are
            // written as plain blocks
            local_name!("ol")
            | local_name!("ul")
            | local_name!("blockquote")
            | local_name!("table")
                if opts.link_body =>
            {
                matched = false;
            }
            local_name!("ol") | local_name!("ul") => self.write_list(text, tree_node, e, opts),
            local_name!("a") => self.write_link(text, tree_node),
            local_name!("img") => Self::write_img(text, tree_node),
            local_name!("pre") => self.write_pre(text, tree_node, opts),
            local_name!("blockquote") => self.write_blockquote(text, tree_node),
            local_name!("table") => self.write_table(text, tree_node),
            local_name!("code") => self.write_code(text, tree_node, opts),
            local_name!("strong") | local_name!("b") | local_name!("em") | local_name!("i") => {
                self.write_emphasis(text, tree_node, opts);
            }
            _ => matched = false,
        }
        matched
    }
}

// Limited set of elements treated as block-level in Markdown output.
pub(super) const fn is_md_block(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("article")
            | local_name!("blockquote")
            | local_name!("section")
            | local_name!("div")
            | local_name!("p")
            | local_name!("h1")
            | local_name!("h2")
            | local_name!("h3")
            | local_name!("h4")
            | local_name!("h5")
            | local_name!("h6")
            | local_name!("ul")
            | local_name!("ol")
            | local_name!("dl")
            | local_name!("table")
            | local_name!("hr")
    )
}

/// An element that is written as its own line, or table cell, outside link text.
fn is_link_block(name: &QualName) -> bool {
    // TODO: add dt/dd and the other HTML5 block elements missing from `is_md_block`.
    is_md_block(name)
        || matches!(
            name.local,
            local_name!("li") | local_name!("tr") | local_name!("th") | local_name!("td")
        )
}

pub(super) const fn is_list(name: &QualName) -> bool {
    matches!(name.local, local_name!("ul") | local_name!("ol"))
}
