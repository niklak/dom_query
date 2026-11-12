use html5ever::{LocalName, local_name};
use memchr::{memchr_iter, memrchr_iter};

use crate::{Element, NodeId, NodeRef};

use super::super::block::{is_block_wrapper, is_list, is_md_block};
use super::super::constants::{LIST_OFFSET_BASE, MAX_LIST_NUMBER};
use super::super::ctx::{FormatOpts, ListContext, ListKind};
use super::super::host::MDHost;
use super::super::serializer::add_block_end;
use super::super::text_utils::{add_linebreaks, linebreak, trim_trailing_space};
use super::image::image_src;
use super::table::ends_with_cell_break;

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
    let lb = linebreak(opts.table_cell);

    let start = text.len();
    // how the item before ended; `None` before the first item or after stray content
    let mut prev_item = None;
    // whether the last stray child that wrote anything ends in an `<option>`, and, once
    // stray content shows after the last item, whether it continues that item as a lazy line
    let (mut stray_space_due, mut lazy_stray) = (false, None);
    for child in list.children_it(false) {
        if !child.has_name("li") {
            let prev = prev_item.take();
            if write_stray(h, text, &child, prev, &mut stray_space_due, opts.table_cell) {
                let lazy = matches!(prev, Some(End::Text | End::Quote | End::Lazy));
                lazy_stray = lazy_stray.or(Some(lazy));
            } else {
                prev_item = prev;
            }
            continue;
        }
        // a marker after a lazy line starts the next item of the list above
        let lazy = lazy_stray.take().is_some_and(|lazy| {
            end_stray_line(text);
            lazy
        });
        stray_space_due = false;
        if matches!(ctx.kind, ListKind::Ol(_)) {
            // `<li value>` renumbers this item and the ones after it
            if let Some(v) = list_number(&child, "value") {
                ctx.kind = ListKind::Ol(v);
            }
        }

        let marker = ctx.prefix();
        let numbered_one = matches!(ctx.kind, ListKind::Ul | ListKind::Ol(1));
        ctx.advance_ol_number();
        let item = Item::render(h, &child, &ctx, marker.len());
        if let Some(prev) = prev_item {
            push_gap(text, item_gap(prev, item.first), lb);
        } else {
            let interrupts = lazy || (numbered_one && !item.marker_alone(&marker));
            start_list(text, &ctx, interrupts);
        }
        item.write_to(text, &marker, ctx.list_indent_len());
        ctx.started = true;
        prev_item = Some(item.last);
        h.put_buf(item.body);
    }
    if has_items {
        h.set_last_list(id, use_alt_marker);
    }
    // without a line end, text after the list would continue its last item
    if text.len() > start {
        add_block_end(text, opts.table_cell);
    }
}

/// Writes `node`, content directly in the list, after the item that ended as `prev`;
/// returns whether it wrote anything. `space_due` is whether the last stray child that
/// did ends in an `<option>`.
fn write_stray<H: MDHost>(
    h: &H,
    text: &mut String,
    node: &NodeRef,
    prev: Option<End>,
    space_due: &mut bool,
    cell: bool,
) -> bool {
    let mark = text.len();
    // the item's line ends before the stray content
    if let Some(prev) = prev {
        push_gap(text, item_gap(prev, None), linebreak(cell));
    }
    // a stray child keeps the cell rules, like the link label does
    let mut opts = FormatOpts::new().include_node();
    opts.table_cell = cell;
    let at = text.len();
    *space_due = write_inline(h, text, node, opts, *space_due);
    let wrote = text.len() != at;
    if !wrote {
        text.truncate(mark);
    }
    wrote
}

/// Ends the stray text before a marker: its hard breaks only end its line, or with a
/// second one its paragraph; a lone one would let an empty item underline the text.
fn end_stray_line(text: &mut String) {
    let mut breaks = 0;
    while let Some(rest) = text.strip_suffix("  \n") {
        text.truncate(rest.len());
        breaks += 1;
    }
    if breaks > 0 {
        add_linebreaks(text, "\n", if breaks > 1 { "\n\n" } else { "\n" });
    }
}

