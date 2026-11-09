use html5ever::{LocalName, local_name};

use crate::node::child_nodes;
use crate::{Element, NodeId, NodeRef};

use super::super::block::{is_block_wrapper, is_list, is_md_block};
use super::super::constants::MAX_LIST_NUMBER;
use super::super::ctx::{FormatOpts, ListContext, ListKind};
use super::super::heading::{is_heading, md_prefix};
use super::super::host::MDHost;
use super::super::text_utils::{add_linebreaks, trim_trailing_space};
use super::heading::write_heading_line;
use super::image::image_src;
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
    // a list right after one of the same type takes the other marker, or the two
    // would merge
    let list = NodeRef::new(id, h.tree());
    let has_items = list.children_it(false).any(|c| c.has_name("li"));
    let use_alt_marker = !opts.table_cell && has_items && should_use_alt_marker(h, &el.name.local);
    ctx.alt_marker = use_alt_marker;
    let last_item = written_sibling(h, list.last_child(), NodeRef::prev_sibling)
        .filter(|_| !opts.table_cell)
        .map(|n| n.id);

    // whether stray content ends with a hard break
    let mut stray_break = false;
    for child_id in child_nodes(h.nodes(), &id, false) {
        let child_node = NodeRef::new(child_id, h.tree());
        let is_list_item = child_node.has_name("li");

        let has_blocks = child_node
            .children_it(false)
            .any(|n| writes_item_block(&n, opts.table_cell))
            || has_content_after_list(h, &child_node);

        if is_list_item && matches!(ctx.kind, ListKind::Ol(_)) {
            // `<li value>` renumbers this item and the ones after it
            if let Some(child_el) = child_node.element_ref() {
                if let Some(v) = child_el.attr("value").and_then(|v| parse_list_number(&v)) {
                    ctx.kind = ListKind::Ol(v);
                }
            }
        }

        // a lone hard break before the marker would let an empty item underline the
        // text; a second one already leaves the blank line
        if is_list_item && std::mem::take(&mut stray_break) {
            let end = text.len() - "  \n".len();
            if !text[..end].ends_with("  \n") {
                text.truncate(end);
            }
        }
        if is_list_item && has_blocks {
            let is_last = last_item == Some(child_id);
            write_list_item_blocks(h, text, child_id, &mut ctx, is_last);
        } else if is_list_item {
            write_list_item(h, text, child_id, &mut ctx);
        } else {
            // a stray child keeps the cell rules, like the link label does
            let mut child_opts = FormatOpts::new().include_node();
            child_opts.table_cell = opts.table_cell;
            h.write(text, child_id, child_opts);
            stray_break = text.ends_with("  \n");
        }
    }
    if has_items {
        h.set_last_list(id, use_alt_marker);
    }
    // without a blank line, text after the list would continue its last item
    if !opts.table_cell {
        let end = if opts.list_item && !is_continued(h, &list) {
            "\n"
        } else {
            "\n\n"
        };
        add_linebreaks(text, "\n", end);
    }
}

/// Whether the line after `list` would continue its last item without a blank
/// line; in an item, that line would otherwise make the parent list loose.
fn is_continued<H: MDHost>(h: &H, list: &NodeRef) -> bool {
    let Some(next) = written_sibling(h, list.next_sibling(), NodeRef::next_sibling) else {
        return !list.parent().is_some_and(|p| p.has_name("li"));
    };
    // a list marker or a block start ends the list; stray text in a list does not
    let starts_with_item = node_is_list(&next)
        && written_sibling(h, next.first_child(), NodeRef::next_sibling)
            .is_some_and(|c| c.has_name("li"));
    if starts_with_item || block_can_interrupt_paragraph(h, &next) {
        return false;
    }
    node_is_block(&next) || !ends_in_closed_block(h, list)
}

/// Whether `node` directly follows a list that ends in an item; a list's own
/// marker ends that item, while stray text in the list could take it as text.
pub(in crate::serializing::md) fn follows_list<H: MDHost>(h: &H, node: &NodeRef) -> bool {
    written_sibling(h, node.prev_sibling(), NodeRef::prev_sibling).is_some_and(|n| {
        node_is_list(&n)
            && written_sibling(h, n.last_child(), NodeRef::prev_sibling)
                .is_some_and(|li| li.has_name("li"))
    })
}

