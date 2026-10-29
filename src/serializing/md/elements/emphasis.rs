use std::fmt::Write;

use html5ever::{QualName, local_name};

use crate::node::NodeData;
use crate::{Element, NodeId, NodeRef};

use super::super::block::is_md_block;
use super::super::ctx::{EmphasisScope, FormatOpts};
use super::super::host::MDHost;
use super::super::text_utils::escape_trailing_bang;
use super::image::image_src;

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    let Some(emphasis) = md_emphasis(&el.name) else {
        return;
    };

    let cur_scope = EmphasisScope::from(emphasis);
    let em_opts = opts.include_node().inline().emphasis_scope(cur_scope);
    let mut emphasis_text = String::new();

    h.write_children(&mut emphasis_text, id, em_opts);

    if emphasis_text.is_empty() {
        return;
    }

    if opts.emphasis_scope.contains(cur_scope) {
        push_emphasis(text, &mut emphasis_text, "");
        return;
    }
    // `*` written by the emphasis before joins this run, so look past it
    let before = text.trim_end_matches('*');
    if can_use_marker(before, &emphasis_text, || is_safe_after(h, id, emphasis)) {
        push_emphasis(text, &mut emphasis_text, emphasis);
    } else {
        push_html_emphasis(text, &emphasis_text, emphasis);
    }
}

/// Whether a `*` run around `content` can open and close: punctuation inside
/// the run needs whitespace or punctuation outside it.
fn can_use_marker(before: &str, content: &str, safe_after: impl FnOnce() -> bool) -> bool {
    // only ASCII spaces are moved outside the run; other whitespace stays inside
    let inner = content.trim_matches(' ');
    let (Some(first), Some(last)) = (
        inner.trim_start_matches('*').chars().next(),
        inner.trim_end_matches('*').chars().last(),
    ) else {
        return true;
    };
    if first.is_whitespace() || last.is_whitespace() {
        return false;
    }
    let opens = first.is_alphanumeric()
        || content.starts_with(' ')
        || before.chars().last().is_none_or(is_flanking_safe);
    let closes = last.is_alphanumeric() || content.ends_with(' ') || safe_after();
    opens && closes
}

/// Common non-ASCII punctuation blocks; anything else (a letter, a mark, a
/// format character such as a soft hyphen) falls back to html.
const PUNCTUATION: &[(char, char)] = &[
    ('\u{a1}', '\u{ac}'),
    ('\u{ae}', '\u{bf}'),
    ('\u{2010}', '\u{2027}'),
    ('\u{2030}', '\u{205e}'),
    ('\u{3001}', '\u{3003}'),
    ('\u{3008}', '\u{3011}'),
    ('\u{ff01}', '\u{ff0f}'),
    ('\u{ff1a}', '\u{ff20}'),
    ('\u{ff3b}', '\u{ff40}'),
    ('\u{ff5b}', '\u{ff65}'),
];

/// Whitespace or punctuation.
fn is_flanking_safe(c: char) -> bool {
    c.is_whitespace()
        || c.is_ascii_punctuation()
        || !c.is_alphanumeric() && PUNCTUATION.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
}

/// Whether what follows `id` is safe after a closing `*`: the end of a line or
/// link text, text starting with whitespace or punctuation, or an emphasis of
/// the same type (the two merge).
fn is_safe_after<H: MDHost>(h: &H, id: NodeId, marker: &str) -> bool {
    let nodes = h.nodes();
    let mut node = &nodes[id.value()];
    // an inline parent writes nothing at its end (or a delimiter that joins this run)
    let next = loop {
        if let Some(next) = node.next_sibling {
            break next;
        }
        let Some(parent) = node.parent else {
            return true;
        };
        node = &nodes[parent.value()];
        if let NodeData::Element(e) = &node.data {
            // a link's text is followed by `]`
            if ends_line(&e.name) || (e.name.local == local_name!("a") && e.has_attr("href")) {
                return true;
            }
        }
    };
    // these write nothing of their own, so their text follows directly
    let mut next = next;
    while let NodeData::Element(e) = &nodes[next.value()].data {
        if !matches!(
            e.name.local,
            local_name!("span") | local_name!("sup") | local_name!("sub")
        ) {
            break;
        }
        let Some(child) = nodes[next.value()].first_child else {
            return false;
        };
        next = child;
    }
    match &nodes[next.value()].data {
        NodeData::Text { contents } => contents.chars().next().is_some_and(is_flanking_safe),
        // a link, code span and image start with `[`, `` ` `` and `!`
        NodeData::Element(e) => {
            let writes_text = || writes_text(h, NodeRef::new(next, h.tree()));
            ends_line(&e.name)
                || (e.name.local == local_name!("img") && image_src(e).is_some())
                || ((md_emphasis(&e.name) == Some(marker)
                    || (e.name.local == local_name!("a") && e.has_attr("href"))
                    || e.name.local == local_name!("code"))
                    && writes_text())
        }
        _ => false,
    }
}

