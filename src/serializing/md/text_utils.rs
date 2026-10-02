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

pub(super) fn push_escaped_chunk(text: &mut String, chunk: &str, escape: bool, line_start: bool) {
    if !escape {
        // inline code content, where backslash escapes are not interpreted;
        text.push_str(chunk);
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

pub(super) fn trim_trailing_space(s: &mut String) {
    s.truncate(s.trim_end_matches(' ').len());
}

pub(super) fn add_linebreaks(text: &mut String, linebreak: &str, end: &str) {
    trim_trailing_space(text);
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

/// Formats a Markdown link destination and writes it directly to the buffer.
pub(super) fn push_md_url(text: &mut String, dest: &str) {
    let mut balance: i32 = 0;
    let mut needs_wrap = false;
    let mut has_backslash = false;

    for &b in dest.as_bytes() {
        match b {
            b'(' => balance += 1,
            b')' => {
                balance -= 1;
                if balance < 0 {
                    needs_wrap = true;
                    break;
                }
            }
            b' ' | b'<' | 0..=31 | 127 => {
                needs_wrap = true;
                break;
            }
            b'\\' => has_backslash = true,
            _ => {}
        }
    }

    if needs_wrap || balance != 0 {
        text.reserve(dest.len() + 4);
        text.push('<');
        for c in dest.chars() {
            match c {
                '\\' => text.push_str("\\\\"),
                '<' => text.push_str("\\<"),
                '>' => text.push_str("\\>"),
                '\n' => text.push_str("%0A"),
                '\r' => text.push_str("%0D"),
                c => text.push(c),
            }
        }
        text.push('>');
    } else if has_backslash {
        text.push_str(&dest.replace('\\', "\\\\"));
    } else {
        text.push_str(dest);
    }
}

pub(super) fn push_title(text: &mut String, title: &str) {
    text.push_str(" \"");
    if title.contains('"') {
        let mut normalized = String::with_capacity(title.len());
        push_normalized_text(&mut normalized, title, FormatOpts::new());
        text.push_str(&normalized.replace('"', "\\\""));
    } else {
        push_normalized_text(text, title, FormatOpts::new());
    }
    text.push('"');
}

pub(super) fn max_backtick_run(text: &str) -> usize {
    let mut max = 0;
    let mut current = 0;
    for b in text.as_bytes() {
        current = if *b == b'`' { current + 1 } else { 0 };
        max = max.max(current);
    }
    max
}

pub(super) fn push_code_text(text: &mut String, code_text: &str) {
    if code_text.is_empty() {
        return;
    }

    // a code span right before would merge its closing run with this
    // opening one: `x` and `y` would give `x``y`, a single span
    if text.ends_with('`') {
        text.push(' ');
    }

    let backtick_run = max_backtick_run(code_text);

    if backtick_run == 0 {
        text.push('`');
        text.push_str(code_text);
        text.push('`');
    } else {
        let fence_len = backtick_run + 1;
        let bytes = code_text.as_bytes();
        let needs_space = bytes.first() == Some(&b'`') || bytes.last() == Some(&b'`');

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
}

pub(super) fn trim_space(s: &mut String) {
    s.truncate(s.trim_end().len());
    s.drain(..s.len() - s.trim_start().len());
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
            r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|"
        );
    }
}