/// Whether the last item of `list` ends in a code block or heading, which
/// text after it cannot continue.
fn ends_in_closed_block<H: MDHost>(h: &H, list: &NodeRef) -> bool {
    let mut list = *list;
    loop {
        let last = written_sibling(h, list.last_child(), NodeRef::prev_sibling)
            .filter(|li| li.has_name("li"))
            .and_then(|li| written_sibling(h, li.last_child(), NodeRef::prev_sibling));
        match last {
            Some(n) if node_is_list(&n) => list = n,
            last => {
                return last.is_some_and(|n| {
                    n.has_name("pre") || n.qual_name_ref().is_some_and(|n| is_heading(&n))
                });
            }
        }
    }
}

fn write_list_item<H: MDHost>(h: &H, text: &mut String, node_id: NodeId, ctx: &mut ListContext) {
    let (item_start, block_indent) = write_item_marker(text, ctx);
    write_item_part(h, text, node_id, ctx.opts.list_item(), block_indent);
    pull_up_first_line(text, item_start - 1, block_indent);
    text.push_str(ctx.linebreak());
}

/// Writes an item's marker and returns where its content starts and the column
/// its continuation lines are indented to.
fn write_item_marker(text: &mut String, ctx: &mut ListContext) -> (usize, usize) {
    let prefix = ctx.prefix();
    // shift ol number after writing prefix
    ctx.advance_ol_number();
    let block_indent = ctx.list_indent_len() + prefix.len();
    trim_trailing_space(text);
    // a nested `write` trims the buffer's start, which would shift `item_start`
    text.drain(..text.len() - text.trim_ascii_start().len());
    // an indent at the start of an empty buffer would be trimmed the same way
    if !text.is_empty() {
        // text directly in the list would run into the marker; a blank line keeps an
        // empty first item from underlining it as a setext heading
        if !text.ends_with(ctx.linebreak()) {
            text.push_str(ctx.linebreak());
            if !ctx.started && !ctx.opts.table_cell {
                text.push_str(ctx.linebreak());
            }
        }
        text.extend(std::iter::repeat_n(' ', ctx.list_indent_len()));
    }
    ctx.started = true;
    text.push_str(&prefix);
    (text.len(), block_indent)
}

