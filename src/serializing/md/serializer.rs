use std::cell::{Cell, Ref, RefCell};

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{Element, NodeId, Tree};

use crate::node::{NodeData, NodeRef, TreeNode, child_nodes};

use super::block::{
    Block, BlockWriter, Separator, is_block_wrapper, is_link_block, is_md_block, is_spaced_inline,
};
use super::constants::{DEFAULT_SKIP_TAGS, MAX_NESTING};
use super::ctx::FormatOpts;
use super::elements;
use super::host::MDHost;
use super::inline::Inline;

pub struct MDSerializer<'a> {
    pub(super) root_node: &'a NodeRef<'a>,
    pub(super) nodes: Ref<'a, Vec<TreeNode>>,
    pub(super) skip_tags: &'a [&'a str],
    last_list: Cell<Option<(NodeId, bool)>>,
    depth: Cell<usize>,
    // at most one per nesting level, freed with the serializer
    bufs: RefCell<Vec<String>>,
    /// Stacks of nodes to write, reused by nested `write` calls so none allocates its own.
    op_stacks: RefCell<Vec<Vec<Op>>>,
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
            op_stacks: RefCell::default(),
        }
    }

    pub fn serialize(&self, include_node: bool) -> StrTendril {
        let opts = FormatOpts {
            include_node,
            ..Default::default()
        };
        let mut out = BlockWriter::document(String::new(), opts);
        self.write(&mut out, self.root_node.id, opts);
        out.finish().into()
    }

    fn write_element(
        &self,
        out: &mut BlockWriter,
        e: &Element,
        id: NodeId,
        opts: FormatOpts,
    ) -> bool {
        let depth = self.depth.get();
        if depth >= MAX_NESTING {
            // these handlers add one frame at most: they dispatch to no other handler, or a
            // heading outside a cell only to children past the cap (link text holds no heading)
            let leaf = match e.name.local {
                local_name!("img")
                | local_name!("input")
                | local_name!("pre")
                | local_name!("code") => true,
                local_name!("h1")
                | local_name!("h2")
                | local_name!("h3")
                | local_name!("h4")
                | local_name!("h5")
                | local_name!("h6") => !opts.table_cell,
                _ => false,
            };
            if !leaf {
                return false;
            }
        }
        self.depth.set(depth + 1);
        let handled = elements::write_element(self, out, e, id, opts);
        self.depth.set(depth);
        handled
    }
}

/// Owes what separates an opening element from what precedes it: a space in a link body,
/// a blank line before a block, a line break before an item outside a list.
fn open_block(out: &mut BlockWriter, name: &QualName, opts: FormatOpts) {
    if opts.link_body {
        out.owe(Separator::Space);
    } else if name.local == local_name!("p") {
        out.open_paragraph();
    } else if is_md_block(name) {
        if !is_block_wrapper(name) {
            out.join_paragraphs();
        }
        out.owe(Separator::BlankLine);
        // a `---` line could underline the text above it in an item; a cell has no line for
        // a rule in an item
        if name.local == local_name!("hr") && !opts.table_cell {
            out.push_block(Block::Rule, if out.in_list_item() { "***" } else { "---" });
        } else if name.local == local_name!("hr") && !opts.list_item {
            out.push_raw("---");
        }
    } else {
        out.join_paragraphs();
        if name.local == local_name!("li") {
            out.owe(Separator::LineBreak);
        }
    }
}

/// A step of `write`; a close knows whether its element is a block (`is_link_block`).
enum Op {
    Open(NodeId),
    /// an element whose children are written; whether it is a block
    Close(NodeId, bool),
}

