use html5ever::{LocalName, local_name};

use crate::{Element, NodeId, NodeRef};

use super::super::block::{
    BlockEnd, BlockWriter, LinePrefix, Preceding, Separator, is_list, is_md_block,
};
use super::super::constants::MAX_LIST_NUMBER;
use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::image::image_src;

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    let ordered = el.name.local == local_name!("ol");
    let mut number = el
        .attr("start")
        .and_then(|v| parse_list_number(&v))
        .unwrap_or(1);
    let list = NodeRef::new(id, h.tree());
    let has_items = list.children_it(false).any(|c| c.has_name("li"));
    // a list right after one of the same type takes the other marker, or the two would merge
    let use_alt_marker = !opts.table_cell && has_items && should_use_alt_marker(h, &el.name.local);
    let mut item_opts = opts.list_item();
    item_opts.include_node = false;
    // content directly in the list is no item's: it keeps only the cell and emphasis rules
    let mut stray_opts = FormatOpts::new().include_node();
    stray_opts.table_cell = opts.table_cell;
    stray_opts.emphasis_scope = opts.emphasis_scope;
    let mut preceding = Preceding::Nothing;
    for child in list.children_it(false) {
        if !child.has_name("li") {
            // whitespace between pieces of such content still keeps them apart
            if is_blank(&child) && !matches!(preceding, Preceding::Stray(_)) {
                continue;
            }
            // content directly in the list: after an item it takes the next line, which
            // continues that item lazily unless a paragraph or block ends it
            let lazy = match preceding {
                Preceding::Item(end) => {
                    out.owe_exactly(if end == BlockEnd::Para {
                        Separator::BlankLine
                    } else {
                        Separator::LineBreak
                    });
                    // a blank line the item still owes (after a div, quote or list) ends it
                    out.owed() == Separator::LineBreak
                        && matches!(end, BlockEnd::Text | BlockEnd::Quote | BlockEnd::Lazy)
                }
                Preceding::Stray(end) => end == BlockEnd::Lazy,
                Preceding::Nothing => false,
            };
            if out.starts_line_or_block(|out| h.write(out, child.id, stray_opts)) {
                // text that starts the list would be underlined by an empty item after it
                preceding = Preceding::Stray(if lazy {
                    BlockEnd::Lazy
                } else if matches!(preceding, Preceding::Nothing) {
                    BlockEnd::Para
                } else {
                    BlockEnd::Text
                });
            }
            continue;
        }
        if ordered {
            // `<li value>` renumbers this item and the ones after it
            number = list_number(&child, "value").unwrap_or(number);
        }
        let marker = match (ordered, use_alt_marker) {
            (true, false) => format!("{number}. "),
            (true, true) => format!("{number}) "),
            (false, true) => "* ".to_owned(),
            (false, false) if opts.table_cell => "+ ".to_owned(),
            (false, false) => "- ".to_owned(),
        };
        number = number.saturating_add(1).min(MAX_LIST_NUMBER);
        out.open_prefix(LinePrefix::ListItem { marker, preceding });
        h.write(out, child.id, item_opts);
        preceding = Preceding::Item(out.close_prefix().unwrap_or(BlockEnd::Closed));
    }
    if has_items {
        h.set_last_list(id, use_alt_marker);
    }
    // without a line end, text after the list would continue its last item
    out.owe(Separator::BlankLine);
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
    out: &mut BlockWriter,
    el: &Element,
    id: NodeId,
) {
    let is_checkbox = el
        .attr("type")
        .is_some_and(|t| t.eq_ignore_ascii_case("checkbox"));
    if is_checkbox && starts_list_item(h, NodeRef::new(id, h.tree())) {
        out.push_raw(if el.has_attr("checked") {
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

/// The first node of `node`'s subtree, in document order, that matches;
/// only the nodes `descend` matches are looked into.
fn find_node<'a>(
    node: &NodeRef<'a>,
    descend: impl Fn(&NodeRef) -> bool,
    matches: impl Fn(&NodeRef) -> bool,
) -> Option<NodeRef<'a>> {
    let mut next = Some(*node);
    while let Some(n) = next {
        if matches(&n) {
            return Some(n);
        }
        next = if descend(&n) { n.first_child() } else { None };
        // past the subtree's end, climb to the next sibling short of `node`
        let mut up = n;
        while next.is_none() && up.id != node.id {
            next = up.next_sibling();
            up = up.parent()?;
        }
    }
    None
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
    let rest = value.trim_start_matches(|c: char| c.is_ascii_digit());
    let digits = value.strip_suffix(rest).unwrap_or_default();
    if digits.is_empty() {
        return None;
    }
    // md doesn't support negative list markers, clamp to zero
    if negative {
        return Some(0);
    }

    // strip leading zeros from the digit slice
    let meaningful = digits.trim_start_matches('0');
    match meaningful.len() {
        // the input was "0" or multiple zeros ("000")
        0 => Some(0),
        // 1..=9 digits always fit u32
        1..=9 => meaningful.parse().ok(),
        // clamp to the CommonMark limit
        _ => Some(MAX_LIST_NUMBER),
    }
}
