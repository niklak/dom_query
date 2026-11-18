/// The output text. Only this type writes line breaks, and each new line gets the prefix of
/// the quotes and items around it before its first character.
pub(super) struct Lines {
    text: String,
    prefix: String,
    /// The length of the prefix an empty line takes: up to the last quote.
    blank_prefix_len: usize,
    /// The prefix is due before the next character; a line that ends first takes only
    /// `blank_prefix_len` of it.
    prefix_due: bool,
    /// Where the content of the last line starts, if known: a fragment may start mid-line.
    line_start: Option<usize>,
    /// A table cell joins its lines with `<br>` and indents only marker lines.
    in_cell: bool,
}

/// How much of the prefix was there before a quote or item added its part.
#[derive(Clone, Copy, Default)]
pub(super) struct PrefixMark {
    len: usize,
    blank_prefix_len: usize,
}

impl Lines {
    pub(super) const fn new(text: String, in_cell: bool) -> Self {
        Self {
            text,
            prefix: String::new(),
            blank_prefix_len: 0,
            prefix_due: false,
            line_start: None,
            in_cell,
        }
    }

    #[inline]
    pub(super) fn as_str(&self) -> &str {
        &self.text
    }

    pub(super) fn into_string(self) -> String {
        self.text
    }

    pub(super) fn take(&mut self) -> String {
        self.prefix_due = false;
        self.line_start = None;
        std::mem::take(&mut self.text)
    }

    #[inline]
    pub(super) fn push_str(&mut self, s: &str) {
        debug_assert!(!s.contains('\n'), "a line break goes through `new_line`");
        if !s.is_empty() {
            self.write_prefix();
            self.text.push_str(s);
        }
    }

    #[inline]
    pub(super) fn push(&mut self, c: char) {
        debug_assert!(c != '\n', "a line break goes through `new_line`");
        self.write_prefix();
        self.text.push(c);
    }

    /// Lets `change` edit the end of the current line, given where its content starts (0 if
    /// unknown); it must not break the line.
    #[inline]
    pub(super) fn edit_line(&mut self, change: impl FnOnce(&mut String, usize)) {
        self.write_prefix();
        let len = self.text.len();
        change(&mut self.text, self.line_start.unwrap_or(0));
        debug_assert!(!self.text.get(len..).unwrap_or_default().contains('\n'));
    }

    /// Pushes `s`, breaking its lines with `new_line`.
    #[inline]
    pub(super) fn push_lines(&mut self, s: &str) {
        let mut lines = s.split('\n');
        self.push_str(lines.next().unwrap_or_default());
        for line in lines {
            self.new_line();
            self.push_str(line);
        }
    }

    /// Pushes the lines of a block after its first, each on a line of its own.
    pub(super) fn push_following_lines(&mut self, rest: &str) {
        if self.prefix.is_empty() && !self.in_cell {
            // no prefix to add and no `<br>` to join with: append all the lines at once
            self.text.push('\n');
            self.text.push_str(rest);
            let last = rest.rsplit('\n').next().unwrap_or_default();
            self.prefix_due = last.is_empty();
            self.line_start = Some(self.text.len() - last.len());
            return;
        }
        for line in rest.split('\n') {
            self.new_line();
            self.push_str(line);
        }
    }

    pub(super) fn new_line(&mut self) {
        if self.in_cell {
            self.write_prefix();
            self.text.push_str("<br>");
            return;
        }
        if std::mem::take(&mut self.prefix_due) {
            let blank = self.prefix.get(..self.blank_prefix_len).unwrap_or_default();
            self.text.push_str(blank);
        }
        self.text.push('\n');
        self.prefix_due = true;
    }

    /// Ends the line, and leaves the next empty.
    pub(super) fn blank_line(&mut self) {
        self.new_line();
        self.new_line();
    }

    /// Ends the line with a hard break.
    pub(super) fn hard_break(&mut self) {
        self.push_str("  ");
        if self.in_cell {
            self.text.push_str("<br>");
        } else {
            self.text.push('\n');
        }
        self.prefix_due = true;
    }

    /// The prefix goes before the next character, as on a line after a line break.
    #[inline]
    pub(super) const fn prefix_next(&mut self) {
        self.prefix_due = true;
    }

    #[inline]
    fn write_prefix(&mut self) {
        if self.prefix_due {
            self.write_due_prefix();
        }
    }

    #[cold]
    fn write_due_prefix(&mut self) {
        self.prefix_due = false;
        self.text.push_str(&self.prefix);
        self.line_start = Some(self.text.len());
    }

    /// The content of the line starts here, after the markers written.
    #[inline]
    pub(super) fn mark_line_start(&mut self) {
        self.write_prefix();
        self.line_start = Some(self.text.len());
    }

    /// Whether nothing is written yet on a line.
    #[inline]
    pub(super) fn at_line_start(&self) -> bool {
        self.prefix_due || self.line_start == Some(self.text.len())
    }

    pub(super) fn prefix_mark(&self) -> PrefixMark {
        PrefixMark {
            len: self.prefix.len(),
            blank_prefix_len: self.blank_prefix_len,
        }
    }

    pub(super) fn restore_prefix(&mut self, mark: PrefixMark) {
        self.prefix.truncate(mark.len);
        self.blank_prefix_len = mark.blank_prefix_len;
    }

    /// Writes a quote's `> `, which every line after it starts with too.
    pub(super) fn push_quote(&mut self) {
        self.push_str("> ");
        self.prefix.push_str("> ");
        self.blank_prefix_len = self.prefix.len();
    }

    /// Writes an item's marker after `indent` spaces; the lines after it start with as many
    /// spaces as both take.
    pub(super) fn push_marker(&mut self, indent: usize, marker: &str, width: usize) {
        self.write_prefix();
        self.text.extend(std::iter::repeat_n(' ', indent));
        self.text.push_str(marker);
        self.prefix.extend(std::iter::repeat_n(' ', indent + width));
    }
}
