use crate::{Element, NodeId};

use super::super::block::{Block, BlockWriter, Separator};
use super::super::ctx::FormatOpts;
use super::super::heading::{escape_closing_hashes, heading_prefix};
use super::super::host::MDHost;

/// Writes a heading on one line, as an ATX heading ends at its line end.
pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    el: &Element,
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
    opts.heading = true;
    let buf = h.write_buf(id, opts);
    let mut content = buf.trim_ascii();
    // a trailing hard break adds nothing; an odd run of `\` makes `\<br>` literal text
    while let Some(rest) = content.strip_suffix("<br>") {
        if rest.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 1 {
            break;
        }
        content = rest.trim_ascii_end();
    }
    let mut line = h.take_buf();
    let prefix = heading_prefix(&el.name);
    // an empty heading's line ends at its marker
    line.push_str(if content.is_empty() {
        prefix.trim_end()
    } else {
        prefix
    });
    let start = line.len();
    line.push_str(content);
    escape_closing_hashes(&mut line, start);
    out.push_block(Block::Heading, &line);
    out.owe(Separator::BlankLine);
    h.put_buf(buf);
    h.put_buf(line);
}
