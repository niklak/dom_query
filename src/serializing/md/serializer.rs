use std::cell::{Cell, Ref, RefCell};

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{Element, NodeId, Tree};

use crate::node::{NodeData, NodeRef, SerializeOp, TreeNode, child_nodes};

use super::block::{is_link_block, is_list, is_md_block};
use super::constants::{DEFAULT_SKIP_TAGS, MAX_NESTING};
use super::ctx::FormatOpts;
use super::elements;
use super::elements::list::{follows_list, list_can_interrupt_paragraph};
use super::elements::table::add_cell_block_break;
use super::heading::md_prefix;
use super::host::MDHost;
use super::text_utils::{
    add_linebreaks, double_linebreak, linebreak, push_block_space, push_normalized_text,
    trim_space, trim_trailing_space,
};

pub struct MDSerializer<'a> {
    pub(super) root_node: &'a NodeRef<'a>,
    pub(super) nodes: Ref<'a, Vec<TreeNode>>,
    pub(super) skip_tags: &'a [&'a str],
    last_list: Cell<Option<(NodeId, bool)>>,
    depth: Cell<usize>,
    // at most one per nesting level, freed with the serializer
    bufs: RefCell<Vec<String>>,
}

impl<'a> MDSerializer<'a> {
    pub fn new(root_node: &'a NodeRef, skip_tags: Option<&'a [&'a str]>) -> Self {
        let skip_tags = skip_tags.unwrap_or(&DEFAULT_SKIP_TAGS);
        let nodes = root_node.tree.nodes.borrow();
        MDSerializer {
            root_node,
            nodes,
            skip_tags,
            last_list: Cell::new(None),
            depth: Cell::new(0),
            bufs: RefCell::default(),
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

    fn write_element(&self, text: &mut String, e: &Element, id: NodeId, opts: FormatOpts) -> bool {
        let depth = self.depth.get();
        // these handlers never dispatch to other handlers, so they add one frame at most
        let leaf = matches!(
            e.name.local,
            local_name!("img") | local_name!("input") | local_name!("pre") | local_name!("code")
        );
        if depth >= MAX_NESTING && !leaf {
            return false;
        }
        self.depth.set(depth + 1);
        let handled = elements::write_element(self, text, e, id, opts);
        self.depth.set(depth);
        handled
    }

    /// Separates an opening element from what precedes it: a space in a link body,
    /// a line break before a block, `<br>` in a table cell.
    fn open_block(
        &self,
        text: &mut String,
        name: &QualName,
        id: NodeId,
        opts: FormatOpts,
        double_br: &str,
    ) {
        let linebreak = linebreak(opts.table_cell);
        if opts.link_body {
            if is_link_block(name) {
                push_block_space(text);
            }
        } else if is_md_block(name) {
            if opts.table_cell {
                add_cell_block_break(text, linebreak);
            } else {
                let node = NodeRef::new(id, self.root_node.tree);
                // a blank line before a nested list would make the parent list loose
                let tight = opts.list_item
                    && is_list(name)
                    && (list_can_interrupt_paragraph(self, &node) || follows_list(self, &node));
                let end = if tight { linebreak } else { double_br };
                add_linebreaks(text, linebreak, end);
            }
        }
    }
}

impl MDHost for MDSerializer<'_> {
    fn take_buf(&self) -> String {
        self.bufs.borrow_mut().pop().unwrap_or_default()
    }

    fn put_buf(&self, mut buf: String) {
        let mut pool = self.bufs.borrow_mut();
        // only pathological nesting fills this; past that, free instead of retaining
        if pool.len() >= 64 {
            return;
        }
        buf.clear();
        pool.push(buf);
    }

    fn tree(&self) -> &Tree {
        self.root_node.tree
    }

    fn nodes(&self) -> Ref<'_, Vec<TreeNode>> {
        Ref::clone(&self.nodes)
    }

    fn is_skipped(&self, name: &QualName) -> bool {
        self.skip_tags.contains(&name.local.as_ref())
    }

    /// The last list written and whether the next same-type list must use the
    /// alternate marker.
    fn last_list(&self) -> Option<(NodeId, bool)> {
        self.last_list.get()
    }

    /// Records the last written list; `alt_marker` is the marker that list used,
    /// and the next same-type list gets the opposite one.
    fn set_last_list(&self, list: NodeId, alt_marker: bool) {
        self.last_list.set(Some((list, !alt_marker)));
    }

    fn write(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        // a link label is single-line text: only its pipe escaping follows the cell
        // shortcut: a new cell layout rule reachable in a label must check link_body too
        let double_br = double_linebreak(opts.table_cell && !opts.link_body);

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

                            // linebreaks are disabled for md blocks if md is skipped (for code)
                            if !opts.skip_md {
                                self.open_block(text, &e.name, id, opts, double_br);
                                // push md prefixes only when md mode is active
                                if !opts.link_body {
                                    if let Some(prefix) = md_prefix(&e.name) {
                                        text.push_str(prefix);
                                    }
                                }
                                // delegate serialization if the element has its own custom handler
                                if self.write_element(text, e, id, opts) {
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
                    if !text.ends_with(double_br) {
                        close_element(text, name, opts, &ops);
                    }
                }
            }
        }

        if !opts.include_node && !opts.link_body {
            trim_space(text);
        }
    }
}

