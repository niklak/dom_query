use html5ever::local_name;

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
    opts: FormatOpts,
) {
    let default_opts = FormatOpts::new();

    let Some(href) = el.attr("href") else {
        // no href: the contents are written in place, like any inline element
        h.write_children(text, id, opts.include_node());
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
}
