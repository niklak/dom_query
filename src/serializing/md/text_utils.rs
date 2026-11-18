use memchr::memchr3;
use std::borrow::Cow;

use super::ctx::FormatOpts;

/// A `line` for `push_normalized_text` that `text` never reaches: no word is at a line start.
pub(super) const NEVER_AT_LINE_START: usize = usize::MAX;

/// Pushes the words of `new_text`, escaped; `line` is where the last line of `text` starts.
pub(super) fn push_normalized_text(
    text: &mut String,
    line: usize,
    new_text: &str,
    f_opts: FormatOpts,
) {
    let mut space = false;
    push_words(text, line, new_text, f_opts, &mut space);
    if space {
        text.push(' ');
    }
}

/// `push_normalized_text`, but a space at the end is owed in `space` instead of written, and
/// one owed before is written only if words follow.
pub(super) fn push_words(
    text: &mut String,
    line: usize,
    new_text: &str,
    f_opts: FormatOpts,
    space: &mut bool,
) {
    if new_text.trim_ascii().is_empty() {
        *space |= !new_text.is_empty() && !text.ends_with(['\n', ' ']);
        return;
    }
    if std::mem::take(space) {
        text.push(' ');
    }

    // a link body buffer starts after its `[`, so it keeps its leading space
    let follows_newline = text.ends_with(['\n', ' ']) || (text.is_empty() && !f_opts.link_body);
    // a fragment keeps a leading space for its parent to place, but not past a marker or break
    let line_start = text.len() == line && !text.is_empty();
    let push_start_whitespace = (f_opts.inline || !follows_newline)
        && !line_start
        && new_text.starts_with(|c: char| c.is_ascii_whitespace());
    let push_end_whitespace = new_text.ends_with(|c: char| c.is_ascii_whitespace());

    let escape = !f_opts.skip_escape;
    let mut iter = new_text.split_ascii_whitespace();

    let Some(first) = iter.next() else { return };

    // only the first word of a text node can continue a Markdown line, also after spaces that
    // a block drops; a link body sits after its `[`, never at a line start
    let is_line_start = !f_opts.link_body
        && text
            .get(line..)
            .is_some_and(|s| s.bytes().all(|b| b == b' '));
    if push_start_whitespace {
        text.push(' ');
    } else if escape && matches!(first, "." | ")") && ends_with_marker_digits(text, line) {
        // `<span>1</span>. foo`: the digits already written at the line
        // start and this `.` would form an ordered-list marker
        text.push('\\');
    }
    push_escaped_chunk(text, first, escape, is_line_start);
    for word in iter {
        text.push(' ');
        push_escaped_chunk(text, word, escape, false);
    }

    *space = push_end_whitespace && !text.ends_with(|c: char| c.is_ascii_whitespace());
}

/// True when `text` ends with one to nine digits that start the line at `line`, so a
/// following `.` or `)` would make an ordered-list marker.
fn ends_with_marker_digits(text: &str, line: usize) -> bool {
    // CommonMark ordered-list markers have one to nine digits
    let stem = text.trim_end_matches(|c: char| c.is_ascii_digit());
    if !(1..=9).contains(&(text.len() - stem.len())) {
        return false;
    }
    // the digits start the line when only indentation precedes them
    stem.trim_end_matches(' ').len() <= line
}