fn ends_with_hard_break(name: &QualName, text: &str, linebreak: &str) -> bool {
    name.local != local_name!("br")
        && text
            .strip_suffix(linebreak)
            .is_some_and(|rest| rest.ends_with("  "))
}

/// Ends an element written by `write`; `ops` holds the work still to do.
fn close_element(text: &mut String, name: &QualName, opts: FormatOpts, ops: &[SerializeOp]) {
    let linebreak = linebreak(opts.table_cell);
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
        if is_md_block(name) {
            add_block_end(text, opts.table_cell);
        } else if matches!(
            name.local,
            local_name!("br") | local_name!("li") | local_name!("tr") | local_name!("caption")
        ) {
            if ends_with_hard_break(name, text, linebreak) {
                return;
            }
            trim_trailing_space(text);
            if opts.link_body {
                // a hard break can leave a blank line inside `[...]`
                text.push_str("<br>");
            } else {
                // normalize `<br>` as linebreak
                // fallback for `li`, `tr` and `caption` elements outside their standard context
                text.push_str("  ");
                text.push_str(linebreak);
            }
        } else if matches!(name.local, local_name!("td") | local_name!("th")) {
            // a table written as text keeps its cells apart
            push_block_space(text);
        }
    } else if name.local == local_name!("br") && !text.ends_with(' ') {
        // normalize `<br>` as a space when Markdown output is disabled
        text.push(' ');
    }
}

/// Ends a block: a blank line, or a `<br>` inside a table cell.
pub(super) fn add_block_end(text: &mut String, table_cell: bool) {
    if table_cell {
        add_cell_block_break(text, linebreak(true));
    } else {
        add_linebreaks(text, linebreak(false), double_linebreak(false));
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::Document;

    #[test]
    fn test_scratch_reuses_buffer() {
        let doc = Document::from("<p>x</p>");
        let root = doc.root();
        let ser = MDSerializer::new(&root, None);
        {
            let mut buf = ser.scratch();
            buf.push_str("abc");
        }
        let buf = ser.scratch();
        assert!(buf.is_empty());
        assert!(buf.capacity() >= 3);
    }

    #[test]
    fn test_scratch_pool_is_capped() {
        let html = format!(
            "{}x{}",
            "<blockquote>".repeat(100),
            "</blockquote>".repeat(100)
        );
        let doc = Document::from(html);
        let root = doc.root();
        let ser = MDSerializer::new(&root, None);
        ser.serialize(false);
        assert_eq!(ser.bufs.borrow().len(), 64);
    }
}
