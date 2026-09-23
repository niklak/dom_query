use super::constants::{ALWAYS_ESCAPED, LINE_START_ESCAPED};

pub(super) fn push_normalized_text(text: &mut String, new_text: &str, escape: bool) {
    let follows_newline = text.ends_with(['\n', ' ']) || text.is_empty();
    // a space mid-line does not make block syntax (`#`, `>`, ...) significant;
    // only an actual line start does. The indentation plus open list marker a
    // list item was just written with counts as one, or `1. a` inside an item
    // would serialize as a nested list and `#` as a heading.
    let is_line_start = at_block_start(text);
    let push_start_whitespace = !follows_newline && new_text.starts_with(char::is_whitespace);
    let push_end_whitespace = new_text.ends_with(char::is_whitespace);

    let mut result = String::with_capacity(new_text.len());
    let mut iter = new_text.split_whitespace();

    if let Some(first) = iter.next() {
        if push_start_whitespace {
            result.push(' ');
        } else if escape && first.starts_with(['.', ')']) && ends_with_marker_digits(text) {
            // `<span>1</span>. foo`: the digits already written at the line
            // start and this `.` would form an ordered-list marker
            result.push('\\');
        }
        // only the first word of a text node can continue a Markdown line
        push_escaped_chunk(&mut result, first, escape, is_line_start);
        for word in iter {
            result.push(' ');
            push_escaped_chunk(&mut result, word, escape, false);
        }
    }
    if result.is_empty() && follows_newline {
        return;
    }

    text.push_str(&result);

    if push_end_whitespace && !text.ends_with(char::is_whitespace) {
        text.push(' ');
    }
}

/// True when the current line holds only indentation, optionally followed by
/// a list marker opened on this line (`- `, `+ `, `12. `). Block syntax
/// characters (`#`, `>`, markers) are significant at that position.
///
/// Scans back from the end of `text` and stops at the first byte that rules
/// the position out: looking at the whole current line for every text node
/// made long lines quadratic.
fn at_block_start(text: &str) -> bool {
    let bytes = text.as_bytes();
    // the marker, if any, is the last word and is followed by one space
    let mut end = bytes.len();
    if bytes.last() == Some(&b' ') {
        end -= 1;
    }
    // an ordered marker has at most nine digits and its `.`/`)`
    let window = end.saturating_sub(10);
    let marker_start = match bytes[window..end]
        .iter()
        .rposition(|&b| b == b' ' || b == b'\n')
    {
        Some(i) => window + i + 1,
        None if window == 0 => 0,
        None => return false,
    };
    let marker = &text[marker_start..end];
    let is_marker =
        end < bytes.len() && (matches!(marker, "-" | "+") || is_ordered_list_marker(marker));
    if !is_marker && !marker.is_empty() {
        return false;
    }
    // everything before, back to the line start, must be indentation
    bytes[..marker_start]
        .iter()
        .rev()
        .take_while(|&&b| b != b'\n')
        .all(|&b| b == b' ')
}

/// True when the current line is a line start followed by one to nine
/// digits, which a following `.` or `)` turns into an ordered-list marker.
fn ends_with_marker_digits(text: &str) -> bool {
    let digits = text
        .bytes()
        .rev()
        .take(10)
        .take_while(u8::is_ascii_digit)
        .count();
    (1..=9).contains(&digits) && at_block_start(&text[..text.len() - digits])
}

/// An ordered-list marker like `3.` or `12)` at the beginning of a line would
/// otherwise turn the line into a list item.
fn is_ordered_list_marker(chunk: &str) -> bool {
    let Some(stem) = chunk.strip_suffix(['.', ')']) else {
        return false;
    };
    !stem.is_empty() && stem.as_bytes().iter().all(u8::is_ascii_digit)
}

pub(super) fn push_escaped_chunk(text: &mut String, chunk: &str, escape: bool, line_start: bool) {
    if !escape {
        // inline code content, where backslash escapes are not interpreted
        text.push_str(chunk);
        return;
    }
    let list_marker = line_start && is_ordered_list_marker(chunk);
    let mut chars = chunk.chars().peekable();
    let mut is_first = true;
    while let Some(c) = chars.next() {
        // `~` is escaped anywhere too: it opens a code fence at a line start
        // (`~~~`) and marks strikethrough in GitHub Flavored Markdown
        let escaped = if ALWAYS_ESCAPED.contains(&c) || c == '~' {
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
            text.push('\\');
        }
        text.push(c);
        is_first = false;
    }
}

pub(super) fn trim_right_tendril_space(s: &mut String) {
    s.truncate(s.trim_end_matches(' ').len());
}

pub(super) fn join_tendril_strings(seq: &[String], sep: &str) -> String {
    let mut result = String::new();
    let mut iter = seq.iter();

    if let Some(first) = iter.next() {
        result.push_str(first);
    }

    for tendril in iter {
        result.push_str(sep);
        result.push_str(tendril);
    }
    result
}

pub(super) fn add_linebreaks(text: &mut String, linebreak: &str, end: &str) {
    trim_right_tendril_space(text);
    while !text.ends_with(&end) {
        text.push_str(linebreak);
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

    use super::*;

    #[test]
    fn test_escape_text() {
        let t = r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|";
        let mut text = String::new();
        // mid-line: only characters with Markdown meaning anywhere are escaped
        push_normalized_text(&mut text, t, true);
        assert_eq!(
            text,
            r"Some text: x \`y\` \*z\* \_w\_ \[v\] \<u> #h >q -l +m !i . .5 5. \|q\|"
        );

        // at the beginning of a line, block-starting characters are escaped too
        let mut text = String::new();
        for word in ["#h", ">q", "-l", "+m", "2024.", "5)", "."] {
            push_escaped_chunk(&mut text, word, true, true);
            text.push(' ');
        }
        text.pop();
        assert_eq!(text, r"\#h \>q \-l \+m 2024\. 5\) .");

        // escape: false is used for inline code content, where backslash
        // escapes have no effect, so the content is emitted as is
        let mut text = String::new();
        push_normalized_text(&mut text, t, false);
        assert_eq!(
            text,
            r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|"
        );
    }

    #[test]
    fn test_at_block_start() {
        for text in [
            "",
            "\n",
            "a\n",
            "    ",
            "a\n  ",
            "- ",
            "+ ",
            "12. ",
            "3) ",
            "  - ",
            "x\n    1. ",
            "999999999. ",
        ] {
            assert!(at_block_start(text), "{text:?}");
        }
        for text in [
            "a",
            "a ",
            "-",
            "- a",
            "-  ",
            "a - ",
            "1.",
            "1.x ",
            "x1. ",
            "1000000000. ",
            "a\n  b ",
            "中文 ",
        ] {
            assert!(!at_block_start(text), "{text:?}");
        }
    }
}
