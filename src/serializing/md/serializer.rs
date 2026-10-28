use std::cell::Ref;

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{NodeId, Tree};

use crate::node::{NodeData, NodeRef, SerializeOp, TreeNode, child_nodes};

use super::block::{is_link_block, is_list, is_md_block};
use super::constants::DEFAULT_SKIP_TAGS;
use super::ctx::FormatOpts;
use super::elements;
use super::elements::list::list_can_interrupt_paragraph;
use super::elements::table::add_cell_block_break;
use super::heading::{escape_closing_hashes, is_heading, md_prefix};
use super::host::MDHost;
use super::text_utils::{
    add_linebreaks, double_linebreak, linebreak, push_block_space, push_normalized_text,
    trim_space, trim_trailing_space,
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
}

impl MDHost for MDSerializer<'_> {
    fn tree(&self) -> &Tree {
        self.root_node.tree
    }

    fn nodes(&self) -> Ref<'_, Vec<TreeNode>> {
        Ref::clone(&self.nodes)
    }

    fn is_skipped(&self, name: &QualName) -> bool {
        self.skip_tags.contains(&name.local.as_ref())
    }

    fn write(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        let linebreak = linebreak(opts.table_cell);
        let double_br = double_linebreak(opts.table_cell);

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
                            if self.is_skipped(&e.name) {
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
                                            && list_can_interrupt_paragraph(&node);
                                        let end = if tight { linebreak } else { double_br };
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
                                if elements::write_element(self, text, e, id, opts) {
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
                    if text.ends_with(double_br) {
                        continue;
                    }

                    if opts.link_body && is_link_block(name) {
                        // no joining space when nothing follows in the link body
                        if ops
                            .iter()
                            .rev()
                            .any(|op| matches!(op, SerializeOp::Open(_)))
                        {
                            push_block_space(text);
                        }
                    } else if !opts.skip_md {
                        if is_heading(name) {
                            escape_closing_hashes(text);
                        }
                        if is_md_block(name) {
                            if opts.table_cell {
                                add_cell_block_break(text, linebreak);
                            } else {
                                add_linebreaks(text, linebreak, double_br);
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

        if !opts.include_node && !opts.link_body {
            trim_space(text);
        }
    }
}
