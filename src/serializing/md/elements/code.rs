use std::cell::Ref;

use html5ever::local_name;

use crate::node::{NodeData, TreeNode, ancestor_nodes, child_nodes, descendant_nodes};
use crate::{Element, NodeId, TreeNodeOps};

use super::super::constants::{CODE_LANGUAGE_ATTRIBUTES, CODE_LANGUAGE_PREFIX};
use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::text_utils::{push_block_space, trim_trailing_space};

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    el: &Element,
    id: NodeId,
    opts: FormatOpts,
) {
    if el.name.local == local_name!("pre") {
        write_pre(h, text, id, opts);
    } else {
        write_code(h, text, id, opts);
    }
}

/// Tries to find the language label in the given node using a heuristic.
///
/// Pages may use custom `data-` attributes on the tag itself.
fn find_code_language<H: MDHost>(h: &H, id: NodeId) -> Option<String> {
    let nodes = h.nodes();
    // Check the current node
    if let Some(language) = find_code_lang_attribute(&nodes[id.value()]) {
        return Some(language);
    }

    ancestor_nodes(Ref::clone(&nodes), &id, Some(3))
        .find_map(|ancestor_id| find_code_lang_attribute(&nodes[ancestor_id.value()]))
        .or_else(|| find_code_language_css_class(h, id))
}

/// Tries to find the language from the CSS class of the first `<code>` element child of the `<pre>` block.
fn find_code_language_css_class<H: MDHost>(h: &H, id: NodeId) -> Option<String> {
    let nodes = h.nodes();
    let code_elem = child_nodes(Ref::clone(&nodes), &id, false).find_map(|child_id| {
        let node = &nodes[child_id.value()];
        node.as_element()
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
fn write_pre<H: MDHost>(h: &H, text: &mut String, id: NodeId, opts: FormatOpts) {
    let content = TreeNodeOps::text_of(h.nodes(), id);

    // a fence cannot go in link text; a code span keeps the text on one line
    if opts.link_body {
        let code_text = content.split_ascii_whitespace().collect::<Vec<_>>();
        push_block_space(text);
        push_code_text(text, &code_text.join(" "));
        // only a following sibling needs a separator; no trailing space ends the label
        if h.tree().next_sibling_of(&id).is_some() {
            push_block_space(text);
        }
        return;
    }

    // The fence must be longer than any backtick run in the content,
    // otherwise an interior fence-length line terminates the block early
    // (CommonMark §fenced-code-blocks).
    let fence_len = max_backtick_run(&content).max(2) + 1;

    if !text.ends_with("\n\n") {
        text.push('\n');
    }
    text.extend(std::iter::repeat_n('`', fence_len));

    if let Some(lang) = find_code_language(h, id) {
        text.push_str(&lang);
    }
    text.push('\n');

    text.push_str(&content);

    // The closing fence goes on its own line; a final newline of the
    // content already ends the last line, and empty content has no line.
    if !content.is_empty() && !content.ends_with('\n') {
        text.push('\n');
    }

    text.extend(std::iter::repeat_n('`', fence_len));
    text.push('\n');
}

/// Writes the content of the `<code>` block. Generally a `<code>` tag is used inline, but unfortunately
/// it's also used instead of a `<pre>` block. In case the `<code>` block contains multiline
/// text, it's handled as a `<pre>` code block.
fn write_code<H: MDHost>(h: &H, text: &mut String, id: NodeId, opts: FormatOpts) {
    let is_multiline = {
        let nodes = h.nodes();
        descendant_nodes(Ref::clone(&nodes), &id)
            .map(|id| &nodes[id.value()])
            .filter_map(|t| match t.data {
                NodeData::Text { ref contents } => Some(contents),
                _ => None,
            })
            .any(|text| text.trim().contains('\n'))
    };

    if is_multiline {
        return write_pre(h, text, id, opts);
    }

    // edge spaces are content, so keep them; whitespace around a line
    // break at an edge is source indentation
    // TODO: return to this -- read the raw edges without allocating the
    // full text a second time
    let raw = TreeNodeOps::text_of(h.nodes(), id);
    let mut code_text = String::new();
    h.write(
        &mut code_text,
        id,
        FormatOpts::new()
            .include_node()
            .inline()
            .skip_escape()
            .skip_md(),
    );
    if raw.trim_end_matches([' ', '\t']).ends_with('\n') {
        trim_trailing_space(&mut code_text);
    }
    if raw.trim_start_matches([' ', '\t']).starts_with('\n') {
        code_text.drain(..code_text.len() - code_text.trim_start_matches(' ').len());
    }
    push_code_text(text, &code_text);
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
    let mut max = 0;
    let mut current = 0;
    for b in text.as_bytes() {
        current = if *b == b'`' { current + 1 } else { 0 };
        max = max.max(current);
    }
    max
}

fn push_code_text(text: &mut String, code_text: &str) {
    if code_text.is_empty() {
        return;
    }

    // a code span right before would merge its closing run with this
    // opening one: `x` and `y` would give `x``y`, a single span
    if let Some(before) = text.strip_suffix('`') {
        // an escaped backtick is text, which a code span never ends with (see below)
        if before.bytes().rev().take_while(|&b| b == b'\\').count() % 2 == 0 {
            text.push(' ');
        }
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
    text.push_str(code_text);
    if needs_space {
        text.push(' ');
    }
    text.extend(std::iter::repeat_n('`', fence_len));
}
