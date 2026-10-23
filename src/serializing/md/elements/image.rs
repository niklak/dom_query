use html5ever::local_name;
use tendril::StrTendril;

use crate::node::TreeNode;

use super::super::ctx::FormatOpts;
use super::super::serializer::MDSerializer;
use super::super::text_utils::{push_md_url, push_normalized_text, push_title};

impl MDSerializer<'_> {
    pub(in crate::serializing::md) fn write_img(text: &mut String, img_node: &TreeNode) {
        let Some(el) = img_node.as_element() else {
            return;
        };

        let src = el
            .attr("src")
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                el.attr("srcset").and_then(|s| {
                    s.split_ascii_whitespace()
                        .next()
                        .map(|u| u.trim_end_matches(','))
                        .filter(|u| !u.is_empty())
                        .map(StrTendril::from_slice)
                })
            })
            .or_else(|| el.attr("data-src").filter(|s| !s.trim().is_empty()));
        if let Some(src) = src {
            text.push_str("![");

            if let Some(alt) = el.attr_ref(local_name!("alt")) {
                push_normalized_text(text, alt, FormatOpts::new());
            }
            text.push(']');
            text.push('(');
            push_md_url(text, &src);
            if let Some(title) = el.attr_ref(local_name!("title")) {
                push_title(text, title);
            }
            text.push(')');
        }
    }
}
