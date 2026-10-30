use html5ever::local_name;

use crate::node::child_nodes;
use crate::{Element, NodeId, NodeRef};

use super::super::block::{is_block_wrapper, is_list, is_md_block};
use super::super::constants::MAX_LIST_NUMBER;
use super::super::ctx::{FormatOpts, ListContext, ListKind};
use super::super::heading::{escape_closing_hashes, is_heading, md_prefix};
use super::super::host::MDHost;
use super::super::text_utils::{add_linebreaks, trim_trailing_space};
use super::table::add_cell_block_break;

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
            .any(|n| node_is_list_item_block(&n));

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
    trim_trailing_space(text);
    text.extend(std::iter::repeat_n(' ', ctx.list_indent_len()));
    text.push_str(&prefix);
    h.write(text, node_id, ctx.opts.list_item());
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
    // a nested `write` trims the buffer's start, which would shift `item_start`
    text.drain(..text.len() - text.trim_ascii_start().len());
    text.extend(std::iter::repeat_n(' ', ctx.list_indent_len()));
    text.push_str(&prefix);

    let item_start = text.len();
    // whether inline content is waiting for a block to be separated from
    let mut in_paragraph = false;
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
                in_paragraph = false;
                text.push_str(&block_indent);
                text.push_str("---");
                text.push_str(ctx.linebreak());
                continue;
            }
            let indented = block_requires_indent(&c);
            // a block after inline content starts its own line
            if !indented && in_paragraph {
                // a blank line keeps a prose block from merging into the text
                if ctx.opts.table_cell {
                    add_cell_block_break(text, ctx.linebreak());
                } else {
                    while !text.ends_with("\n\n") {
                        text.push_str(ctx.linebreak());
                    }
                }
            }
            if text.len() > item_start && (!indented || !in_paragraph) {
                text.push_str(&block_indent);
            }

            if indented {
                write_list_item_indented(h, text, &c, &block_indent, item_start, ctx);
                blank_line_due = true;
            } else {
                let heading = c.qual_name_ref().filter(|n| is_heading(n));
                if let Some(prefix) = heading.as_ref().and_then(|n| md_prefix(n)) {
                    text.push_str(prefix);
                }
                h.write(text, c.id, ctx.opts.list_item());
                // a heading ends at its line end; a blank line would make the list loose
                if heading.is_some() {
                    escape_closing_hashes(text);
                } else {
                    text.push_str(ctx.linebreak());
                }
            }
            in_paragraph = false;
            text.push_str(ctx.linebreak());
        } else {
            // text after a block starts a line; unindented it would leave the item
            if text.len() > item_start && text.ends_with('\n') && !is_blank_text {
                text.push_str(&block_indent);
            }
            let start = text.len();
            // inline text can start a line, so it is escaped as in a plain item
            h.write(text, c.id, ctx.opts.list_item().include_node());
            if text.len() > start && !is_blank_text {
                // a nested list is a block, not paragraph text
                in_paragraph = !node_is_list(&c);
            }
        }
    }
    if !text.ends_with('\n') {
        text.push_str(ctx.linebreak());
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
    // the block is indented below, so its nested indentation starts at zero
    let opts = ctx
        .opts
        .offset(ctx.opts.offset.saturating_sub(block_indent.len()));
    h.write(&mut block, block_node.id, opts.include_node());
    let block = block.trim_matches('\n');

    // after inline text, the block goes on its own line
    let at_line_start = text.len() == item_start
        || text
            .strip_suffix(block_indent)
            .is_some_and(|t| t.ends_with('\n'));
    if !at_line_start {
        // a trailing hard break is redundant before the block
        text.truncate(text.trim_end().len().max(item_start));
        // a prose block needs a blank line, or it merges into the preceding text
        if !block_can_interrupt_paragraph(block_node) && !ctx.opts.table_cell {
            text.push_str(ctx.linebreak());
        }
        text.push_str(ctx.linebreak());
        text.push_str(block_indent);
    }

    for (i, line) in block.split('\n').enumerate() {
        if i > 0 {
            text.push_str(ctx.linebreak());
            if !line.is_empty() {
                text.push_str(block_indent);
            }
        }
        text.push_str(line);
    }
}

fn node_is_md_block(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_md_block(&name))
}

/// A child of `<li>` that is written as its own block; nested lists are
/// handled by `write_list`.
fn node_is_list_item_block(node: &NodeRef) -> bool {
    !node_is_list(node) && (node_is_md_block(node) || node.has_name("pre"))
}

/// A `<div>`-like element holding only blocks or lists; its children are written
/// as the item's own blocks. Inline text, a table or a rule would be glued or flattened.
fn node_is_block_wrapper(node: &NodeRef) -> bool {
    node.qual_name_ref()
        .is_some_and(|name| is_block_wrapper(&name))
        && node
            .children_it(false)
            .any(|n| node_is_list_item_block(&n) || node_is_list(&n))
        && node.children_it(false).all(|n| {
            node_is_list(&n)
                || (node_is_list_item_block(&n) && !n.has_name("table") && !n.has_name("hr"))
                || n.is_comment()
                || (n.is_text() && n.text().trim_ascii().is_empty())
        })
}

fn node_is_list(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_list(&name))
}

/// A fence or a block quote can interrupt a paragraph, so it needs no blank line;
/// the other indented blocks are plain paragraphs and do.
fn block_can_interrupt_paragraph(node: &NodeRef) -> bool {
    node.has_name("pre") || node.has_name("blockquote")
}

fn block_requires_indent(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| {
        matches!(
            name.local,
            local_name!("pre")
                | local_name!("blockquote")
                | local_name!("dl")
                | local_name!("figure")
        )
    })
}

/// Whether a list can start right after a line of text: its first item must
/// have text, and an ordered list must start at 1 (`CommonMark` §5.2).
pub(in crate::serializing::md) fn list_can_interrupt_paragraph<H: MDHost>(
    h: &H,
    node: &NodeRef,
) -> bool {
    let Some(ref first_li) = node.first_element_child().filter(|n| n.has_name("li")) else {
        return false;
    };
    let has_text = first_li
        .children_it(false)
        .take_while(|c| !node_is_list(c))
        .any(|c| has_visible_text(h, &c));

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

/// Whether the subtree renders text; skipped elements and nested lists render none.
fn has_visible_text<H: MDHost>(h: &H, node: &NodeRef) -> bool {
    if node.is_comment() || node_is_list(node) {
        return false;
    }
    if let Some(name) = node.qual_name_ref() {
        return !h.is_skipped(&name) && node.children_it(false).any(|c| has_visible_text(h, &c));
    }
    !node.text().trim().is_empty()
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
