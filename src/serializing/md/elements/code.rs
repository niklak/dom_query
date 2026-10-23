use std::cell::Ref;

use html5ever::local_name;

use crate::TreeNodeOps;
use crate::node::{NodeData, TreeNode, ancestor_nodes, child_nodes, descendant_nodes};

use super::super::constants::{CODE_LANGUAGE_ATTRIBUTES, CODE_LANGUAGE_PREFIX};
use super::super::ctx::FormatOpts;
use super::super::serializer::MDSerializer;
use super::super::text_utils::{push_block_space, trim_trailing_space};

impl MDSerializer<'_> {
    /// Tries to find the language label in the given node using a heuristic.
    ///
    /// Pages may use custom `data-` attributes on the tag itself.
    fn find_code_language(&self, node: &TreeNode) -> Option<String> {
        // Check the current node
        if let Some(language) = find_code_lang_attribute(node) {
            return Some(language);
        }

        ancestor_nodes(Ref::clone(&self.nodes), &node.id, Some(3))
            .find_map(|id| find_code_lang_attribute(&self.nodes[id.value()]))
            .or_else(|| self.find_code_language_css_class(node))
    }

    /// Tries to find the language from the CSS class of the first `<code>` element child of the `<pre>` block.
    fn find_code_language_css_class(&self, pre_node: &TreeNode) -> Option<String> {
        let code_elem = child_nodes(Ref::clone(&self.nodes), &pre_node.id, false).find_map(|id| {
            let node = &self.nodes[id.value()];
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
    pub(in crate::serializing::md) fn write_pre(
        &self,
        text: &mut String,
        pre_node: &TreeNode,
        opts: FormatOpts,
    ) {
        let content = TreeNodeOps::text_of(Ref::clone(&self.nodes), pre_node.id);

        // a fence cannot go in link text; a code span keeps the text on one line
        if opts.link_body {
            let code_text = content.split_ascii_whitespace().collect::<Vec<_>>();
            push_block_space(text);
            push_code_text(text, &code_text.join(" "));
            push_block_space(text);
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

        if let Some(lang) = self.find_code_language(pre_node) {
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
    pub(in crate::serializing::md) fn write_code(
        &self,
        text: &mut String,
        code_node: &TreeNode,
        opts: FormatOpts,
    ) {
        let is_multiline = descendant_nodes(Ref::clone(&self.nodes), &code_node.id)
            .map(|id| &self.nodes[id.value()])
            .filter_map(|t| match t.data {
                NodeData::Text { ref contents } => Some(contents),
                _ => None,
            })
            .any(|text| text.trim().contains('\n'));

        if is_multiline {
            return self.write_pre(text, code_node, opts);
        }

        // edge spaces are content, so keep them; whitespace around a line
        // break at an edge is source indentation
        // TODO: return to this -- read the raw edges without allocating the
        // full text a second time
        let raw = TreeNodeOps::text_of(Ref::clone(&self.nodes), code_node.id);
        let mut code_text = String::new();
        self.write(
            &mut code_text,
            code_node.id,
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
}

fn find_code_lang_attribute(node: &TreeNode) -> Option<String> {
    node.as_element()?
        .attrs
        .iter()
        .find(|attr| CODE_LANGUAGE_ATTRIBUTES.contains(&attr.name.local.as_ref()))
        .map(|attr| sanitize_attr_value(&attr.value))
}

/// Keep only the first whitespace‑delimited token and a conservative set of characters.
fn sanitize_attr_value(raw: &str) -> String {
    let token = raw.split_ascii_whitespace().next().unwrap_or("");
    token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.' | '#'))
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
    if text.ends_with('`') {
        text.push(' ');
    }

    let fence_len = max_backtick_run(code_text) + 1;
    let bytes = code_text.as_bytes();
    // a backtick at either edge would merge into the fence, and a parser
    // strips one space from each side of content like ` a `
    let needs_space = bytes.first() == Some(&b'`')
        || bytes.last() == Some(&b'`')
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
