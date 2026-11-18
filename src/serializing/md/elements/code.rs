use std::borrow::Cow;
use std::cell::Ref;

use html5ever::local_name;

use crate::node::{NodeData, TreeNode, ancestor_nodes, child_nodes, descendant_nodes};
use crate::{Element, NodeId};

use super::super::block::{Block, BlockWriter, Separator};
use super::super::constants::{CODE_LANGUAGE_ATTRIBUTES, CODE_LANGUAGE_PREFIX};
use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::inline::Inline;
use super::super::text_utils::{push_escaped_pipes, trim_trailing_space};

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    if el.name.local == local_name!("pre") {
        write_pre(h, out, id, opts, true);
    } else {
        write_code(h, out, id, opts);
    }
}

/// Tries to find the language label in the given node using a heuristic.
///
/// Pages may use custom `data-` attributes on the tag itself.
fn find_code_language<H: MDHost>(h: &H, id: NodeId) -> Option<String> {
    let nodes = h.nodes();
    // Check the current node
    if let Some(language) = nodes.get(id.value()).and_then(find_code_lang_attribute) {
        return Some(language);
    }

    ancestor_nodes(Ref::clone(&nodes), &id, Some(3))
        .filter_map(|ancestor_id| nodes.get(ancestor_id.value()))
        .find_map(find_code_lang_attribute)
        .or_else(|| find_code_language_css_class(h, id))
}

/// Tries to find the language from the CSS class of the first `<code>` element child of the `<pre>` block.
fn find_code_language_css_class<H: MDHost>(h: &H, id: NodeId) -> Option<String> {
    let nodes = h.nodes();
    let code_elem = child_nodes(Ref::clone(&nodes), &id, false).find_map(|child_id| {
        nodes
            .get(child_id.value())?
            .as_element()
            .filter(|el| el.name.local == local_name!("code"))
    });

    code_elem?
        .class()?
        .split_ascii_whitespace()
        .find_map(|class| class.strip_prefix(CODE_LANGUAGE_PREFIX))
        .map(sanitize_attr_value)
}

/// Transforms a `<pre>` code block, possibly with an associated language label that the resulting
/// block is annotated with.
fn write_pre<H: MDHost>(h: &H, out: &mut BlockWriter, id: NodeId, opts: FormatOpts, pre: bool) {
    let nodes = h.nodes();
    let content = pre_text(&nodes, id);
    let mut span = h.scratch();

    // a fence cannot go in link text; a code span keeps the text on one line
    if opts.link_body {
        let code_text = content.split_ascii_whitespace().collect::<Vec<_>>();
        push_code_text(&mut span, &code_text.join(" "), opts.table_cell);
        // a `<pre>` is a block, set apart; a `<code>` keeps only its edge whitespace
        let ws = |c: char| c.is_ascii_whitespace();
        if pre || content.starts_with(ws) {
            out.owe(Separator::Space);
        }
        if !span.is_empty() {
            out.push(Inline::Code(&span));
        }
        if pre || content.ends_with(ws) {
            out.owe(Separator::Space);
        }
        return;
    }

    // a cell cannot hold a fence; each line becomes a code span, joined like cell blocks
    if opts.table_cell {
        out.owe(Separator::LineBreak);
        // a lone `\r` (possible via the DOM API) is a line ending too and would end the row
        for (i, line) in content.lines().flat_map(|l| l.split('\r')).enumerate() {
            if i > 0 {
                out.push(Inline::HtmlBreak);
            }
            span.clear();
            push_code_text(&mut span, line, true);
            if !span.is_empty() {
                out.push(Inline::Code(&span));
            }
        }
        out.owe(Separator::LineBreak);
        return;
    }

    // The fence must be longer than any backtick run in the content,
    // otherwise an interior fence-length line terminates the block early
    // (CommonMark §fenced-code-blocks).
    let fence_len = max_backtick_run(&content).max(2) + 1;
    span.extend(std::iter::repeat_n('`', fence_len));
    if let Some(lang) = find_code_language(h, id) {
        span.push_str(&lang);
    }
    out.push_block(Block::CodeFence, &span);
    // a final newline of the content already ends its last line
    if !content.is_empty() {
        out.push_block(
            Block::CodeFence,
            content.strip_suffix('\n').unwrap_or(&content),
        );
    }
    let fence = span.get(..fence_len).unwrap_or_default();
    out.push_block(Block::CodeFence, fence);
    out.owe(Separator::LineBreak);
}

/// The text of a `<pre>` block, with each `<br>` as a line break.
fn pre_text<'a>(nodes: &'a Ref<Vec<TreeNode>>, id: NodeId) -> Cow<'a, str> {
    let mut content = Cow::Borrowed("");
    for child_id in descendant_nodes(Ref::clone(nodes), &id) {
        let Some(node) = nodes.get(child_id.value()) else {
            continue;
        };
        match &node.data {
            // a lone text node, the usual case, is borrowed
            NodeData::Text { contents } if content.is_empty() => {
                content = Cow::Borrowed(contents.as_ref());
            }
            NodeData::Text { contents } => content.to_mut().push_str(contents),
            NodeData::Element(el) if el.name.local == local_name!("br") => {
                content.to_mut().push('\n');
            }
            _ => {}
        }
    }
    content
}