/// Separates an item that follows no item from what precedes it in `text`;
/// `interrupts` if the item can start right under the line of text above.
fn start_list(text: &mut String, ctx: &ListContext, interrupts: bool) {
    let lb = ctx.linebreak();
    trim_trailing_space(text);
    if text.is_empty() {
        return;
    }
    // text directly in the list would run into the marker
    let in_line = !text.ends_with(lb);
    if in_line {
        text.push_str(lb);
    }
    // a blank line keeps the item from joining the line above: always after text that
    // starts the list, where an empty item would underline it as a setext heading, and
    // elsewhere unless the item interrupts it
    let blank = (in_line && !ctx.started) || (!interrupts && !text.ends_with("\n\n"));
    if blank && !ctx.opts.table_cell {
        text.push('\n');
    }
}

/// How a block starts, seen from the line above it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Start {
    Text,
    /// a `<p>`, set off by blank lines that keep its item loose
    Para,
    Quote,
    /// a heading, fence or rule, or a list item that can interrupt a paragraph
    Block,
    /// a list item that cannot interrupt a paragraph (`CommonMark` §5.2)
    List,
}

/// How a block ends, seen from the line below it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum End {
    /// a paragraph at this column: text after it continues it
    Text,
    Para,
    /// a `>` line at this column: text after it would join the quote lazily
    Quote,
    /// a table row or a line in a nested list: text after it would join it, but
    /// any block start, a list item too, ends it
    Lazy,
    /// a heading, fence or rule, which ends at its own line end
    Closed,
    /// raw HTML, which runs on to the next blank line (`<` in text is escaped)
    Html,
}

/// The separator between two blocks.
#[derive(Clone, Copy)]
enum Gap {
    Line,
    Blank,
    /// an empty `>` line, which ends the quote's paragraph without a blank line
    QuoteEnd,
}

/// The separator between two blocks of an item, from how the first ends and the
/// next starts (`CommonMark` §4.8, §5.1). Only the pairs listed can share lines.
fn gap(prev: End, next: Start) -> Gap {
    match (prev, next) {
        // the blank lines around a paragraph keep its item loose
        (End::Para, _) | (_, Start::Para) => Gap::Blank,
        // text would continue the quote's paragraph lazily
        (End::Quote, Start::Text) => Gap::QuoteEnd,
        // a heading, fence or rule ends at its own line end, a `>` line or block start
        // interrupts a paragraph, and any item start (§5.2) ends a quote, table or nested item
        (End::Closed, _)
        | (End::Text, Start::Quote | Start::Block)
        | (End::Quote, Start::Block | Start::List)
        | (End::Lazy, Start::Quote | Start::Block | Start::List) => Gap::Line,
        // raw HTML runs on to a blank line, two quotes in a row would merge, and text
        // would continue the paragraph, table or nested item above it
        _ => Gap::Blank,
    }
}

/// The separator between items: a marker starts the next one after any line, so a line
/// break, unless a paragraph at either side keeps the blank line that makes the list loose.
fn item_gap(prev: End, next: Option<Start>) -> Gap {
    if prev == End::Para || next == Some(Start::Para) {
        Gap::Blank
    } else {
        Gap::Line
    }
}

fn push_gap(text: &mut String, gap: Gap, lb: &str) {
    text.push_str(lb);
    match gap {
        Gap::Line => {}
        Gap::Blank => text.push_str(lb),
        Gap::QuoteEnd => {
            text.push('>');
            text.push_str(lb);
        }
    }
}

/// An item's blocks written at column zero, joined by their gaps.
struct Item {
    body: String,
    /// how the first block starts; `None` for an empty item
    first: Option<Start>,
    last: End,
    /// whether the first block is a nested list, which sits at the content column
    /// (`- - a`), and so do the lists after it, or they would nest in its items
    starts_with_list: bool,
    /// the column of the content in a cell, where nothing indents it, else 0
    column: usize,
    /// how far past the content column a nested list sits (`LIST_OFFSET_BASE`)
    nested: usize,
    /// whether the item is in a table cell, whose lines are joined by `<br>`
    cell: bool,
}