fn write_list_item_blocks<H: MDHost>(
    h: &H,
    text: &mut String,
    node_id: NodeId,
    ctx: &mut ListContext,
    is_last: bool,
) {
    let child_node = NodeRef::new(node_id, h.tree());
    let last_p = is_last
        .then(|| separate_last_paragraph(h, text, &child_node))
        .flatten();
    let (item_start, block_indent) = write_item_marker(text, ctx);
    let mut paragraph_end = None;
    // whether inline content is waiting for a block to be separated from
    let mut in_paragraph = false;
    // a block written indented is followed by a separating line unless the item
    // ends or a code block comes next; there it would only make the list loose
    let mut blank_line_due = false;
    let mut last_block = None;
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
        let is_blank_text = c.is_ascii_empty();
        if blank_line_due && !c.has_name("pre") && !is_blank_text {
            let line = last_block
                .take()
                .map_or(Some(""), |b| line_after_block(h, &b, &c));
            push_block_end(text, line, block_indent, ctx.linebreak());
            blank_line_due = false;
        }
        if node_is_list_item_block(&c) {
            if c.has_name("hr") {
                // a cell has no line for a rule; keep the words on either side apart
                if ctx.opts.table_cell {
                    // a cell has no line for a rule; keep the words on either side apart
                    if text.len() > item_start {
                        blank_line_due = true;
                    }
                    continue;
                }
                // own line: `- ---` would end the list; `***` can't make the text above
                // a heading, so a blank line is only kept next to a paragraph
                text.truncate(text.trim_ascii_end().len());
                let next = children.iter().rev().find(|n| !is_blank(n));
                let by_paragraph =
                    paragraph_end == Some(text.len()) || next.is_some_and(|n| n.has_name("p"));

                text.push_str(ctx.linebreak());
                if by_paragraph {
                    text.push_str(ctx.linebreak());
                }
                in_paragraph = false;
                text.extend(std::iter::repeat_n(' ', block_indent));
                text.push_str("***");
                text.push_str(ctx.linebreak());
                continue;
            }
            let indented = block_requires_indent(&c);
            // a block after inline content starts its own line
            if !indented && in_paragraph {
                if ctx.opts.table_cell {
                    add_cell_block_break(text, ctx.linebreak());
                } else if block_can_interrupt_paragraph(h, &c) {
                    // a heading or list interrupts the text; one line break is enough
                    if !text.ends_with('\n') {
                        text.push_str(ctx.linebreak());
                    }
                } else {
                    // a blank line keeps a prose block from merging into the text
                    push_blank_line(text);
                }
            }
            if last_p == Some(c.id) {
                push_blank_line(text);
            }
            if text.len() > item_start && (!indented || !in_paragraph) {
                text.extend(std::iter::repeat_n(' ', block_indent));
            }

            if indented {
                let end = text.trim_end().len();
                write_list_item_indented(h, text, &c, block_indent, item_start, ctx);
                blank_line_due = true;
                // a cell has no lines to end a block with; an empty block keeps the blank line
                last_block = (!ctx.opts.table_cell && text.trim_end().len() > end).then_some(c);
            } else {
                let heading = c.qual_name_ref().filter(|n| is_heading(n));
                if let Some(prefix) = heading.as_ref().and_then(|n| md_prefix(n)) {
                    text.push_str(prefix);
                    write_heading_line(h, text, c.id, ctx.opts);
                    // a heading ends at its line end; a blank line would make the list loose
                } else {
                    write_item_part(h, text, c.id, ctx.opts.list_item(), block_indent);
                    text.push_str(ctx.linebreak());
                    paragraph_end = Some(text.trim_ascii_end().len());
                }
            }
            in_paragraph = false;
            text.push_str(ctx.linebreak());
        } else {
            let start = text.trim_end_matches(' ').len();
            // inline text can start a line, so it is escaped as in a plain item
            let opts = ctx.opts.list_item().include_node();
            write_item_part(h, text, c.id, opts, block_indent);
            if text.len() > start && !is_blank_text {
                // a nested list is a block, not paragraph text
                in_paragraph = !node_is_list(&c);
            }
        }
    }
    pull_up_first_line(text, item_start - 1, block_indent);
    if !text.ends_with('\n') {
        text.push_str(ctx.linebreak());
    }
}

/// Writes a code block, block quote or table inside a list item. Each is written
/// from column zero, so every line after the first is indented to the
/// item's content column, or the block would end the list.
fn write_list_item_indented<H: MDHost>(
    h: &H,
    text: &mut String,
    block_node: &NodeRef,
    block_indent: usize,
    item_start: usize,
    ctx: &ListContext,
) {
    let mut buf = h.scratch();
    // the block is indented below, so its nested indentation starts at zero
    let opts = ctx
        .opts
        .offset(ctx.opts.offset.saturating_sub(block_indent));
    h.write(&mut buf, block_node.id, opts.include_node());
    let block = buf.trim_matches('\n');

    // after inline text, the block goes on its own line
    let at_line_start = text.len() == item_start
        || text
            .rfind('\n')
            .is_some_and(|pos| text.len() == pos + 1 + block_indent);
    // a table's rows would continue the text or list item above it
    let blank_before = block_node.has_name("table") && !text[item_start..].trim_end().is_empty();
    if !at_line_start || blank_before {
        // a trailing hard break is redundant before the block
        text.truncate(text.trim_end().len().max(item_start));
        // a prose block needs a blank line, or it merges into the preceding text
        if blank_before || (!ctx.opts.table_cell && !block_can_interrupt_paragraph(h, block_node)) {
            text.push_str(ctx.linebreak());
        }
        text.push_str(ctx.linebreak());
        text.extend(std::iter::repeat_n(' ', block_indent));
    }

    push_indented(text, block, ctx.linebreak(), block_indent, false);
}