/// Whether the text under `id` starts and ends with a line break, past spaces and tabs.
fn edge_line_breaks<H: MDHost>(h: &H, id: NodeId) -> (bool, bool) {
    let nodes = h.nodes();
    let (mut starts, mut ends) = (None, false);
    for child_id in descendant_nodes(Ref::clone(&nodes), &id) {
        let Some(NodeData::Text { contents }) = nodes.get(child_id.value()).map(|n| &n.data) else {
            continue;
        };
        let rest = contents.trim_start_matches([' ', '\t']);
        if !rest.is_empty() {
            starts.get_or_insert(rest.starts_with('\n'));
            ends = rest.trim_end_matches([' ', '\t']).ends_with('\n');
        }
    }
    (starts == Some(true), ends)
}

/// Writes the content of the `<code>` block. Generally a `<code>` tag is used inline, but unfortunately
/// it's also used instead of a `<pre>` block. In case the `<code>` block contains multiline
/// text, it's handled as a `<pre>` code block.
fn write_code<H: MDHost>(h: &H, out: &mut BlockWriter, id: NodeId, opts: FormatOpts) {
    let is_multiline = {
        let nodes = h.nodes();
        descendant_nodes(Ref::clone(&nodes), &id)
            .filter_map(|id| nodes.get(id.value()))
            .filter_map(|t| match t.data {
                NodeData::Text { ref contents } => Some(contents),
                _ => None,
            })
            .any(|text| text.trim().contains('\n'))
    };

    if is_multiline {
        return write_pre(h, out, id, opts, false);
    }

    // edge spaces are content, so keep them; whitespace around a line
    // break at an edge is source indentation
    let (starts_with_break, ends_with_break) = edge_line_breaks(h, id);
    let code_opts = FormatOpts::new()
        .include_node()
        .inline()
        .skip_escape()
        .skip_md();
    let mut code = BlockWriter::fragment(h.take_buf(), code_opts);
    h.write(&mut code, id, code_opts);
    let mut code_text = code.finish();
    if ends_with_break {
        trim_trailing_space(&mut code_text);
    }
    if starts_with_break {
        code_text.drain(..code_text.len() - code_text.trim_start_matches(' ').len());
    }
    let mut span = h.scratch();
    push_code_text(&mut span, &code_text, opts.table_cell);
    if !span.is_empty() {
        out.push(Inline::Code(&span));
    }
    h.put_buf(code_text);
}

fn find_code_lang_attribute(node: &TreeNode) -> Option<String> {
    node.as_element()?
        .attrs
        .iter()
        .find(|attr| CODE_LANGUAGE_ATTRIBUTES.contains(&attr.name.local.as_ref()))
        .map(|attr| sanitize_attr_value(&attr.value))
}

/// Keep only the first whitespace‑delimited token, alphanumerics and conservative punctuation.
fn sanitize_attr_value(raw: &str) -> String {
    let token = raw.split_ascii_whitespace().next().unwrap_or("");
    token
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '+' | '.' | '#' | ';'))
        // the label lands in a `class` attribute downstream, so keep it short
        .take(64)
        .collect()
}

fn max_backtick_run(text: &str) -> usize {
    if memchr::memchr(b'`', text.as_bytes()).is_none() {
        return 0;
    }
    let mut max = 0;
    let mut current = 0;
    for b in text.as_bytes() {
        current = if *b == b'`' { current + 1 } else { 0 };
        max = max.max(current);
    }
    max
}

/// A code span right before would merge its closing run with the next one's opening:
/// `x` and `y` would give `x``y`. An escaped backtick is text, which no span ends with.
pub(in crate::serializing::md) fn ends_with_code_span(text: &str) -> bool {
    text.strip_suffix('`')
        .is_some_and(|before| before.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 0)
}

fn push_code_text(text: &mut String, code_text: &str, in_cell: bool) {
    if code_text.is_empty() {
        return;
    }

    let fence_len = max_backtick_run(code_text) + 1;
    let bytes = code_text.as_bytes();
    // a backtick at either edge would merge into the fence, a trailing backslash would pass for an
    // escaped backtick before a next span, and a parser strips one space from each side of ` a `
    let needs_space = bytes.first() == Some(&b'`')
        || matches!(bytes.last(), Some(b'`' | b'\\'))
        || (bytes.first() == Some(&b' ')
            && bytes.last() == Some(&b' ')
            && bytes.iter().any(|&b| b != b' '));

    text.extend(std::iter::repeat_n('`', fence_len));
    if needs_space {
        text.push(' ');
    }
    // GFM strips one backslash before a `|` in a table cell, even in code, where a
    // backslash is literal: a code span in a cell must escape its own pipes
    if in_cell {
        push_escaped_pipes(text, code_text, false);
    } else {
        text.push_str(code_text);
    }
    if needs_space {
        text.push(' ');
    }
    text.extend(std::iter::repeat_n('`', fence_len));
}