impl Item {
    /// Writes the children of `li` as blocks into `body`, each alone in `buf` first:
    /// consecutive inline children make one text block, a wrapper of blocks gives its own.
    fn render<H: MDHost>(h: &H, li: &NodeRef, ctx: &ListContext, marker_len: usize) -> Self {
        let buf = &mut h.scratch();
        let cell = ctx.opts.table_cell;
        let mut item = Item {
            // the list gives it back with `put_buf`
            body: h.take_buf(),
            first: None,
            // an empty item takes no lazy line
            last: End::Closed,
            starts_with_list: false,
            // a nested list's lines are indented to the content column by `write_to`,
            // except in a cell, so there the list writes its whole column itself
            column: if cell {
                ctx.list_indent_len() + marker_len
            } else {
                0
            },
            nested: LIST_OFFSET_BASE.saturating_sub(marker_len),
            cell,
        };
        // inline text can start a line, so it is escaped as at a line start
        let opts = ctx.opts.list_item().include_node();
        // text holding a `<p>` is a paragraph, which keeps the item loose
        let has_para = |c: &NodeRef| holds_para(h, c);
        if li.children_it(false).all(|c| !node_is_block(&c)) {
            // the common item, in one call, and none for an empty one
            if li.first_child().is_some() {
                h.write(buf, li.id, opts.offset(item.list_offset(false)));
                // the `<li>` ends in a hard break of its own, which is no paragraph
                let lb = linebreak(cell);
                let own = buf.strip_suffix(lb).and_then(|b| b.strip_suffix("  "));
                let may_para = para_break(own.unwrap_or(buf), cell);
                let holds_para = || may_para && li.children_it(false).any(|c| has_para(&c));
                item.push(buf, holds_para, false);
            }
            return item;
        }
        // the inline children whose text is in `buf`; a `<p>` in one makes it a paragraph
        let mut wrote = Vec::new();
        // the space after an `<option>` is written by the next write, not this one
        let mut space_due = false;
        let mut next = li.first_child();
        while let Some(c) = next {
            next = next_child(li, &c, false);
            if c.is_comment() || c.qual_name_ref().is_some_and(|n| h.is_skipped(&n)) {
                continue;
            }
            if node_is_block_wrapper(&c) {
                next = next_child(li, &c, true);
                continue;
            }
            if !node_is_block(&c) {
                let (opts, mark) = (opts.offset(item.list_offset(false)), buf.len());
                space_due = write_inline(h, buf, &c, opts, space_due);
                // a child that wrote no text shows none in a `<p>`; an emphasis run that
                // joins the one before rewrites the text before `mark`
                if buf.get(mark..).is_none_or(|w| !w.trim_ascii().is_empty()) {
                    wrote.push(c);
                }
                continue;
            }
            let may_para = para_break(buf, cell);
            item.push(buf, || may_para && wrote.iter().any(has_para), false);
            wrote.clear();
            space_due = false;
            let is_list = node_is_list(&c);
            if c.has_name("hr") {
                // a cell has no line for a rule; the blocks around it stay apart
                if !cell {
                    buf.push_str("***");
                }
            } else {
                let mut block_opts = opts.offset(item.list_offset(is_list));
                // a `<p>` adds only blank lines around its text, which the gap replaces
                block_opts.include_node = !c.has_name("p");
                h.write(buf, c.id, block_opts);
            }
            item.push(buf, || has_para(&c), is_list);
        }
        let may_para = para_break(buf, cell);
        item.push(buf, || may_para && wrote.iter().any(has_para), false);
        item
    }

    /// The column of the nested lists in the next block: `nested` past the content
    /// column, or the content column for a list that starts the item and every list after it.
    fn list_offset(&self, is_list: bool) -> usize {
        if self.starts_with_list || (is_list && self.first.is_none()) {
            self.column
        } else {
            self.column + self.nested
        }
    }

