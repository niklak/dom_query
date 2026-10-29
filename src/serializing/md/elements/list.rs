use html5ever::{QualName, local_name};

use crate::node::child_nodes;
use crate::{Element, NodeId, NodeRef};

use super::super::block::{is_list, is_md_block};
use super::super::constants::MAX_LIST_NUMBER;
use super::super::ctx::{FormatOpts, ListContext, ListKind};
use super::super::heading::{escape_closing_hashes, is_heading, md_prefix};
use super::super::host::MDHost;
use super::super::text_utils::{add_linebreaks, trim_trailing_space};

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    let kind = if el.name.local == local_name!("ol") {
        let start = el
            .attr("start")
            .and_then(|v| parse_list_number(&v))
            .unwrap_or(1);
        ListKind::Ol(start)
    } else {
        ListKind::Ul
    };

    let mut ctx = ListContext::new(opts, kind);

    for child_id in child_nodes(h.nodes(), &id, false) {
        let child_node = NodeRef::new(child_id, h.tree());

        let is_list_item = child_node.query_or(false, |t| {
            t.as_element()
                .is_some_and(|e| e.name.local == local_name!("li"))
        });

        let has_blocks = child_node
            .children_it(false)
            .any(|n| node_is_list_item_block(&n))
            || has_content_after_list(h, &child_node);

        if is_list_item && matches!(ctx.kind, ListKind::Ol(_)) {
            // `<li value>` renumbers this item and the ones after it
            if let Some(child_el) = child_node.element_ref() {
                if let Some(v) = child_el.attr("value").and_then(|v| parse_list_number(&v)) {
                    ctx.kind = ListKind::Ol(v);
                }
            }
        }

        if is_list_item && has_blocks {
            write_list_item_blocks(h, text, child_id, &mut ctx);
        } else if is_list_item {
            write_list_item(h, text, child_id, &mut ctx);
        } else {
            h.write(text, child_id, FormatOpts::new().include_node());
        }
    }
    // without a blank line, text after the list would continue its last item
    if !opts.table_cell {
        add_linebreaks(text, "\n", "\n\n");
    }
}

fn write_list_item<H: MDHost>(h: &H, text: &mut String, node_id: NodeId, ctx: &mut ListContext) {
    let prefix = ctx.prefix();
    // shift ol number after writing prefix
    ctx.advance_ol_number();
    let block_indent = " ".repeat(ctx.list_indent_len() + prefix.len());
    trim_trailing_space(text);
    text.extend(std::iter::repeat_n(' ', ctx.list_indent_len()));
    text.push_str(&prefix);
    // a nested `write` trims the buffer's start, which would shift `marker_end`
    text.drain(..text.len() - text.trim_ascii_start().len());
    let marker_end = text.len() - 1;
    write_item_part(h, text, node_id, ctx.opts.list_item(), &block_indent);
    pull_up_first_line(text, marker_end, &block_indent);
    text.push_str(ctx.linebreak());
}

fn write_list_item_blocks<H: MDHost>(
    h: &H,
    text: &mut String,
    node_id: NodeId,
    ctx: &mut ListContext,
) {
    let child_node = NodeRef::new(node_id, h.tree());
    let prefix = ctx.prefix();
    ctx.advance_ol_number();
    let block_indent = " ".repeat(ctx.list_indent_len() + prefix.len());
    trim_trailing_space(text);
    text.extend(std::iter::repeat_n(' ', ctx.list_indent_len()));
    text.push_str(&prefix);
    // a nested `write` trims the buffer's start, which would shift `item_start`
    text.drain(..text.len() - text.trim_ascii_start().len());

    let item_start = text.len();
    let mut is_first_block = true;
    // a code block or quote is followed by a blank line unless the item ends
    // or a code block comes next; there it would only make the list loose
    let mut blank_line_due = false;
    // reversed, so `pop` yields children in document order
    let mut children: Vec<_> = child_node.children_it(true).collect();
    while let Some(c) = children.pop() {
        if c.is_comment() {
            continue;
        }
        // block children are written without their own node, so skip them here
        if c.qual_name_ref().is_some_and(|n| h.is_skipped(&n)) {
            continue;
        }
        if node_is_block_wrapper(&c) {
            children.extend(c.children_it(true));
            continue;
        }
        let is_blank_text = c.is_text() && c.text().trim_ascii().is_empty();
        if blank_line_due && !c.has_name("pre") && !is_blank_text {
            text.push_str(ctx.linebreak());
            blank_line_due = false;
        }
        if node_is_list_item_block(&c) {
            if c.has_name("hr") {
                if ctx.opts.table_cell {
                    // a cell has no line for a rule; keep the words on either side apart
                    if text.len() > item_start {
                        blank_line_due = true;
                    }
                    continue;
                }
                // own line: `- ---` would end the list, `---` under text makes a heading
                text.truncate(text.trim_ascii_end().len());
                let has_content = text.len() > item_start;
                text.push_str(ctx.linebreak());
                if has_content {
                    text.push_str(ctx.linebreak());
                }
                is_first_block = false;
                text.push_str(&block_indent);
                text.push_str("---");
                text.push_str(ctx.linebreak());
                continue;
            }
            let indented = c.has_name("pre") || c.has_name("blockquote");
            // a block after inline content starts its own indented line, after a
            // blank line unless it can interrupt that text
            if !indented && text.len() > item_start && !text.ends_with('\n') {
                text.push_str(ctx.linebreak());
                // a cell joins blocks with `<br>` and has no paragraphs to keep apart
                if starts_paragraph(&c) && !ctx.opts.table_cell {
                    text.push_str(ctx.linebreak());
                }
            }
            if !is_first_block || (!indented && text.len() > item_start) {
                text.push_str(&block_indent);
            }
            is_first_block = false;

            if indented {
                write_list_item_indented(h, text, &c, &block_indent, item_start, ctx);
                blank_line_due = true;
            } else {
                let heading = c.qual_name_ref().filter(|n| is_heading(n));
                if let Some(prefix) = heading.as_ref().and_then(|n| md_prefix(n)) {
                    text.push_str(prefix);
                }
                write_item_part(h, text, c.id, ctx.opts.list_item(), &block_indent);
                // a heading ends at its line end; a blank line would make the list loose
                if heading.is_some() {
                    escape_closing_hashes(text);
                } else {
                    text.push_str(ctx.linebreak());
                }
            }
            text.push_str(ctx.linebreak());
        } else {
            // inline text can start a line, so it is escaped as in a plain item
            let opts = if node_is_list(&c) {
                ctx.opts
            } else {
                ctx.opts.list_item()
            };
            write_item_part(h, text, c.id, opts.include_node(), &block_indent);
        }
    }
    pull_up_first_line(text, item_start - 1, &block_indent);
    if !text.ends_with('\n') {
        text.push_str(ctx.linebreak());
    }
}

