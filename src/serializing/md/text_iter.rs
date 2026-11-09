use memchr::memchr;

/// Splits Markdown text into paragraphs at blank lines, collapsing runs.
/// Edge blank lines keep the same parts as `split("\n\n")`.
pub(super) struct Paragraphs<'a> {
    s: &'a str,
    pos: usize,
    done: bool,
    trailing_empty: bool,
}

impl<'a> Paragraphs<'a> {
    pub(super) fn new(s: &'a str) -> Self {
        // a leading spaces-only line is blank: drop its spaces, keep its newline
        let pos = s
            .find('\n')
            .filter(|&end| s[..end].bytes().all(|b| b == b' '))
            .unwrap_or(0);
        Self {
            s,
            pos,
            done: false,
            trailing_empty: false,
        }
    }
}

impl<'a> Iterator for Paragraphs<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.done {
            return None;
        }
        if self.trailing_empty {
            self.trailing_empty = false;
            self.done = true;
            return Some("");
        }

        let bytes = self.s.as_bytes();
        let len = bytes.len();
        let start = self.pos;
        let mut cursor = start;

        while let Some(off) = memchr(b'\n', &bytes[cursor..]) {
            cursor += off;
            let mut after = cursor + 1;
            while after < len && bytes[after] == b' ' {
                after += 1;
            }
            if after < len && bytes[after] == b'\n' {
                // skip the blank line and any following blank lines
                self.pos = after + 1;
                while self.pos < len {
                    let mut line = self.pos;
                    while line < len && bytes[line] == b' ' {
                        line += 1;
                    }
                    if line < len && bytes[line] == b'\n' {
                        self.pos = line + 1;
                    } else {
                        break;
                    }
                }
                // a blank run at the end leaves an empty last part
                self.trailing_empty = self.pos >= len;
                return Some(&self.s[start..cursor]);
            }
            cursor += 1;
        }

        self.done = true;
        let part = &self.s[start..];

        // a final spaces-only line is blank: keep its newline, drop the spaces
        let tail = part.rfind('\n').map_or(0, |k| k + 1);
        if part[tail..].bytes().all(|b| b == b' ') {
            if tail == 0 {
                Some("")
            } else {
                Some(&part[..tail])
            }
        } else {
            Some(part)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Paragraphs;

    #[test]
    fn splits_at_blank_lines() {
        let cases = [
            ("a\n\nb", &["a", "b"][..]),
            ("a\n  \n  \nb", &["a", "b"]),
            ("a  \n  \n", &["a  ", ""]),
            ("  \n  \nb", &["", "b"]),
            (" \nb\n\nc", &["\nb", "c"]),
            ("a\n", &["a\n"]),
            ("", &[""]),
        ];
        for (text, expected) in cases {
            assert_eq!(Paragraphs::new(text).collect::<Vec<_>>(), expected);
        }
    }
}
