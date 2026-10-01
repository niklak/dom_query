use std::borrow::Cow;

use super::constants::{ALWAYS_ESCAPED, LINE_START_ESCAPED};
use super::opts::FormatOpts;

#[allow(clippy::cast_possible_truncation)]
pub(super) fn push_normalized_text(text: &mut String, new_text: &str, f_opts: FormatOpts) {
    if !text.ends_with(['\n', ' '])
        && !new_text.is_empty()
        && new_text.chars().all(char::is_whitespace)
    {
        text.push(' ');
        return;
    }

    let follows_newline = text.ends_with(['\n', ' ']) || text.is_empty();
    let push_start_whitespace =
        (f_opts.inline || !follows_newline) && new_text.starts_with(char::is_whitespace);
    let push_end_whitespace = new_text.ends_with(char::is_whitespace);

    let mut result = String::with_capacity(new_text.len());
    let mut iter = new_text.split_whitespace();

    if let Some(first) = iter.next() {
        if push_start_whitespace {
            result.push(' ');
        }
        let escape = !f_opts.skip_escape;
        // only the first word of a text node can continue a Markdown line
        let is_line_start = text.is_empty() || text.ends_with('\n');
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

/// An ordered-list marker like `3.` or `12)` at the beginning of a line would
/// otherwise turn the line into a list item.
fn is_ordered_list_marker(chunk: &str) -> bool {
    let Some(stem) = chunk.strip_suffix(['.', ')']) else {
        return false;
    };
    !stem.is_empty() && stem.as_bytes().iter().all(u8::is_ascii_digit)
}

pub(super) fn push_escaped_chunk(
    text: &mut String,
    chunk: &str,
    escape: bool,
    line_start: bool,
) {
    if !escape {
        // inline code content, where backslash escapes are not interpreted;
        // backticks are escaped so they cannot terminate the code span
        for c in chunk.chars() {
            if c == '`' {
                text.push('\\');
            }
            text.push(c);
        }
        return;
    }

    let list_marker = line_start && is_ordered_list_marker(chunk);
    let mut chars = chunk.chars().peekable();
    let mut is_first = true;
    while let Some(c) = chars.next() {
        let escaped = if ALWAYS_ESCAPED.contains(&c) {
            true
        } else if LINE_START_ESCAPED.contains(&c) {
            // only the first character of the chunk can open a block construct
            line_start && is_first
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

pub(super) fn push_emphasis(acc: &mut String, emphasis_content: &mut String, marker: &str) {
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

    acc.push_str(emphasis_content);
}

#[allow(clippy::cast_possible_truncation)]
pub(super) fn try_trim_prev_emphasis_marker(acc: &mut String, marker: &str) -> bool {
    if marker.is_empty() || acc.is_empty() {
        return false;
    }

    let has_space = acc.ends_with(' ');
    let trimmed = acc.trim_ascii_end();
    let trailing = &acc[trimmed.len()..];

    if (trailing.is_empty() || trailing == " ") && is_exact_marker_end(trimmed, marker) {
        let trim_bytes = marker.len()  + usize::from(has_space);
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

pub(super) fn escape_md_url<'a>(dest: &'a str) -> Cow<'a, str> {
    if md_link_needs_wrap(dest) {
        let mut out = String::with_capacity(dest.len() + 4);
        out.push('<');
        for c in dest.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '<' => out.push_str("\\<"),
                '>' => out.push_str("\\>"),
                '\n' => out.push_str("%0A"),
                '\r' => out.push_str("%0D"),
                c => out.push(c),
            }
        }
        out.push('>');
        Cow::Owned(out)
    } else if dest.contains('\\') {
        Cow::Owned(dest.replace('\\', "\\\\"))
    } else {
        Cow::Borrowed(dest)
    }
}

fn md_link_needs_wrap(dest: &str) -> bool {
    let mut balance: i32 = 0;
    for c in dest.chars() {
        match c {
            '(' => balance += 1,
            ')' => {
                balance -= 1;
                if balance < 0 {
                    return true;
                }
            }
            ' ' | '<' => return true,
            c if c.is_ascii_control() => return true,
            _ => {}
        }
    }
    balance != 0
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_escape_text() {
        let t = r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|";
        let mut text = String::new();
        // mid-line: only characters with Markdown meaning anywhere are escaped
        push_normalized_text(&mut text, t, FormatOpts::new());
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

        // escape: false is used for inline code content: only backticks are
        // escaped so they cannot terminate the code span
        let mut text = String::new();
        push_normalized_text(&mut text, t, FormatOpts::new().skip_escape());
        assert_eq!(
            text,
            r"Some text: x \`y\` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|"
        );
    }
}
