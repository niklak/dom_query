use crate::{Element, NodeId};

use super::super::ctx::FormatOpts;
use super::super::host::MDHost;

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    _el: &Element,
    id: NodeId,
    _opts: FormatOpts,
) {
    let opts = FormatOpts::new();
    let mut quote_buf = String::new();
    h.write(&mut quote_buf, id, opts);

    if quote_buf.is_empty() {
        return;
    }

    while !text.ends_with("\n\n") {
        text.push('\n');
    }

    for line in quote_buf.lines() {
        text.push_str("> ");
        text.push_str(line);
        text.push('\n');
    }

    text.push('\n');
}