/// Whether `node` writes any text: a non-empty text node outside skipped tags.
fn writes_text<H: MDHost>(h: &H, node: NodeRef) -> bool {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        for c in n.children_it(false) {
            if c.is_text() && !c.text().is_empty() {
                return true;
            }
            if c.qual_name_ref().is_some_and(|name| !h.is_skipped(&name)) {
                stack.push(c);
            }
        }
    }
    false
}

fn ends_line(name: &QualName) -> bool {
    is_md_block(name)
        || matches!(
            name.local,
            local_name!("br") | local_name!("li") | local_name!("td") | local_name!("th")
        )
}

fn push_html_emphasis(acc: &mut String, content: &str, marker: &str) {
    if content.starts_with(' ') && !acc.ends_with(' ') {
        acc.push(' ');
    }
    let tag = if marker == "**" { "strong" } else { "em" };
    let _ = write!(acc, "<{tag}>{}</{tag}>", content.trim_matches(' '));
    if content.ends_with(' ') {
        acc.push(' ');
    }
}

const fn md_emphasis(name: &QualName) -> Option<&'static str> {
    match name.local {
        local_name!("strong") | local_name!("b") => Some("**"),
        local_name!("em") | local_name!("i") => Some("*"),
        _ => None,
    }
}

fn push_emphasis(acc: &mut String, emphasis_content: &mut String, marker: &str) {
    if emphasis_content.trim().is_empty() {
        acc.push_str(emphasis_content);
        return;
    }

    let trim_prev_marker = try_trim_prev_emphasis_marker(acc, marker);

    if emphasis_content.starts_with(' ') {
        emphasis_content.remove(0);
        if !acc.ends_with(' ') {
            acc.push(' ');
        }
    }

    let push_end_whitespace = emphasis_content.ends_with(' ');
    if push_end_whitespace {
        emphasis_content.pop();
    }
    emphasis_content.push_str(marker);

    if push_end_whitespace {
        emphasis_content.push(' ');
    }

    if !trim_prev_marker {
        acc.push_str(marker);
    }

    // emphasis merged into the one around or before it adds no marker here
    if emphasis_content.starts_with('[') {
        escape_trailing_bang(acc);
    }
    acc.push_str(emphasis_content);
}

fn try_trim_prev_emphasis_marker(acc: &mut String, marker: &str) -> bool {
    if marker.is_empty() || acc.is_empty() {
        return false;
    }

    let has_space = acc.ends_with(' ');
    let trimmed = acc.trim_ascii_end();
    let trailing = &acc[trimmed.len()..];

    if (trailing.is_empty() || trailing == " ") && is_exact_marker_end(trimmed, marker) {
        let trim_bytes = marker.len() + usize::from(has_space);
        acc.truncate(acc.len() - trim_bytes);

        if has_space {
            acc.push(' ');
        }
        true
    } else {
        false
    }
}

#[inline]
fn is_exact_marker_end(tail: &str, marker: &str) -> bool {
    // this may change later, to support "_", "__"
    let Some(prefix) = tail.strip_suffix(marker) else {
        return false;
    };
    if marker == "*" && prefix.ends_with('*') {
        return false;
    }
    !prefix.bytes().rev().take_while(|&b| b == b'\\').count() % 2 != 0
}
