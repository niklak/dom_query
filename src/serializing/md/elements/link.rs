use std::cell::Ref;

use html5ever::local_name;

use crate::node::{NodeData, descendant_nodes};
use crate::{Element, NodeId};

use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::text_utils::{
    escape_trailing_bang, push_md_url, push_normalized_text, push_title,
};

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    el: &Element,
    id: NodeId,
    _opts: FormatOpts,
) {
    let default_opts = FormatOpts::new();

    let Some(href) = el.attr("href") else {
        // no href: fallback to serializing node contents
        h.write(text, id, default_opts);
        return;
    };

    let mut link_text = String::new();
    let has_img = has_descendant_img(h, &id);
    let has_inline = has_descendant_inline(h, &id);
    let has_md = has_img || has_inline;
    // an inline-formatted label keeps the plain label's whitespace-only content
    // and trailing space, which the Markdown body writer drops
    let keep_space = has_inline && !has_img;

    let mut is_md_body = false;
    if !has_md || keep_space {
        let link_opts = FormatOpts::new().include_node().skip_escape();
        h.write_text(&mut link_text, id, link_opts);
    }
    // a link with only whitespace to show still separates the words around it
    if !link_text.is_empty() && link_text.trim_ascii().is_empty() {
        push_normalized_text(text, &link_text, default_opts);
        return;
    }
    // serialize as markdown if body contains an image, inline formatting or non-text elements
    if has_md || link_text.is_empty() {
        let trailing_space = keep_space && link_text.ends_with(|c: char| c.is_ascii_whitespace());
        let body_opts = if has_md {
            default_opts.link_body()
        } else {
            default_opts
        };
        link_text.clear();
        h.write(&mut link_text, id, body_opts);
        is_md_body = true;
        if trailing_space && !link_text.is_empty() {
            link_text.push(' ');
        }
    }

    if link_text.is_empty() {
        return;
    }

    // an unescaped trailing `!` would turn `[x](u)` into image syntax
    escape_trailing_bang(text);
    text.push('[');
    if is_md_body {
        text.push_str(&link_text);
    } else {
        push_normalized_text(text, &link_text, default_opts);
    }

    text.push_str("](");
    push_md_url(text, &href);

    if let Some(title) = el.attr_ref(local_name!("title")) {
        push_title(text, title);
    }

    text.push(')');
}

/// Whether the link body holds an image, which must be nested inside the link.
fn has_descendant_img<H: MDHost>(h: &H, id: &NodeId) -> bool {
    let nodes = h.nodes();
    descendant_nodes(Ref::clone(&nodes), id).any(|child_id| {
        matches!(
            &nodes[child_id.value()].data,
            NodeData::Element(e) if e.name.local == local_name!("img")
        )
    })
}

/// Whether the link body holds an inline element with Markdown formatting of its own.
fn has_descendant_inline<H: MDHost>(h: &H, id: &NodeId) -> bool {
    let nodes = h.nodes();
    descendant_nodes(Ref::clone(&nodes), id).any(|child_id| {
        matches!(
            &nodes[child_id.value()].data,
            NodeData::Element(e) if matches!(
                e.name.local,
                local_name!("code")
                    | local_name!("b")
                    | local_name!("strong")
                    | local_name!("i")
                    | local_name!("em")
            )
        )
    })
}
