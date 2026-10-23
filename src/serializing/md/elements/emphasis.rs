use html5ever::{QualName, local_name};

use crate::NodeRef;
use crate::node::TreeNode;

use super::super::ctx::{EmphasisScope, FormatOpts};
use super::super::serializer::MDSerializer;
use super::super::text_utils::escape_trailing_bang;

impl MDSerializer<'_> {
    pub(in crate::serializing::md) fn write_emphasis(
        &self,
        text: &mut String,
        emphasis_node: &TreeNode,
        opts: FormatOpts,
    ) {
        let node = NodeRef::new(emphasis_node.id, self.root_node.tree);
        let Some(emphasis) = emphasis_node
            .as_element()
            .map(|el| &el.name)
            .and_then(md_emphasis)
        else {
            return;
        };

        let cur_scope = EmphasisScope::from(emphasis);
        let em_opts = opts.include_node().inline().emphasis_scope(cur_scope);
        let mut emphasis_text = String::new();

        for child_node in node.children_it(false) {
            self.write(&mut emphasis_text, child_node.id, em_opts);
        }

        if emphasis_text.is_empty() {
            return;
        }

        let marker = if opts.emphasis_scope.contains(cur_scope) {
            ""
        } else {
            emphasis
        };
        push_emphasis(text, &mut emphasis_text, marker);
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