/// The line ending `block`, written indented in a list item, before `next`: none
/// where `next` ends it, `>` before text a quote would take in, else a blank line.
fn line_after_block<H: MDHost>(h: &H, block: &NodeRef, next: &NodeRef) -> Option<&'static str> {
    let is_quote = block.has_name("blockquote");
    let is_pre = block.has_name("pre");
    if node_is_block(next) {
        let ends = (is_quote || is_pre || block.has_name("table"))
            && block_can_interrupt_paragraph(h, next)
            && !(is_quote && next.has_name("blockquote"));
        return (!ends).then_some("");
    }
    if is_pre {
        None
    } else if is_quote {
        Some(">")
    } else {
        Some("")
    }
}

/// Writes `line` after a block: `None` writes nothing, `""` a blank line.
fn push_block_end(text: &mut String, line: Option<&str>, indent: usize, linebreak: &str) {
    let Some(line) = line else { return };
    if !line.is_empty() {
        text.extend(std::iter::repeat_n(' ', indent));
        text.push_str(line);
    }
    text.push_str(linebreak);
}

/// Writes a node of a list item. A line it starts at column zero would leave the
/// item, so each is indented to `indent`; nested lists get an offset relative to it.
fn write_item_part<H: MDHost>(
    h: &H,
    text: &mut String,
    id: NodeId,
    opts: FormatOpts,
    indent: usize,
) {
    // a cell joins lines with `<br>`, so there is no line start to indent
    if opts.table_cell {
        h.write(text, id, opts);
        return;
    }
    let start = text.trim_end_matches(' ').len();
    // spaces at a line start are an indent, re-applied below
    if text[..start].ends_with('\n') {
        text.truncate(start);
    }
    // an emphasis run can join the one before it by dropping that run's closing
    // `**` before `start`; a `*` after whitespace is a list marker, not a closing run
    let closing = text[..start].trim_end_matches(['*', '~']);
    let kept = if closing.ends_with(|c: char| !c.is_whitespace()) {
        closing.len()
    } else {
        start
    };
    h.write(text, id, opts.offset(opts.offset.saturating_sub(indent)));
    let start = kept.min(text.len());
    let at_line_start = text[..start].ends_with('\n');
    // a single line not at a line start needs no indent, so skip the copy
    if !at_line_start && memchr::memchr(b'\n', &text.as_bytes()[start..]).is_none() {
        return;
    }
    let written = text.split_off(start);
    push_indented(text, &written, "\n", indent, at_line_start);
}

/// Appends `block`, joining its lines with `linebreak` and indenting each
/// non-empty one after the first (and the first if `indent_first`).
fn push_indented(
    text: &mut String,
    block: &str,
    linebreak: &str,
    indent: usize,
    indent_first: bool,
) {
    let mut rest = block;
    let mut first = true;
    loop {
        let (line, next) = match memchr::memchr(b'\n', rest.as_bytes()) {
            Some(pos) => (&rest[..pos], Some(&rest[pos + 1..])),
            None => (rest, None),
        };
        if !first {
            text.push_str(linebreak);
        }
        if (!first || indent_first) && !line.is_empty() {
            text.extend(std::iter::repeat_n(' ', indent));
        }
        text.push_str(line);
        let Some(next) = next else { break };
        rest = next;
        first = false;
    }
}

/// Moves an item's first content onto its marker line, which starts at
/// `marker_end`: a marker followed by a blank line ends the item, and `- ---`
/// would be a rule, not an item holding one.
fn pull_up_first_line(text: &mut String, marker_end: usize, indent: usize) {
    let Some(start) = text[marker_end..].find(|c: char| !c.is_ascii_whitespace()) else {
        return;
    };
    let Some(line_start) = text[marker_end..marker_end + start].rfind('\n') else {
        return;
    };
    let body = text.split_off(marker_end + line_start + 1);
    text.truncate(marker_end);
    let marker_line = text.rfind('\n').map_or(0, |pos| pos + 1);
    text.push(' ');
    let depth = |line: &str| line.len() - line.trim_start_matches(' ').len();
    // a nested list moved up starts at the content column, so the rest of it
    // moves left by as much to keep its items together
    let shift = depth(&body).saturating_sub(indent);
    let mut in_list = true;
    for (i, line) in body.split('\n').enumerate() {
        if i == 0 {
            // `- ---` would be a rule, not an item holding one
            let first = match line.trim_start() {
                "---" => "***",
                first => first,
            };
            if is_rule_like(&text[marker_line..], first) {
                text.pop();
                text.push('\n');
                text.extend(std::iter::repeat_n(' ', indent));
            }
            text.push_str(first);
            continue;
        }
        text.push('\n');
        in_list &= line.trim_ascii().is_empty() || depth(line) >= indent + shift;
        text.push_str(if in_list {
            &line[shift.min(depth(line))..]
        } else {
            line
        });
    }
}