    /// Moves the block in `buf` to the body after the gap its edges call for; an empty
    /// block is dropped. Text that holds a `<p>` is a paragraph; a quote, table or list
    /// holding one is not.
    fn push(&mut self, buf: &mut String, holds_para: impl FnOnce() -> bool, is_list: bool) {
        let (offset, cell) = (self.list_offset(is_list), self.cell);
        let (block, indent) = trim_block(buf, cell);
        if !block.is_empty() {
            let head = first_line(block, cell);
            let (first, last) = match (first_edge(head), last_edge(last_line(block, cell), is_list))
            {
                (Start::Text, End::Text) if holds_para() => (Start::Para, End::Para),
                edges => edges,
            };
            if self.first.is_none() {
                self.first = Some(first);
                // a list holding only stray text starts no list
                self.starts_with_list = is_list && strip_marker(head).is_some();
            } else {
                // a cell has no blank line, nor a line to hold a `>`
                let gap = if cell {
                    Gap::Line
                } else {
                    gap(self.last, first)
                };
                push_gap(&mut self.body, gap, linebreak(cell));
            }
            // only a nested list starts a line with a marker, at `offset`, but a table written as
            // text trims it; in a cell, which holds no table, emphasis can open a line too
            if strip_marker(head).is_some() && (indent == offset || !cell) {
                self.body.extend(std::iter::repeat_n(' ', offset));
            }
            self.body.push_str(block);
            self.last = last;
        }
        buf.clear();
    }

    /// The item's first line, `---` written as a rule that a marker can't underline.
    fn first_line(&self) -> &str {
        let first = first_line(&self.body, self.cell);
        if first == "---" { "***" } else { first }
    }

    /// Whether the marker stands alone on its line: for an empty item, one that starts
    /// with a nested list keeping its own column, or one whose marker line would be a
    /// rule (`- - -`, `* ***`).
    fn marker_alone(&self, marker: &str) -> bool {
        let line = self.first_line();
        self.first.is_none() || line.starts_with(' ') || is_rule_like(marker, line)
    }

    /// Writes the list's indent, `marker` and the item, its first line on the marker
    /// line and every later line indented to the content column, or it would leave the item.
    fn write_to(&self, text: &mut String, marker: &str, list_indent: usize) {
        let cell = self.cell;
        text.extend(std::iter::repeat_n(' ', list_indent));
        let indent = list_indent + marker.len();
        if self.marker_alone(marker) {
            text.push_str(marker.trim_end());
            if self.first.is_none() {
                return;
            }
            text.push_str(linebreak(cell));
            if !cell {
                text.extend(std::iter::repeat_n(' ', indent));
            }
        } else {
            text.push_str(marker);
        }
        let line = self.first_line();
        text.push_str(line);
        let rest = &self.body[line.len()..];
        if cell {
            text.push_str(rest);
        } else {
            push_indented(text, rest, indent);
        }
    }
}

/// Appends inline `node` to the text in `buf`, with the space due if the last node that
/// wrote anything ends in an `<option>`; returns whether the space is due after `node`.
fn write_inline<H: MDHost>(
    h: &H,
    buf: &mut String,
    node: &NodeRef,
    mut opts: FormatOpts,
    space_due: bool,
) -> bool {
    let mark = buf.len();
    opts.space_due = space_due;
    h.write(buf, node.id, opts);
    if buf.len() == mark {
        space_due
    } else {
        ends_in_option(h, node)
    }
}

/// Appends `lines`, which start with a line break, indenting each line to `indent`
/// unless it is blank.
fn push_indented(text: &mut String, lines: &str, indent: usize) {
    let mut push_line = |line: &str| {
        text.push('\n');
        if !line.is_empty() {
            text.extend(std::iter::repeat_n(' ', indent));
        }
        text.push_str(line);
    };
    let mut starts = memchr_iter(b'\n', lines.as_bytes())
        .map(|i| i + 1)
        .peekable();
    while let Some(start) = starts.next() {
        let end = starts.peek().map_or(lines.len(), |next| next - 1);
        push_line(&lines[start..end]);
    }
}

fn first_line(block: &str, cell: bool) -> &str {
    let lb = linebreak(cell);
    find(block, lb, false).map_or(block, |i| &block[..i])
}

fn last_line(block: &str, cell: bool) -> &str {
    let lb = linebreak(cell);
    find(block, lb, true).map_or(block, |i| &block[i + lb.len()..])
}

/// The first, or with `rev` the last, index of `pat` in `hay`.
fn find(hay: &str, pat: &str, rev: bool) -> Option<usize> {
    let (hay, pat) = (hay.as_bytes(), pat.as_bytes());
    let at = |&i: &usize| hay[i..].starts_with(pat);
    if rev {
        memrchr_iter(pat[0], hay).find(at)
    } else {
        memchr_iter(pat[0], hay).find(at)
    }
}