/// An ordered-list marker like `3.` or `12)` at the beginning of a line would
/// otherwise turn the line into a list item.
fn is_ordered_list_marker(chunk: &str) -> bool {
    chunk
        .strip_suffix(['.', ')'])
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Appends `content` to `out`, putting a backslash before every `|`.
/// With `skip_escaped`, a `|` that already follows a backslash is left alone.
pub(super) fn push_escaped_pipes(out: &mut String, content: &str, skip_escaped: bool) -> bool {
    let mut pieces = content.split('|');
    let mut prev = pieces.next().unwrap_or_default();
    out.push_str(prev);
    let mut changed = false;
    for piece in pieces {
        if skip_escaped && prev.ends_with('\\') {
            out.push('|');
        } else {
            out.push_str("\\|");
            changed = true;
        }
        out.push_str(piece);
        prev = piece;
    }
    changed
}

pub(super) fn push_escaped_chunk(text: &mut String, chunk: &str, escape: bool, line_start: bool) {
    if !escape {
        text.push_str(chunk);
        return;
    }

    let list_marker = line_start && is_ordered_list_marker(chunk);
    let bytes = chunk.as_bytes();
    // a first word of only `=` could underline the previous line into a setext heading
    let setext_underline = line_start && bytes.iter().all(|&b| b == b'=');
    let mut last = 0;

    text.reserve(chunk.len());

    for (i, &b) in bytes.iter().enumerate() {
        if b == b'&' && starts_char_reference(bytes.get(i + 1..).unwrap_or_default()) {
            text.push_str(chunk.get(last..i).unwrap_or_default());
            text.push_str("&amp;");
            last = i + 1;
            continue;
        }
        let escaped = if is_byte_always_escaped(b) {
            true
        } else if is_byte_escaped_at_line_start(b) {
            line_start && i == 0
        } else if b == b'!' {
            bytes.get(i + 1) == Some(&b'[')
        } else if b == b'.' || b == b')' {
            list_marker
        } else if b == b'=' {
            setext_underline && i == 0
        } else {
            false
        };

        if escaped {
            text.push_str(chunk.get(last..i).unwrap_or_default());
            text.push('\\');
            last = i;
        }
    }

    text.push_str(chunk.get(last..).unwrap_or_default());
}

/// True when an ampersand is followed by a character reference a Markdown
/// renderer would decode, so it must be written as `&amp;` to stay literal.
fn starts_char_reference(rest: &[u8]) -> bool {
    // prose `&` is usually followed by a space or punctuation: reject it at once
    if !matches!(rest.first(), Some(b'#' | b'a'..=b'z' | b'A'..=b'Z')) {
        return false;
    }
    // the longest HTML entity name is 31 characters, plus the semicolon
    let Some(semi) = rest.iter().take(32).position(|&b| b == b';') else {
        return false;
    };
    let name = rest.get(..semi).unwrap_or_default();
    if let [b'#', b'x' | b'X', digits @ ..] = name {
        return !digits.is_empty() && digits.iter().all(u8::is_ascii_hexdigit);
    }
    if let [b'#', digits @ ..] = name {
        return !digits.is_empty() && digits.iter().all(u8::is_ascii_digit);
    }
    // the first byte is alphabetic (checked above), and named references are alphanumeric
    name.iter().all(u8::is_ascii_alphanumeric)
}

/// Escapes a trailing `!` so the `[` written right after it
/// cannot turn the text into image syntax (`Wow![x](u)`).
pub(super) fn escape_trailing_bang(text: &mut String) {
    if text.ends_with('!') {
        text.pop();
        text.push_str("\\!");
    }
}

pub(super) fn trim_trailing_space(s: &mut String) {
    s.truncate(s.trim_end_matches(' ').len());
}

/// Formats a Markdown link destination and writes it directly to the buffer.
pub(super) fn push_md_url(text: &mut String, dest: &str) {
    // unclosed `(` so far
    let mut open: usize = 0;
    let mut has_backslash = false;

    let dest = &html_escape(dest);
    // a bare empty destination would read a following title as the destination
    let mut needs_wrap = dest.is_empty();

    for &b in dest.as_bytes() {
        match b {
            b'(' => open += 1,
            b')' => {
                let Some(rest) = open.checked_sub(1) else {
                    needs_wrap = true;
                    break;
                };
                open = rest;
            }
            b' ' | b'<' | 0..=31 | 127 => {
                needs_wrap = true;
                break;
            }
            b'\\' => has_backslash = true,
            _ => {}
        }
    }

    if needs_wrap || open != 0 {
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
        push_normalized_text(&mut normalized, 0, title, FormatOpts::new());
        text.push_str(&normalized.replace('"', "\\\""));
    } else {
        push_normalized_text(text, NEVER_AT_LINE_START, title, FormatOpts::new());
    }
    text.push('"');
}

pub(super) fn push_block_space(text: &mut String) {
    if !text.is_empty() && !text.ends_with(|c: char| c.is_ascii_whitespace()) {
        text.push(' ');
    }
}

/// Escapes special characters for HTML attribute values (quotes, ampersands, and &nbsp;).
/// Note: `<` and `>` are intentionally skipped here.
pub fn html_escape(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    if memchr3(b'"', b'&', 0xC2, bytes).is_none() {
        return Cow::Borrowed(text);
    }
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(['"', '&', '\u{a0}']) {
        let (before, after) = rest.split_at_checked(i).unwrap_or((rest, ""));
        result.push_str(before);
        let mut chars = after.chars();
        result.push_str(match chars.next() {
            Some('&') => "&amp;",
            Some('"') => "&quot;",
            _ => "&nbsp;",
        });
        rest = chars.as_str();
    }
    result.push_str(rest);
    Cow::Owned(result)
}

/// Characters with Markdown meaning anywhere in a line, escaped unconditionally.
const fn is_byte_always_escaped(b: u8) -> bool {
    // "\\`*_[]<|~"
    matches!(
        b,
        b'\\' | b'`' | b'*' | b'_' | b'[' | b']' | b'<' | b'|' | b'~'
    )
}

///// Characters that can only start a Markdown block at the beginning of a line
const fn is_byte_escaped_at_line_start(b: u8) -> bool {
    matches!(b, b'#' | b'>' | b'-' | b'+')
}

#[cfg(test)]
#[allow(clippy::string_slice, clippy::indexing_slicing)]
mod tests {

    use super::*;

    #[test]
    fn test_escape_text() {
        let t = r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|";
        let mut text = String::new();
        // mid-line: only characters with Markdown meaning anywhere are escaped
        push_normalized_text(&mut text, 0, t, FormatOpts::new());
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

        // a leading space written inline does not hide the line start
        let mut text = String::new();
        push_normalized_text(&mut text, 0, " # x", FormatOpts::new().inline());
        assert_eq!(text, r" \# x");

        // escape: false is used for inline code content: only backticks are
        // escaped so they cannot terminate the code span
        let mut text = String::new();
        push_normalized_text(&mut text, 0, t, FormatOpts::new().skip_escape());
        assert_eq!(
            text,
            r"Some text: x `y` *z* _w_ [v] <u> #h >q -l +m !i . .5 5. |q|"
        );
    }

    #[test]
    fn test_starts_char_reference() {
        assert!(starts_char_reference(b"ouml;"));
        assert!(starts_char_reference(b"#246;"));
        assert!(starts_char_reference(b"#x26;"));
        assert!(starts_char_reference(b"#X26;"));
        // the semicolon falls outside the longest possible entity name
        assert!(!starts_char_reference(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;"));
        // empty or invalid bodies
        assert!(!starts_char_reference(b"#;"));
        assert!(!starts_char_reference(b"#x;"));
        assert!(!starts_char_reference(b"#xG;"));
        assert!(!starts_char_reference(b"#2x6;"));
        // no opening name/number or leading whitespace
        assert!(!starts_char_reference(b""));
        assert!(!starts_char_reference(b";"));
        assert!(!starts_char_reference(b" ouml;"));
    }

    #[test]
    fn test_html_escape_borrows_plain_text() {
        assert!(matches!(html_escape("/a?b=1"), Cow::Borrowed("/a?b=1")));
    }

    #[test]
    fn test_html_escape() {
        struct TestCase {
            name: &'static str,
            input: &'static str,
            expected: &'static str,
        }

        let test_cases = [
            // Empty string
            TestCase {
                name: "empty string",
                input: "",
                expected: "",
            },
            // No special characters
            TestCase {
                name: "no special chars",
                input: "hello_world_123",
                expected: "hello_world_123",
            },
            // Angle brackets untouched
            TestCase {
                name: "angle brackets untouched",
                input: "a < b and c > d",
                expected: "a < b and c > d",
            },
            // Quotes escaped
            TestCase {
                name: "quotes escaped",
                input: "Width: \"100%\"",
                expected: "Width: &quot;100%&quot;",
            },
            // Ampersand escaped
            TestCase {
                name: "ampersand escaped",
                input: "Tom & Jerry",
                expected: "Tom &amp; Jerry",
            },
            // Non-breaking space converted
            TestCase {
                name: "non-breaking space",
                input: "Hello\u{00A0}World",
                expected: "Hello&nbsp;World",
            },
            // Mixed attribute content
            TestCase {
                name: "mixed attribute content",
                input: "class=\"btn & primary\" title=\"1 < 2 & 3 > 2\"",
                expected: "class=&quot;btn &amp; primary&quot; title=&quot;1 < 2 &amp; 3 > 2&quot;",
            },
            // C2 byte without nbsp
            TestCase {
                name: "c2 byte without nbsp",
                input: "Test\u{0080}Value",
                expected: "Test\u{0080}Value",
            },
            TestCase {
                name: "amp in url",
                input: "/?q=&copy;",
                expected: "/?q=&amp;copy;",
            },
            TestCase {
                name: "amp in url",
                input: "/?a=1&b=2",
                expected: "/?a=1&amp;b=2",
            },
        ];

        for tc in test_cases {
            let actual = html_escape(tc.input);
            assert_eq!(
                actual, tc.expected,
                "Failed test case: '{}' (input: {:?})",
                tc.name, tc.input
            );
        }
    }
}