/// Moves an item's first line of content onto its marker line, which starts at
/// `marker_end`: a marker followed by a blank line ends the item, and a bare
/// marker after text continues that paragraph.
fn pull_up_first_line(text: &mut String, marker_end: usize, indent: &str) {
    let Some(start) = text[marker_end..].find(|c: char| !c.is_ascii_whitespace()) else {
        return;
    };
    let Some(line_start) = text[marker_end..marker_end + start].rfind('\n') else {
        return;
    };
    let body = text.split_off(marker_end + line_start + 1);
    text.truncate(marker_end);
    text.push(' ');
    let depth = |line: &str| line.len() - line.trim_start_matches(' ').len();
    // a nested list moved up starts at the content column, so the rest of it
    // moves left by as much to keep its items together; with no shift every
    // line stays as it is
    let shift = depth(&body).saturating_sub(indent.len());
    let mut in_list = true;
    for (i, line) in body.split('\n').enumerate() {
        if i == 0 {
            // `- ---` would be a rule, not an item holding one
            text.push_str(match line.trim_start() {
                "---" => "***",
                first => first,
            });
            continue;
        }
        text.push('\n');
        in_list &= line.trim_ascii().is_empty() || depth(line) >= indent.len() + shift;
        text.push_str(if in_list {
            &line[shift.min(depth(line))..]
        } else {
            line
        });
    }
}

/// Writes a code block or block quote inside a list item. Both are written
/// from column zero, so every line after the first is indented to the
/// item's content column, or the block would end the list.
fn write_list_item_indented<H: MDHost>(
    h: &H,
    text: &mut String,
    block_node: &NodeRef,
    block_indent: &str,
    item_start: usize,
    ctx: &ListContext,
) {
    let mut block = String::new();
    h.write(&mut block, block_node.id, ctx.opts.include_node());
    let block = block.trim_matches('\n');

    // after inline text, the block goes on its own line; a fence or `>`
    // can interrupt a paragraph, so no blank line is needed and the list
    // stays tight
    let at_line_start = text.len() == item_start
        || text
            .strip_suffix(block_indent)
            .is_some_and(|t| t.ends_with('\n'));
    if !at_line_start {
        // a trailing hard break is redundant before the block
        text.truncate(text.trim_end().len().max(item_start));
        text.push_str(ctx.linebreak());
        text.push_str(block_indent);
    }

    push_indented(text, block, ctx.linebreak(), block_indent, false);
}

/// Writes a node of a list item. A line it starts at column zero would leave the
/// item, so each is indented to `indent`; nested lists get an offset relative to it.
fn write_item_part<H: MDHost>(
    h: &H,
    text: &mut String,
    id: NodeId,
    opts: FormatOpts,
    indent: &str,
) {
    // a cell joins lines with `<br>`, so there is no line start to indent
    if opts.table_cell {
        h.write(text, id, opts);
        return;
    }
    let mut start = text.trim_end_matches(' ').len();
    // spaces at a line start are an indent, re-applied below
    if text[..start].ends_with('\n') {
        text.truncate(start);
    }
    h.write(
        text,
        id,
        opts.offset(opts.offset.saturating_sub(indent.len())),
    );
    start = start.min(text.len());
    let at_line_start = text[..start].ends_with('\n');
    let written = text.split_off(start);
    push_indented(text, &written, "\n", indent, at_line_start);
}

