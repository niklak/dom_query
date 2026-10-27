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

    let mut is_md_body = false;
    // try to collect plain text if there is no image
    if !has_img {
        let link_opts = FormatOpts::new().include_node().skip_escape();
        h.write_text(&mut link_text, id, link_opts);
    }
    // a link with only whitespace to show still separates the words around it
    if !link_text.is_empty() && link_text.trim_ascii().is_empty() {
        push_normalized_text(text, &link_text, default_opts);
        return;
    }
    // serialize as markdown if body contains images or non-text elements
    if has_img || link_text.is_empty() {
        let body_opts = if has_img {
            default_opts.link_body()
        } else {
            default_opts
        };
        h.write(&mut link_text, id, body_opts);
        is_md_body = true;
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

fn has_descendant_img<H: MDHost>(h: &H, id: &NodeId) -> bool {
    let nodes = h.nodes();
    descendant_nodes(Ref::clone(&nodes), id).any(|child_id| {
        matches!(
            &nodes[child_id.value()].data,
            NodeData::Element(e) if e.name.local == local_name!("img")
        )
    })
}
