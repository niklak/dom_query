use crate::{Element, NodeId};

use super::super::ctx::FormatOpts;
use super::super::heading::escape_closing_hashes;
use super::super::host::MDHost;
use super::super::text_utils::{add_linebreaks, double_linebreak, linebreak};
use super::table::add_cell_block_break;

/// Writes a heading's content and ends its block; the `#` prefix is already written.
pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    _el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    write_heading_line(h, text, id, opts);
    escape_closing_hashes(text);
    let linebreak = linebreak(opts.table_cell);
    if opts.table_cell {
        add_cell_block_break(text, linebreak);
    } else {
        add_linebreaks(text, linebreak, double_linebreak(false));
    }
}

/// Writes a heading's content on one line: an ATX heading ends at the line end.
pub(in crate::serializing::md) fn write_heading_line<H: MDHost>(
    h: &H,
    text: &mut String,
    id: NodeId,
    opts: FormatOpts,
) {
    // a table cell already joins blocks with `<br>`
    let mut opts = if opts.table_cell {
        opts
    } else {
        opts.link_body()
    };
    opts.include_node = false;
    // a separate buffer keeps a leading `<br>` from trimming the space after the `#`s
    let mut content = String::new();
    h.write(&mut content, id, opts);
    let mut content = content.trim_ascii();
    // a trailing hard break adds nothing; an odd run of `\` makes `\<br>` literal text
    while let Some(rest) = content.strip_suffix("<br>") {
        if rest.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 1 {
            break;
        }
        content = rest.trim_ascii_end();
    }
    text.push_str(content);
}
