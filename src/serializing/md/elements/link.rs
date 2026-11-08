use html5ever::local_name;

use crate::{Element, NodeId, NodeRef};

use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::serializer::add_block_end;
use super::super::text_utils::{
    escape_trailing_bang, push_block_space, push_md_url, push_normalized_text, push_title,
};
use super::list::node_is_block;

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    let default_opts = FormatOpts::new();

    let Some(href) = el.attr_ref(local_name!("href")) else {
        // no href: the contents are written in place, like any inline element
        h.write_children(text, id, opts.include_node());
        return;
    };

    let mut link_text = h.scratch();
    // the label keeps the cell rules, so a code span in it still escapes its pipes
    let mut label_opts = default_opts.link_body();
    label_opts.table_cell = opts.table_cell;
    h.write(&mut link_text, id, label_opts);

    // a link with only whitespace to show still separates the words around it
    if link_text.trim_ascii().is_empty() {
        if !link_text.is_empty() {
            push_normalized_text(text, &link_text, default_opts);
        }
        return;
    }

    // text before the link must not join the paragraph its first block starts
    let node = NodeRef::new(id, h.tree());
    if starts_with_block(&node) {
        push_block_sep(text, opts);
    }

    // an unescaped trailing `!` would turn `[x](u)` into image syntax
    escape_trailing_bang(text);
    text.push('[');
    text.push_str(&link_text);
    text.push_str("](");
    push_md_url(text, href);

    if let Some(title) = el.attr_ref(local_name!("title")) {
        push_title(text, title);
    }

    text.push(')');

    // text after the link must not join the paragraph its last block ended
    if ends_with_block(&node) {
        push_block_sep(text, opts);
    }
}

fn push_block_sep(text: &mut String, opts: FormatOpts) {
    if opts.link_body || opts.inline || opts.list_item || opts.heading {
        // no blank line fits here, as in link text
        push_block_space(text);
    } else {
        add_block_end(text, opts.table_cell);
    }
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
            if node_is_block(&c) {
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
