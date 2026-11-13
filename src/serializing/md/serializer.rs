use std::cell::{Cell, Ref, RefCell};

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{Element, NodeId, Tree};

use crate::node::{NodeData, NodeRef, SerializeOp, TreeNode, child_nodes};

use super::block::{is_link_block, is_list, is_md_block, is_spaced_inline};
use super::constants::{DEFAULT_SKIP_TAGS, MAX_NESTING};
use super::ctx::FormatOpts;
use super::elements;
use super::elements::table::{add_cell_block_break, ends_with_cell_break};
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
    /// A word boundary left at the end of a write, for the write that follows it.
    word_space: Cell<bool>,
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
            word_space: Cell::new(false),
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

    fn write_element(
        &self,
        text: &mut String,
        e: &Element,
        id: NodeId,
        opts: FormatOpts,
        space: &mut PendingSpace,
    ) -> bool {
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
        let at = text.len();
        let spaced = std::mem::take(&mut space.0);
        if spaced {
            PendingSpace::push(text, self.first_char(id), opts);
        }
        let start = text.len();
        let handled = elements::write_element(self, text, e, id, opts);
        self.depth.set(depth);
        // an element that writes nothing leaves the space pending
        if text.len() == start {
            text.truncate(at);
            space.0 = spaced;
        }
        space.0 |= self.word_space.take();
        handled
    }

    /// Separates an opening element from what precedes it: a space in a link body,
    /// a line break before a block, `<br>` in a table cell.
    fn open_block(text: &mut String, name: &QualName, opts: FormatOpts, double_br: &str) {
        let linebreak = linebreak(opts.table_cell);
        if opts.link_body {
            if is_link_block(name) {
                push_block_space(text);
            }
        } else if is_md_block(name) {
            if opts.table_cell {
                add_cell_block_break(text, linebreak);
            } else {
                // in an item, the list adds a blank line itself if it can't interrupt the text
                let tight = opts.list_item && is_list(name);
                let end = if tight { linebreak } else { double_br };
                add_linebreaks(text, linebreak, end);
            }
        } else if name.local == local_name!("li") {
            // an item outside a list starts a line, as its end does
            if opts.table_cell {
                add_cell_block_break(text, linebreak);
            } else {
                add_linebreaks(text, linebreak, linebreak);
            }
        }
    }

    /// The first character `id` writes: its first non-empty text, past skipped tags and empty
    /// elements; `None` at an image, input or `<br>`, or after a fixed number of steps.
    fn first_char(&self, id: NodeId) -> Option<char> {
        let mut cur = self.nodes[id.value()].first_child?;
        // a fixed bound keeps the walks linear in total, however long a space stays pending
        for _ in 0..32 {
            let node = &self.nodes[cur.value()];
            let child = match &node.data {
                NodeData::Text { contents } if !contents.is_empty() => {
                    return contents.chars().next();
                }
                NodeData::Element(e) if !self.is_skipped(&e.name) => {
                    if matches!(
                        e.name.local,
                        local_name!("img") | local_name!("input") | local_name!("br")
                    ) {
                        return None;
                    }
                    node.first_child
                }
                _ => None,
            };
            if let Some(child) = child {
                cur = child;
                continue;
            }
            // the next node in document order, without leaving `id`
            let mut n = node;
            cur = loop {
                if let Some(next) = n.next_sibling {
                    break next;
                }
                let parent = n.parent?;
                if parent == id {
                    return None;
                }
                n = &self.nodes[parent.value()];
            };
        }
        None
    }
}

/// A word boundary written as a space only before the next text that starts a word;
/// one still pending when a code write ends is dropped, otherwise the next write gets it.
pub(super) struct PendingSpace(bool);

impl PendingSpace {
    /// Marks a boundary at `name`: a block in code (md skipped), an option or svg elsewhere.
    fn mark_at(&mut self, name: &QualName, opts: FormatOpts) {
        self.0 |= if opts.skip_md {
            is_link_block(name)
        } else {
            is_spaced_inline(name)
        };
    }

    pub(super) fn flush(&mut self, text: &mut String, next: &str, opts: FormatOpts) {
        if next.is_empty() || !std::mem::take(&mut self.0) {
            return;
        }
        Self::push(text, next.chars().next(), opts);
    }

    /// No space before whitespace or closing punctuation, after an opening bracket or a `<br>`
    /// tag (code writes none, text escapes its `<`), or between CJK; `next` may be unknown.
    fn push(text: &mut String, next: Option<char>, opts: FormatOpts) {
        let closing = |c: char| {
            c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}')
        };
        let opening = |c: char| c.is_whitespace() || matches!(c, '(' | '[' | '{');
        let cjk = |c: char| matches!(c, '\u{3000}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}' | '\u{ff00}'..='\u{ffef}');
        let br = !opts.skip_md && ends_with_cell_break(text);
        if next.is_some_and(closing)
            || text.ends_with(opening)
            || br
            || (text.ends_with(cjk) && next.is_some_and(cjk))
        {
            return;
        }
        push_block_space(text);
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

        // code (md skipped) joins its blocks with a space; an option or svg is written apart
        let mut block_space = PendingSpace(self.word_space.take() || opts.space_due);
        let opts = FormatOpts {
            space_due: false,
            ..opts
        };
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
                            block_space.flush(text, contents, opts);
                            push_normalized_text(text, contents, opts);
                        }
                        NodeData::Element(e) => {
                            if self.is_skipped(&e.name) {
                                continue;
                            }

                            // code (md skipped) joins its blocks with a space instead of linebreaks
                            block_space.mark_at(&e.name, opts);
                            if !opts.skip_md {
                                Self::open_block(text, &e.name, opts, double_br);
                                // push md prefixes only when md mode is active
                                if !opts.link_body {
                                    if let Some(prefix) = md_prefix(&e.name) {
                                        text.push_str(prefix);
                                    }
                                }
                                // delegate serialization if the element has its own custom handler
                                if self.write_element(text, e, id, opts, &mut block_space) {
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
                    block_space.mark_at(name, opts);
                    if !text.ends_with(double_br) {
                        close_element(text, name, opts, &ops);
                    }
                }
            }
        }

        if !opts.include_node && !opts.link_body {
            trim_space(text);
        }
        self.word_space.set(!opts.skip_md && block_space.0);
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
