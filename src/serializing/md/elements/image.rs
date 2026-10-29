use html5ever::local_name;
use tendril::StrTendril;

use crate::{Element, NodeId};

use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::text_utils::{push_md_url, push_normalized_text, push_title};

pub(in crate::serializing::md) fn write<H: MDHost>(
    _h: &H,
    text: &mut String,
    el: &Element,
    _id: NodeId,
    _opts: FormatOpts,
) {
    if let Some(src) = image_src(el) {
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

/// The image's address: `src`, else the first `srcset` candidate, else `data-src`;
/// without one the image writes nothing.
pub(in crate::serializing::md) fn image_src(el: &Element) -> Option<StrTendril> {
    el.attr("src")
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
        .or_else(|| el.attr("data-src").filter(|s| !s.trim().is_empty()))
}