/// A word boundary: a block in code (md skipped), an option or svg elsewhere.
fn is_word_boundary(name: &QualName, opts: FormatOpts) -> bool {
    if opts.skip_md {
        is_link_block(name)
    } else {
        is_spaced_inline(name)
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

    fn write(&self, out: &mut BlockWriter, root_id: NodeId, opts: FormatOpts) {
        let mut ops = self.op_stacks.borrow_mut().pop().unwrap_or_default();
        let children = |id| child_nodes(Ref::clone(&self.nodes), &id, true).map(Op::Open);
        if opts.include_node {
            ops.push(Op::Open(root_id));
        } else {
            ops.extend(children(root_id));
        }
        while let Some(op) = ops.pop() {
            match op {
                Op::Open(id) => {
                    let Some(node) = self.nodes.get(id.value()) else {
                        continue;
                    };
                    match &node.data {
                        NodeData::Text { contents } => out.push_text(contents, opts),
                        NodeData::Element(e) => {
                            if self.is_skipped(&e.name) {
                                continue;
                            }
                            let block = is_link_block(&e.name);
                            if is_word_boundary(&e.name, opts) {
                                out.push(Inline::WordBoundary);
                            }
                            if !opts.skip_md {
                                if block {
                                    open_block(out, &e.name, opts);
                                }
                                // delegate serialization if the element has its own custom handler
                                if self.write_element(out, e, id, opts) {
                                    continue;
                                }
                            }
                            ops.push(Op::Close(id, block));
                            ops.extend(children(id));
                        }
                        _ => {}
                    }
                }
                Op::Close(id, block) => {
                    let Some(name) = self.nodes.get(id.value()).and_then(|n| n.as_element()) else {
                        continue;
                    };
                    let name = &name.name;
                    if is_word_boundary(name, opts) {
                        out.push(Inline::WordBoundary);
                    }
                    if block || name.local == local_name!("br") {
                        close_element(out, name, opts);
                    }
                }
            }
        }
        self.op_stacks.borrow_mut().push(ops);
    }
}

/// Ends an element written by `write`: a block owes the separator after it, a `<br>` or an
/// item, row or caption out of place ends its line.
fn close_element(out: &mut BlockWriter, name: &QualName, opts: FormatOpts) {
    if opts.skip_md {
        // `<br>` is a space when Markdown output is disabled
        if name.local == local_name!("br") {
            out.push(Inline::Text(" ", opts));
        }
    } else if opts.link_body && is_link_block(name) {
        out.owe(Separator::Space);
    } else if name.local == local_name!("p") {
        out.close_paragraph();
    } else if is_md_block(name) {
        out.owe(Separator::BlankLine);
    } else if matches!(
        name.local,
        local_name!("br") | local_name!("li") | local_name!("tr") | local_name!("caption")
    ) {
        if opts.link_body {
            // a hard break can leave a blank line inside `[...]`
            out.push(Inline::HtmlBreak);
        } else {
            if name.local == local_name!("br") {
                out.hard_break();
            } else {
                out.ensure_hard_break();
            }
            // an item that wrote nothing still sets the text around it apart
            if name.local == local_name!("li") && out.owed() >= Separator::LineBreak {
                out.owe(Separator::BlankLine);
            }
        }
    } else {
        // a `<td>` or `<th>`, the last link block: a table written as text keeps its cells apart
        out.owe(Separator::Space);
    }
}
#[cfg(test)]
#[allow(clippy::string_slice, clippy::indexing_slicing)]
mod tests {

    use super::super::block::{BlockEnd, LinePrefix, Preceding};
    use super::*;
    use crate::Document;

    #[test]
    fn test_item_end_from_middle_line() {
        // a block whose text is only on a middle line still sets its item end
        let mut out = BlockWriter::document(String::new(), FormatOpts::new());
        out.open_prefix(LinePrefix::ListItem {
            marker: "- ".to_string(),
            preceding: Preceding::Nothing,
        });
        out.push_raw("a");
        out.push_block(Block::Html, "\n<x>\n");
        assert!(out.close_prefix() == Some(BlockEnd::Html));
    }

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
        let doc = Document::from("<p>x</p>");
        let root = doc.root();
        let ser = MDSerializer::new(&root, None);
        for _ in 0..100 {
            ser.put_buf(String::with_capacity(8));
        }
        assert_eq!(ser.bufs.borrow().len(), 64);
    }
}
