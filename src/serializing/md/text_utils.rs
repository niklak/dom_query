use tendril::StrTendril;

use super::constants::{ALWAYS_ESCAPED, LINE_START_ESCAPED};

#[allow(clippy::cast_possible_truncation)]
pub(super) fn push_normalized_text(text: &mut StrTendril, new_text: &str, escape: bool) {
    let follows_newline = text.ends_with(['\n', ' ']) || text.is_empty();
    // a space mid-line does not make block syntax (`#`, `>`, ...) significant;
    // only an actual line start does. The indentation plus open list marker a
    // list item was just written with counts as one, or `1. a` inside an item
    // would serialize as a nested list and `#` as a heading.
    let is_line_start = at_block_start(text);
    let push_start_whitespace = !follows_newline && new_text.starts_with(char::is_whitespace);
    let push_end_whitespace = new_text.ends_with(char::is_whitespace);

    let mut result = StrTendril::with_capacity(new_text.len() as u32);
    let mut iter = new_text.split_whitespace();

    if let Some(first) = iter.next() {
        if push_start_whitespace {
            result.push_char(' ');
        }
        // only the first word of a text node can continue a Markdown line
        push_escaped_chunk(&mut result, first, escape, is_line_start);
        for word in iter {
            result.push_char(' ');
            push_escaped_chunk(&mut result, word, escape, false);
        }
    }
    if result.is_empty() && follows_newline {
        return;
    }

    text.push_tendril(&result);

    if push_end_whitespace && !text.ends_with(char::is_whitespace) {
        text.push_char(' ');
    }
}

/// True when the current line holds only indentation, optionally followed by
/// a list marker opened on this line (`- `, `+ `, `12. `). Block syntax
/// characters (`#`, `>`, markers) are significant at that position.
fn at_block_start(text: &str) -> bool {
    let line = text.rsplit('\n').next().unwrap_or("");
    let rest = line.trim_start_matches(' ');
    if rest.is_empty() {
        // empty line, or the continuation indentation inside a list item
        return true;
    }
    let Some(marker) = rest.strip_suffix(' ') else {
        return false;
    };
    if matches!(marker, "-" | "+") {
        return true;
    }
    is_ordered_list_marker(marker)
}

/// An ordered-list marker like `3.` or `12)` at the beginning of a line would
/// otherwise turn the line into a list item.
fn is_ordered_list_marker(chunk: &str) -> bool {
    let Some(stem) = chunk.strip_suffix(['.', ')']) else {
        return false;
    };
    !stem.is_empty() && stem.as_bytes().iter().all(u8::is_ascii_digit)
}

pub(super) fn push_escaped_chunk(
    text: &mut StrTendril,
    chunk: &str,
    escape: bool,
    line_start: bool,
) {
    if !escape {
        // inline code content, where backslash escapes are not interpreted
        text.push_slice(chunk);
        return;
    }
    let list_marker = line_start && is_ordered_list_marker(chunk);
    let mut chars = chunk.chars().peekable();
    let mut is_first = true;
    while let Some(c) = chars.next() {
        let escaped = if ALWAYS_ESCAPED.contains(&c) {
            true
        } else if LINE_START_ESCAPED.contains(&c) {
            // only the first character of the chunk can open a block
            // construct; a word of only `#` can also close an ATX heading
            // anywhere (`## Title #` renders as "Title")
            is_first && (line_start || (c == '#' && chunk.bytes().all(|b| b == b'#')))
        } else if c == '!' {
            chars.peek() == Some(&'[')
        } else if c == '.' || c == ')' {
            list_marker
        } else {
            false
        };
        if escaped {
            text.push_char('\\');
        }
        text.push_char(c);
        is_first = false;
    }
}

pub(super) fn trim_right_tendril_space(s: &mut StrTendril) {
    while !s.is_empty() && s.ends_with(' ') {
        s.pop_back(1);
    }
}

pub(super) fn join_tendril_strings(seq: &[StrTendril], sep: &str) -> StrTendril {
    let mut result = StrTendril::new();
    let mut iter = seq.iter();

    if let Some(first) = iter.next() {
        result.push_tendril(first);
    }

    for tendril in iter {
        result.push_slice(sep);
        result.push_tendril(tendril);
    }
    result
}

pub(super) fn add_linebreaks(text: &mut StrTendril, linebreak: &str, end: &str) {
    trim_right_tendril_space(text);
    while !text.ends_with(&end) {
        text.push_slice(linebreak);
    }
}

/// Keep only the first whitespace‑delimited token and a conservative set of characters.
pub(super) fn sanitize_attr_value(raw: &str) -> String {
    let token = raw.split_ascii_whitespace().next().unwrap_or("");
    token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.' | '#'))
        .collect()
}

#[cfg(test)]
mod tests {

    use tendril::StrTendril;

    use super::*;

    #[test]
    fn test_escape_text() {
        let t = r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|";
        let mut text = StrTendril::new();
        // mid-line: only characters with Markdown meaning anywhere are escaped
        push_normalized_text(&mut text, t, true);
        assert_eq!(
            text.as_ref(),
            r"Some text: x \`y\` \*z\* \_w\_ \[v\] \<u> #h >q -l +m !i . .5 5. \|q\|"
        );

        // at the beginning of a line, block-starting characters are escaped too
        let mut text = StrTendril::new();
        for word in ["#h", ">q", "-l", "+m", "2024.", "5)", "."] {
            push_escaped_chunk(&mut text, word, true, true);
            text.push_char(' ');
        }
        text.pop_back(1);
        assert_eq!(text.as_ref(), r"\#h \>q \-l \+m 2024\. 5\) .");

        // escape: false is used for inline code content, where backslash
        // escapes have no effect, so the content is emitted as is
        let mut text = StrTendril::new();
        push_normalized_text(&mut text, t, false);
        assert_eq!(
            text.as_ref(),
            r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|"
        );
    }
}
