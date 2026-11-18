use html5ever::local_name;

use crate::{Element, NodeId, NodeRef};

use super::super::block::{BlockWriter, Separator, is_md_block};
use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::inline::{Fragment, Inline};

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    el: &Element,
    id: NodeId,
    href: &str,
    opts: FormatOpts,
) {
    // the label keeps the cell rules, so a code span in it still escapes its pipes
    let mut label_opts = FormatOpts::new().link_body();
    label_opts.table_cell = opts.table_cell;
    let mut writer = BlockWriter::fragment(h.take_buf(), label_opts);
    h.write(&mut writer, id, label_opts);
    writer.finish_fragment();
    let label = writer.content();

    // a link with only whitespace to show still separates the words around it
    if label.text.trim_ascii().is_empty() {
        // a block in it writes nothing but still owes its space
        out.owe(writer.owed().min(Separator::Space));
        out.push(Inline::Text(label.text, FormatOpts::new()));
        // with no text, this only passes on its word boundaries
        let label = Fragment { text: "", ..label };
        out.push(Inline::Link {
            label,
            href,
            title: None,
        });
        return h.put_buf(writer.into_buf());
    }

    // text before the link must not join the paragraph its first block starts
    let node = NodeRef::new(id, h.tree());
    let sep = if opts.inline || opts.list_item || opts.heading {
        // no blank line fits here, as in link text
        Separator::Space
    } else {
        Separator::BlankLine
    };
    if starts_with_block(&node) {
        out.owe(sep);
    }

    let title = el.attr_ref(local_name!("title"));
    out.push(Inline::Link { label, href, title });
    // a space the label still owes, as after a code span ending in whitespace
    out.owe(writer.owed().min(Separator::Space));
    h.put_buf(writer.into_buf());

    // text after the link must not join the paragraph its last block ended
    if ends_with_block(&node) {
        out.owe(sep);
    }
}

/// An element written as its own block outside link text.
fn node_is_block(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_md_block(&name)) || node.has_name("pre")
}

fn starts_with_block(node: &NodeRef) -> bool {
    edge_is_block(node, false)
}

fn ends_with_block(node: &NodeRef) -> bool {
    edge_is_block(node, true)
}

fn edge_is_block<'a>(node: &NodeRef<'a>, end: bool) -> bool {
    let child = |n: &NodeRef<'a>| if end { n.last_child() } else { n.first_child() };
    let sibling = |n: &NodeRef<'a>| {
        if end {
            n.prev_sibling()
        } else {
            n.next_sibling()
        }
    };
    // a loop, not recursion: deep nesting must not overflow the stack;
    // an element that holds nothing, such as an empty `<span>`, is passed over
    let mut cur = child(node);
    while let Some(c) = cur {
        if c.is_element() {
            // a list item outside a list is still a block
            if node_is_block(&c) || c.has_name("li") {
                return true;
            }
            if let Some(first) = child(&c) {
                cur = Some(first);
                continue;
            }
            // an image and a break write content of their own, so they end the walk
            if c.has_name("img") || c.has_name("br") {
                return false;
            }
        } else if c.is_text() && !c.is_ascii_empty() {
            return false;
        }
        let mut n = c;
        cur = loop {
            if let Some(s) = sibling(&n) {
                break Some(s);
            }
            match n.parent() {
                Some(p) if p.id != node.id => n = p,
                _ => break None,
            }
        };
    }
    false
}
