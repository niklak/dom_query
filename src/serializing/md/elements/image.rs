use html5ever::local_name;
use tendril::StrTendril;

use crate::{Element, NodeId};

use super::super::block::BlockWriter;
use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::inline::Inline;
use super::super::text_utils::{
    NEVER_AT_LINE_START, push_md_url, push_normalized_text, push_title,
};

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    el: &Element,
    _id: NodeId,
    _opts: FormatOpts,
) {
    if let Some(src) = image_src(el) {
        let mut text = h.scratch();
        text.push_str("![");

        if let Some(alt) = el.attr_ref(local_name!("alt")) {
            push_normalized_text(&mut text, NEVER_AT_LINE_START, alt, FormatOpts::new());
        }
        text.push(']');
        text.push('(');
        push_md_url(&mut text, &src);
        if let Some(title) = el.attr_ref(local_name!("title")) {
            push_title(&mut text, title);
        }
        text.push(')');
        out.push(Inline::Image(&text));
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