/// Whether the marker line is three or more `-` or `*` and spaces, a rule.
fn is_rule_like(marker: &str, first: &str) -> bool {
    let line = || marker.bytes().chain(first.bytes()).filter(|&b| b != b' ');
    (line().all(|b| b == b'-') || line().all(|b| b == b'*')) && line().count() >= 3
}

/// Finds the `<p>` that ends the list's last `item`, writing the blank line before the
/// item if nothing precedes the `<p>`: the one after it is dropped at the list's end.
fn separate_last_paragraph<H: MDHost>(h: &H, text: &mut String, item: &NodeRef) -> Option<NodeId> {
    let p = written_sibling(h, item.last_child(), NodeRef::prev_sibling)
        .filter(|n| n.has_name("p") && has_visible_text(h, n))?;
    // with nothing before it in the item, the blank line goes before the marker
    let first = written_sibling(h, item.first_child(), NodeRef::next_sibling);
    if first.is_some_and(|n| n.id == p.id)
        && written_sibling(h, item.prev_sibling(), NodeRef::prev_sibling)
            .is_some_and(|n| n.has_name("li"))
    {
        push_blank_line(text);
    }
    Some(p.id)
}

fn push_blank_line(text: &mut String) {
    while !text.ends_with("\n\n") {
        text.push('\n');
    }
}

/// The first of `node` and the siblings `step` leads to that is written: not a
/// comment, blank text or skipped element.
fn written_sibling<'a, H: MDHost>(
    h: &H,
    node: Option<NodeRef<'a>>,
    step: fn(&NodeRef<'a>) -> Option<NodeRef<'a>>,
) -> Option<NodeRef<'a>> {
    std::iter::successors(node, step).find(|n| {
        !(n.is_comment()
            || n.is_ascii_empty()
            || n.qual_name_ref().is_some_and(|name| h.is_skipped(&name)))
    })
}

/// Whether a nested list in `item` has content both before and after it; the
/// plain item writer would put what follows at column zero, outside the item.
fn has_content_after_list<H: MDHost>(h: &H, item: &NodeRef) -> bool {
    let (mut before, mut after_list) = (false, false);
    for c in item.children_it(false) {
        if node_is_list(&c) {
            after_list = true;
        } else if has_visible_text(h, &c) {
            if after_list {
                return before;
            }
            before = true;
        }
    }
    false
}
/// Whether the last written list has the same tag, so this list needs the
/// alternate marker to avoid merging with it.
fn should_use_alt_marker<H: MDHost>(h: &H, name: &LocalName) -> bool {
    let Some((last_list_id, alt)) = h.last_list() else {
        return false;
    };
    let last_list = NodeRef::new(last_list_id, h.tree());
    alt && last_list
        .qual_name_ref()
        .is_some_and(|list_name| &list_name.local == name)
}

pub(in crate::serializing::md) fn write_task_marker<H: MDHost>(
    h: &H,
    text: &mut String,
    el: &Element,
    id: NodeId,
) {
    let is_checkbox = el
        .attr("type")
        .is_some_and(|t| t.eq_ignore_ascii_case("checkbox"));
    if is_checkbox && starts_list_item(h, NodeRef::new(id, h.tree())) {
        text.push_str(if el.has_attr("checked") {
            "[x] "
        } else {
            "[ ] "
        });
    }
}