/// `written` without the line breaks and spaces around it, and the indent of its
/// first line.
fn trim_block(written: &str, cell: bool) -> (&str, usize) {
    let mut block = written.trim_ascii_start();
    while let Some(after) = block.strip_prefix("<br>").filter(|_| cell) {
        block = after.trim_ascii_start();
    }
    let lead = &written[..written.len() - block.len()];
    let indent = lead.len() - lead.trim_end_matches(' ').len();
    block = block.trim_ascii_end();
    while cell && ends_with_cell_break(block) {
        block = block[..block.len() - "<br>".len()].trim_ascii_end();
    }
    (block, indent)
}

fn first_edge(line: &str) -> Start {
    if line.starts_with('>') {
        return Start::Quote;
    }
    let line = line.trim_start_matches(' ');
    // `---` under a line of text would underline it as a heading
    let underline = line.bytes().all(|b| b == b'-');
    if (ends_own_line(line) && !underline) || starts_interrupting_item(line) {
        Start::Block
    } else if strip_marker(line).is_some() {
        Start::List
    } else {
        Start::Text
    }
}

/// Only a nested list starts a line with a marker, and `|` in text is escaped, so a line
/// that does either, or is indented in a list's block, is in a nested item or a table.
fn last_edge(line: &str, in_list: bool) -> End {
    if line.starts_with('>') {
        return End::Quote;
    }
    if line.starts_with('<') {
        return End::Html;
    }
    let indented = line.trim_start_matches(' ');
    let mut rest = indented;
    while let Some(after) = strip_marker(rest) {
        rest = after;
    }
    // inline space can indent a line of text too
    let nested = rest.len() < indented.len() || (in_list && indented.len() < line.len());
    // an empty item or a table row in a nested item takes no lazy text
    if ends_own_line(rest) || (nested && (rest.is_empty() || rest.starts_with('|'))) {
        End::Closed
    } else if nested || line.starts_with('|') {
        End::Lazy
    } else {
        End::Text
    }
}

/// Whether the line is a heading, fence or rule, each ending at its own line end.
fn ends_own_line(line: &str) -> bool {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    ((1..=6).contains(&hashes) && matches!(line.as_bytes().get(hashes), None | Some(b' ')))
        // a backtick fence's info string holds no backtick; a code span's line does
        || (line.starts_with("```") && !line.trim_start_matches('`').contains('`'))
        || line.starts_with("~~~")
        || is_rule_like(line, "")
}

/// Whether the line starts an item that can interrupt a paragraph: one with
/// content, numbered 1 if ordered (`CommonMark` §5.2).
fn starts_interrupting_item(line: &str) -> bool {
    let numbered = line.starts_with(|c: char| c.is_ascii_digit());
    strip_marker(line).is_some_and(|rest| !rest.is_empty())
        && (!numbered || line.starts_with("1.") || line.starts_with("1)"))
}

/// The line after a leading list marker and the spaces after it.
fn strip_marker(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    let len = match line.as_bytes().get(digits)? {
        b'-' | b'*' | b'+' if digits == 0 => 1,
        b'.' | b')' if (1..=9).contains(&digits) => digits + 1,
        _ => return None,
    };
    let rest = &line[len..];
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim_start_matches(' '))
}

