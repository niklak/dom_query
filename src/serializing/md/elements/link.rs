use html5ever::local_name;

use crate::{Element, NodeId, NodeRef};

use super::super::block::is_md_block;
use super::super::ctx::FormatOpts;
use super::super::heading::is_heading;
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

    let Some(href) = el.attr("href") else {
        // no href: fallback to serializing node contents
        h.write(text, id, default_opts);
        return;
    };

    let mut link_text = String::new();
    h.write(&mut link_text, id, default_opts.link_body());

    // a link with only whitespace to show still separates the words around it
    if link_text.trim_ascii().is_empty() {
        if !link_text.is_empty() {
            push_normalized_text(text, &link_text, default_opts);
        }
        return;
    }

    // an unescaped trailing `!` would turn `[x](u)` into image syntax
    escape_trailing_bang(text);
    text.push('[');
    text.push_str(&link_text);
    text.push_str("](");
    push_md_url(text, &href);

    if let Some(title) = el.attr_ref(local_name!("title")) {
        push_title(text, title);
    }

    text.push(')');

    // text after the link must not join the paragraph its last block ended
    let node = NodeRef::new(id, h.tree());
    if ends_with_block(&node) {
        if opts.link_body || opts.inline || opts.list_item || in_heading(&node) {
            // no blank line fits here, as in link text
            push_block_space(text);
        } else {
            add_block_end(text, opts.table_cell);
        }
    }
}

fn ends_with_block(node: &NodeRef) -> bool {
    // a loop, not recursion: deep nesting must not overflow the stack
    let mut cur = *node;
    'descend: loop {
        for c in cur.children_it(true) {
            if c.is_element() {
                if node_is_block(&c) {
                    return true;
                }
                cur = c;
                continue 'descend;
            }
            if c.is_text() && !c.text().trim_ascii().is_empty() {
                return false;
            }
        }
        return false;
    }
}

/// Whether the nearest block around the link is a heading, which ends at its line end.
fn in_heading(node: &NodeRef) -> bool {
    node.ancestors_it(None)
        .find_map(|a| {
            a.qual_name_ref()
                .filter(|n| is_md_block(n))
                .map(|n| is_heading(&n))
        })
        .unwrap_or(false)
}
