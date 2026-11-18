use html5ever::local_name;

use crate::{Element, NodeId};

use super::block::BlockWriter;
use super::ctx::FormatOpts;
use super::host::MDHost;

pub(super) mod code;
pub(super) mod emphasis;
pub(super) mod heading;
pub(super) mod image;
pub(super) mod link;
pub(super) mod list;
pub(super) mod quote;
pub(super) mod table;

pub(super) fn write_element<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    e: &Element,
    id: NodeId,
    opts: FormatOpts,
) -> bool {
    match e.name.local {
        // their markers cannot go in link text; their children are
        // written as plain blocks
        local_name!("ol") | local_name!("ul") if !opts.link_body => {
            list::write(h, out, e, id, opts);
        }
        // a cell holds no `>` lines: its quote is written as a plain block
        local_name!("blockquote") if !opts.link_body && !opts.table_cell => {
            quote::write(h, out, id);
        }
        local_name!("table") if !opts.link_body => {
            table::write(h, out, id, opts);
        }
        local_name!("input") if opts.list_item && !opts.link_body && !opts.table_cell => {
            list::write_task_marker(h, out, e, id);
        }
        // no href: the contents are written in place, like any inline element
        local_name!("a") => match e.attr_ref(local_name!("href")) {
            Some(href) => link::write(h, out, e, id, href, opts),
            None => return false,
        },
        local_name!("img") => image::write(h, out, e, id, opts),
        local_name!("pre") | local_name!("code") => code::write(h, out, e, id, opts),
        local_name!("h1")
        | local_name!("h2")
        | local_name!("h3")
        | local_name!("h4")
        | local_name!("h5")
        | local_name!("h6")
            if !opts.link_body =>
        {
            heading::write(h, out, e, id, opts);
        }
        _ => match emphasis::Emphasis::of(&e.name) {
            Some(kind) => emphasis::write(h, out, kind, id, opts),
            None => return false,
        },
    }
    true
}