/// Whether the marker line is three or more `-` or `*` and spaces, a rule.
fn is_rule_like(marker: &str, first: &str) -> bool {
    let line = || marker.bytes().chain(first.bytes()).filter(|&b| b != b' ');
    (line().all(|b| b == b'-') || line().all(|b| b == b'*')) && line().count() >= 3
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

/// Whether the last thing `node` writes is an `<option>`.
fn ends_in_option<H: MDHost>(h: &H, node: &NodeRef) -> bool {
    let descend = |n: &NodeRef| !n.qual_name_ref().is_some_and(|q| h.is_skipped(&q));
    let last = |n: &NodeRef| n.has_name("option") || (n.is_text() && !n.text().is_empty());
    find_node(node, true, descend, last).is_some_and(|n| n.has_name("option"))
}

/// Whether `written` holds the blank line, or `<br>` in a cell, that a `<p>` with text leaves.
fn para_break(written: &str, cell: bool) -> bool {
    find(written, if cell { "<br>" } else { "\n\n" }, false).is_some()
}

/// Whether `node` holds visible text in a `<p>`. Lists and skipped elements hide both,
/// and a link hides its `<p>`s, as it writes its blocks as inline text.
fn holds_para<H: MDHost>(h: &H, node: &NodeRef) -> bool {
    let rendered = |n: &NodeRef| {
        n.qual_name_ref()
            .is_some_and(|q| !is_list(&q) && !h.is_skipped(&q))
    };
    // a `<p>` is not entered: its own walk looks for text in it
    let descend = |n: &NodeRef| rendered(n) && !n.has_name("a") && !n.has_name("p");
    let para = |n: &NodeRef| n.has_name("p") && has_visible_text(h, n);
    find_node(node, false, descend, para).is_some()
}

/// The first node of `node`'s subtree, in document order or its reverse, that
/// matches; only the nodes `descend` matches are looked into.
fn find_node<'a>(
    node: &NodeRef<'a>,
    rev: bool,
    descend: impl Fn(&NodeRef) -> bool,
    matches: impl Fn(&NodeRef) -> bool,
) -> Option<NodeRef<'a>> {
    let mut next = Some(*node);
    while let Some(n) = next {
        if matches(&n) {
            return Some(n);
        }
        next = match (descend(&n), rev) {
            (false, _) => None,
            (true, false) => n.first_child(),
            (true, true) => n.last_child(),
        };
        // past the subtree's end, climb to the next sibling short of `node`
        let mut up = n;
        while next.is_none() && up.id != node.id {
            next = if rev {
                up.prev_sibling()
            } else {
                up.next_sibling()
            };
            up = up.parent()?;
        }
    }
    None
}

/// An element written as its own block outside link text.
pub(in crate::serializing::md) fn node_is_block(node: &NodeRef) -> bool {
    node_is_md_block(node) || node.has_name("pre")
}

/// The child of `li` after `node` in document order, where block wrappers are flattened:
/// into `node` when `enter` is set, and out of each wrapper that ends.
fn next_child<'a>(li: &NodeRef<'a>, node: &NodeRef<'a>, enter: bool) -> Option<NodeRef<'a>> {
    if let Some(first) = node.first_child().filter(|_| enter) {
        return Some(first);
    }
    let mut n = *node;
    loop {
        if let Some(sib) = n.next_sibling() {
            return Some(sib);
        }
        n = n.parent().filter(|p| p.id != li.id)?;
    }
}

/// A `<div>`-like element holding only blocks or lists; its children are written
/// as the item's own blocks. Inline text, a table or a rule would be glued or flattened.
fn node_is_block_wrapper(node: &NodeRef) -> bool {
    if !node
        .qual_name_ref()
        .is_some_and(|name| is_block_wrapper(&name))
    {
        return false;
    }
    let mut has_block = false;
    let all_blocks = node.children_it(false).all(|n| {
        has_block |= node_is_block(&n);
        (node_is_block(&n) && !n.has_name("table") && !n.has_name("hr")) || is_blank(&n)
    });
    all_blocks && has_block
}

fn node_is_list(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_list(&name))
}

/// A comment or a text node holding only whitespace: neither shows content.
fn is_blank(node: &NodeRef) -> bool {
    node.is_comment() || node.is_ascii_empty()
}

/// Whether the subtree renders text; skipped elements and nested lists render none.
fn has_visible_text<H: MDHost>(h: &H, node: &NodeRef) -> bool {
    let rendered = |n: &NodeRef| {
        n.qual_name_ref()
            .is_some_and(|name| !h.is_skipped(&name) && !is_list(&name))
    };
    let text = find_node(
        node,
        false,
        |n| rendered(n) && !n.has_name("img"),
        |n| {
            n.is_nonempty_text()
                || (n.has_name("img")
                    && rendered(n)
                    && n.element_ref().and_then(|el| image_src(&el)).is_some())
        },
    );
    text.is_some()
}

fn list_number(node: &NodeRef, key: &str) -> Option<u32> {
    node.attr(key).and_then(|v| parse_list_number(&v))
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
    match meaningful.len() {
        // the input was "0" or multiple zeros ("000")
        0 => Some(0),
        // 1..=9 digits always fit u32
        1..=9 => meaningful.parse().ok(),
        // clamp to the CommonMark limit
        _ => Some(MAX_LIST_NUMBER),
    }
}
