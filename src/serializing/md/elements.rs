use html5ever::local_name;

use crate::{Element, NodeId};

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
    text: &mut String,
    e: &Element,
    id: NodeId,
    opts: FormatOpts,
) -> bool {
    match e.name.local {
        // their markers cannot go in link text; their children are
        // written as plain blocks
        local_name!("ol") | local_name!("ul") if !opts.link_body => {
            list::write(h, text, e, id, opts);
        }
        local_name!("blockquote") if !opts.link_body => {
            quote::write(h, text, e, id, opts);
        }
        local_name!("table") if !opts.link_body => {
            table::write(h, text, e, id, opts);
        }
        local_name!("a") => link::write(h, text, e, id, opts),
        local_name!("img") => image::write(h, text, e, id, opts),
        local_name!("pre") | local_name!("code") => code::write(h, text, e, id, opts),
        local_name!("strong") | local_name!("b") | local_name!("em") | local_name!("i") => {
            emphasis::write(h, text, e, id, opts);
        }
        local_name!("h1")
        | local_name!("h2")
        | local_name!("h3")
        | local_name!("h4")
        | local_name!("h5")
        | local_name!("h6")
            if !opts.link_body =>
        {
            heading::write(h, text, e, id, opts);
        }
        _ => return false,
    }
    true
}