/// Whether `node` is the first thing in its `<li>`, directly or through a
/// `<p>`, `<label>`, `<span>` or `<div>`, and inline content follows it.
fn starts_list_item<H: MDHost>(h: &H, mut node: NodeRef) -> bool {
    let mut has_content = false;
    loop {
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            if !is_blank(&p) {
                return false;
            }
            prev = p.prev_sibling();
        }
        let mut next = node.next_sibling();
        while !has_content {
            let Some(n) = next else { break };
            // a skipped element is dropped with its subtree, so it is not a boundary
            if n.qual_name_ref().is_some_and(|name| h.is_skipped(&name)) {
                next = n.next_sibling();
                continue;
            }
            // content after a block starts on another line; the marker would stand alone
            if n.has_name("pre") || node_is_md_block(&n) {
                return false;
            }
            has_content = has_visible_text(h, &n);
            next = n.next_sibling();
        }
        match node.parent() {
            Some(parent) if parent.has_name("li") => return has_content,
            Some(parent)
                if ["p", "label", "span", "div"]
                    .iter()
                    .any(|n| parent.has_name(n)) =>
            {
                node = parent;
            }
            _ => return false,
        }
    }
}

fn node_is_md_block(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_md_block(&name))
}

/// A child of `<li>` that is written as its own block; nested lists are
/// handled by `write_list`.
fn node_is_list_item_block(node: &NodeRef) -> bool {
    !node_is_list(node) && node_is_block(node)
}

/// Whether a child of `<li>` puts a block in the item: outside a cell, a wrapper only
/// if it holds a block or list, and an inline element if it holds a paragraph.
fn writes_item_block(node: &NodeRef, table_cell: bool) -> bool {
    if table_cell {
        node_is_list_item_block(node)
    } else if node.qual_name_ref().is_some_and(|n| is_block_wrapper(&n)) {
        holds(node, |n| {
            node_is_block(n) && !n.qual_name_ref().is_some_and(|n| is_block_wrapper(&n))
        })
    } else {
        node_is_list_item_block(node) || holds(node, |n| n.has_name("p"))
    }
}

/// Whether `node` or a descendant matches, outside lists and links, which write
/// their blocks as inline text.
fn holds(node: &NodeRef, matches: impl Fn(&NodeRef) -> bool) -> bool {
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        if matches(&n) {
            return true;
        }
        if !node_is_list(&n) && !n.has_name("a") {
            stack.extend(n.children_it(false));
        }
    }
    false
}

/// An element written as its own block outside link text.
pub(in crate::serializing::md) fn node_is_block(node: &NodeRef) -> bool {
    node_is_md_block(node) || node.has_name("pre")
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
                || is_blank(&n)
        })
}

fn node_is_list(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_list(&name))
}

/// A comment or a text node holding only whitespace: neither shows content.
fn is_blank(node: &NodeRef) -> bool {
    node.is_comment() || node.is_ascii_empty()
}

/// Whether `node` can start on the line right after a paragraph without a blank
/// line: a heading, rule, fence, quote or a list whose first item can interrupt.
fn block_can_interrupt_paragraph<H: MDHost>(h: &H, node: &NodeRef) -> bool {
    if node.has_name("pre")
        || node.has_name("blockquote")
        || node.has_name("hr")
        || node.qual_name_ref().is_some_and(|name| is_heading(&name))
    {
        return true;
    }
    if node_is_list(node) {
        return list_can_interrupt_paragraph(h, node);
    }
    // a wrapper is as tight as the block it starts with
    if node
        .qual_name_ref()
        .is_some_and(|name| is_block_wrapper(&name))
    {
        return node
            .children_it(false)
            .find(|c| c.is_element() || (c.is_text() && !c.is_ascii_empty()))
            .is_some_and(|c| block_can_interrupt_paragraph(h, &c));
    }
    false
}

fn block_requires_indent(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| {
        matches!(
            name.local,
            local_name!("pre")
                | local_name!("blockquote")
                | local_name!("dl")
                | local_name!("figure")
                | local_name!("table")
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
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        if n.is_comment() || node_is_list(&n) {
            continue;
        }
        let Some(name) = n.qual_name_ref() else {
            if n.is_nonempty_text() {
                return true;
            }
            continue;
        };
        if h.is_skipped(&name) {
            continue;
        }
        if n.has_name("img") {
            if n.element_ref().and_then(|el| image_src(&el)).is_some() {
                return true;
            }
            continue;
        }
        stack.extend(n.children_it(true));
    }
    false
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
    // 1..=9 digits always fit u32; longer values were clamped above
    meaningful.parse().ok()
}