/// Appends `block`, joining its lines with `linebreak` and indenting each
/// non-empty one after the first (and the first if `indent_first`).
fn push_indented(
    text: &mut String,
    block: &str,
    linebreak: &str,
    indent: &str,
    indent_first: bool,
) {
    for (i, line) in block.split('\n').enumerate() {
        if i > 0 {
            text.push_str(linebreak);
        }
        if (i > 0 || indent_first) && !line.is_empty() {
            text.push_str(indent);
        }
        text.push_str(line);
    }
}

/// Whether a nested list in `item` has content both before and after it; the
/// plain item writer would put what follows at column zero, outside the item.
fn has_content_after_list<H: MDHost>(h: &H, item: &NodeRef) -> bool {
    let is_content = |c: &NodeRef| {
        let skipped = c.qual_name_ref().is_some_and(|n| h.is_skipped(&n));
        (c.is_text() || (c.is_element() && !skipped))
            && (c.has_name("img") || !c.text().trim_ascii().is_empty())
    };
    let (mut before, mut after_list) = (false, false);
    for c in item.children_it(false) {
        if node_is_list(&c) {
            after_list = true;
        } else if is_content(&c) {
            if after_list {
                return before;
            }
            before = true;
        }
    }
    false
}

/// Whether `block` starts with paragraph text, which would continue a paragraph
/// right before it; a heading or a list can interrupt one.
fn starts_paragraph(block: &NodeRef) -> bool {
    if node_is_list(block) {
        return !can_interrupt_paragraph(block);
    }
    if block.qual_name_ref().is_some_and(|n| is_heading(&n)) {
        return false;
    }
    if !node_is_md_block(block) {
        return true;
    }
    block
        .children_it(false)
        .find(|c| c.is_element() || (c.is_text() && !c.text().trim_ascii().is_empty()))
        .is_none_or(|c| starts_paragraph(&c))
}

fn node_is_md_block(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_md_block(&name))
}

/// A child of `<li>` that is written as its own block; nested lists are
/// handled by `write_list`.
fn node_is_list_item_block(node: &NodeRef) -> bool {
    !node_is_list(node) && (node_is_md_block(node) || node.has_name("pre"))
}

/// A `<div>`-like element holding only blocks; its children are written as the
/// item's own blocks. Inline text, a table or a rule would be glued or flattened.
fn node_is_block_wrapper(node: &NodeRef) -> bool {
    node.qual_name_ref()
        .is_some_and(|name| is_block_wrapper(&name))
        && node.children_it(false).any(|n| node_is_list_item_block(&n))
        && node.children_it(false).all(|n| {
            node_is_list(&n)
                || (node_is_list_item_block(&n) && !n.has_name("table") && !n.has_name("hr"))
                || n.is_comment()
                || (n.is_text() && n.text().trim_ascii().is_empty())
        })
}

const fn is_block_wrapper(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("div") | local_name!("section") | local_name!("article")
    )
}

fn node_is_list(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_list(&name))
}

/// Whether a list can start right after a line of text: its first item must
/// have text, and an ordered list must start at 1 (`CommonMark` §5.2).
pub(in crate::serializing::md) fn can_interrupt_paragraph(node: &NodeRef) -> bool {
    let Some(ref first_li) = node.first_element_child().filter(|n| n.has_name("li")) else {
        return false;
    };
    let has_text = first_li
        .children_it(false)
        .take_while(|c| !node_is_list(c))
        .any(|c| !c.text().trim().is_empty());

    if !has_text {
        return false;
    }
    if !node.has_name("ol") {
        return true;
    }
    let first_number = first_li
        .attr("value")
        .and_then(|v| parse_list_number(&v))
        .or_else(|| node.attr("start").and_then(|v| parse_list_number(&v)))
        .unwrap_or(1);
    first_number == 1
}

/// Parses an `ol start` or `li value` attribute the way HTML does: leading
/// whitespace, an optional sign, then digits, ignoring anything after them
/// (`" 5abc"` is 5). A Markdown list number cannot be negative, so negative
/// values become 0, and values above the nine-digit limit are clamped.
fn parse_list_number(value: &str) -> Option<u32> {
    // Trim leading ASCII whitespace according to WHATWG HTML standard
    let value = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    // Parse an optional sign prefix
    let (negative, value) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    // Extract the contiguous sequence of ASCII digits
    let digits_len = value.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 {
        return None;
    }
    // md doesn't support negative list markers, clamp to zero
    if negative {
        return Some(0);
    }

    // strip leading zeros from the digit slice
    let meaningful = value[..digits_len].trim_start_matches('0');
    // if string becomes empty, the input was "0" or multiple zeros ("000")
    if meaningful.is_empty() {
        return Some(0);
    }
    // fast-path: if significant digits exceed 9, clamp to the CommonMark limit
    if meaningful.len() > 9 {
        return Some(MAX_LIST_NUMBER);
    }
    // safe to unwrap since it contains 1..=9 digits and fits in u32 without overflow
    let n = meaningful.parse::<u32>().unwrap_or(u32::MAX);
    Some(n.min(MAX_LIST_NUMBER))
}
