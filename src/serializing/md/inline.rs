use super::ctx::FormatOpts;
use super::elements::code::ends_with_code_span;
use super::elements::emphasis::{Emphasis, starts_with_hard_break};
use super::lines::Lines;
use super::text_utils::{
    escape_trailing_bang, push_block_space, push_md_url, push_title, push_words,
};

/// What a code span or an image shows: opaque, it counts as a word on both ends.
const WORD: char = '\u{fffc}';
/// What a run of another kind opens with, as far as the run before can tell: its delimiter
/// may fuse with that run's closing one, so it counts as a letter.
const OTHER_RUN: char = 'a';

/// Writes inline Markdown to `Lines`, and tracks the first and last characters a reader would
/// see. Markup like `**` or `](url)` is invisible and never counts as an end.
pub(super) struct InlineWriter {
    first_visible: Option<char>,
    /// A `<br>` counts as `'\n'`.
    last_visible: Option<char>,
    /// A word boundary came before any visible text: the parent decides it.
    boundary_before: bool,
    /// A word boundary came after the last visible text.
    boundary_after: bool,
    /// A space ends the text so far, written once more of the line follows.
    ending_space: bool,
    pending_run: Option<EmphasisRun>,
    written: LastWritten,
    /// An empty buffer for the next pending run.
    spare: String,
    /// Hard breaks owed, written once more of the line follows.
    breaks: usize,
}

/// Inline content finished by a writer of its own, which a parent places as one piece.
#[derive(Clone, Copy)]
pub(super) struct Fragment<'a> {
    pub(super) text: &'a str,
    pub(super) writer: &'a InlineWriter,
}

/// A finished emphasis run, written once what follows shows whether its delimiters fit.
struct EmphasisRun {
    kind: Emphasis,
    /// The content between the delimiters, without edge spaces or hard breaks.
    content: String,
    /// What the text ended with before the opening delimiter.
    start: LastWritten,
    /// Edge spaces, written after the closing delimiter.
    trail: String,
}

/// What the text ends with, as the flanking of an opening delimiter sees it.
#[derive(Clone, Copy)]
enum LastWritten {
    /// A line start or markup, which delimiters can sit next to.
    Edge,
    /// A source character.
    Char(char),
    /// The closing delimiter of a run of `kind`; `char_before` is the character before it, and
    /// `nested` whether a nested run's delimiter comes right before it.
    Closing {
        kind: Emphasis,
        char_before: Option<char>,
        nested: bool,
    },
}

/// One piece of inline output; `InlineWriter::push` writes it.
pub(super) enum Inline<'a> {
    Text(&'a str, FormatOpts),
    /// Shows nothing but keeps the words around it apart: an `<svg>` or `<option>`, or a block
    /// inside code. It becomes a space only when both neighbours need one.
    WordBoundary,
    /// A `<br>` tag, where a hard break could leave a blank line (link text).
    HtmlBreak,
    Code(&'a str),
    Image(&'a str),
    /// Its markup goes around the content once, so the content is finished first.
    Emphasis {
        kind: Emphasis,
        content: Fragment<'a>,
    },
    /// `[label](href "title")`; like emphasis, the label is finished first.
    Link {
        label: Fragment<'a>,
        href: &'a str,
        title: Option<&'a str>,
    },
}

impl InlineWriter {
    pub(super) const fn new() -> Self {
        Self {
            first_visible: None,
            last_visible: None,
            boundary_before: false,
            boundary_after: false,
            ending_space: false,
            pending_run: None,
            written: LastWritten::Edge,
            spare: String::new(),
            breaks: 0,
        }
    }

    /// Writes a text node; with no word boundary, run or break waiting, the common case, it skips
    /// `push`.
    #[inline]
    pub(super) fn push_text(&mut self, out: &mut Lines, s: &str, opts: FormatOpts) {
        if self.boundary_after || self.pending_run.is_some() || self.breaks > 0 {
            return self.push(out, Inline::Text(s, opts));
        }
        let (Some(first), Some(last)) = (s.chars().next(), s.chars().next_back()) else {
            return;
        };
        self.push_words(out, s, opts);
        self.wrote(first, last, LastWritten::Char(last));
    }

    fn push_words(&mut self, out: &mut Lines, s: &str, opts: FormatOpts) {
        if self.at_line_start(out) && s.trim_ascii().is_empty() {
            return;
        }
        let space = &mut self.ending_space;
        out.edit_line(|text, line| push_words(text, line, s, opts, space));
    }

    /// Writes the space owed, before more of the line.
    pub(super) fn write_ending_space(&mut self, out: &mut Lines) {
        if std::mem::take(&mut self.ending_space) {
            out.push(' ');
            self.written = LastWritten::Char(' ');
        }
    }

    /// Drops the space owed, at a line end.
    pub(super) fn drop_ending_space(&mut self) {
        self.ending_space = false;
    }

    /// Writes `inline`, holding back what depends on what follows: a word boundary, a space, hard
    /// breaks, an emphasis run.
    pub(super) fn push(&mut self, out: &mut Lines, inline: Inline) {
        match inline {
            // waits for the next visible character; one before any belongs to the parent
            Inline::WordBoundary if self.first_visible.is_none() => self.boundary_before = true,
            Inline::WordBoundary => self.boundary_after = true,
            // whitespace is a space owed; after a pending run it stays at the run's edge, where a
            // run of its kind may merge with it
            Inline::Text(s, opts) if !s.is_empty() && s.trim_ascii().is_empty() => {
                if let Some(run) = &mut self.pending_run {
                    if !run.trail.ends_with([' ', '\n']) {
                        run.trail.push(' ');
                    }
                } else {
                    self.push_words(out, s, opts);
                }
                self.boundary_after = false;
                self.last_visible = Some(' ');
            }
            // text is judged by its source characters, before escaping
            Inline::Text(s, opts) => {
                let (Some(first), Some(last)) = (s.chars().next(), s.chars().next_back()) else {
                    return;
                };
                self.start_piece(out, first, Some(first));
                self.push_words(out, s, opts);
                self.wrote(first, last, LastWritten::Char(last));
            }
            // a `<br>` is whitespace, so a word boundary before it writes no space of its own
            Inline::HtmlBreak => {
                self.settle(out, Some('\n'));
                self.ending_space = false;
                self.start_piece(out, '\n', Some('\n'));
                out.push_str("<br>");
                self.wrote('\n', '\n', LastWritten::Edge);
            }
            // opaque words; a code span right after another would merge their backticks
            Inline::Code(span) => {
                if !self.start_piece(out, WORD, Some('`')) && ends_with_code_span(out.as_str()) {
                    out.push(' ');
                }
                out.push_str(span);
                self.wrote(WORD, WORD, LastWritten::Edge);
            }
            Inline::Image(md) => {
                self.start_piece(out, WORD, Some('!'));
                out.push_str(md);
                self.wrote(WORD, WORD, LastWritten::Edge);
            }
            // a run that writes nothing passes its word boundaries on
            Inline::Emphasis { content: run, .. } | Inline::Link { label: run, .. }
                if run.text.is_empty() =>
            {
                if run.writer.boundary_before || run.writer.boundary_after {
                    self.push(out, Inline::WordBoundary);
                }
            }
            Inline::Emphasis { kind, content } => {
                if content.writer.boundary_before {
                    self.push(out, Inline::WordBoundary);
                }
                let first = content.writer.first_visible.unwrap_or(WORD);
                self.push_emphasis(out, kind, content.text, first);
                self.shown_run(content.writer);
            }
            Inline::Link { label, href, title } => {
                if label.writer.boundary_before {
                    self.push(out, Inline::WordBoundary);
                }
                self.start_piece(out, label.writer.first_visible.unwrap_or(WORD), Some('['));
                out.edit_line(|text, _| {
                    // an unescaped `!` before `[` would make image syntax
                    escape_trailing_bang(text);
                    text.push('[');
                    text.push_str(label.text);
                    text.push_str("](");
                    push_md_url(text, href);
                    if let Some(title) = title {
                        push_title(text, title);
                    }
                    text.push(')');
                });
                self.shown_run(label.writer);
                self.written = LastWritten::Edge;
            }
        }
    }

    /// Block output written as it is ends the inline run: it drops a word boundary waiting
    /// for it, and its last character is the visible end.
    pub(super) fn push_raw(&mut self, out: &mut Lines, s: &str) {
        self.settle_at_end(out);
        self.ending_space = false;
        debug_assert!(
            !s.is_empty(),
            "raw content is a marker or a line of a block"
        );
        out.push_str(s);
        self.boundary_after = false;
        self.first_visible.get_or_insert('\n');
        self.last_visible = out.as_str().chars().next_back();
        self.written = LastWritten::Edge;
    }

    /// Writes a pending run: what follows is the end of the line or of the enclosing markup.
    pub(super) fn settle_at_end(&mut self, out: &mut Lines) {
        self.settle(out, None);
    }

    /// Ends the line: a word boundary waiting for more of it is dropped.
    pub(super) fn end_line(&mut self) {
        self.boundary_after = false;
        self.first_visible.get_or_insert('\n');
        self.last_visible = Some('\n');
        self.written = LastWritten::Edge;
    }

    /// A hard break, written once more of the line follows.
    pub(super) fn hard_break(&mut self, out: &mut Lines) {
        self.settle(out, Some('\n'));
        self.boundary_after = false;
        self.breaks += 1;
    }

    /// A hard break unless one is owed already, for a line that ends some other way.
    pub(super) fn ensure_hard_break(&mut self, out: &mut Lines) {
        self.settle(out, Some('\n'));
        if self.breaks == 0 {
            self.hard_break(out);
        }
        self.boundary_after = false;
    }

    /// The hard breaks owed.
    pub(super) const fn breaks(&self) -> usize {
        self.breaks
    }

    /// Takes the hard breaks owed, for a block boundary that drops them.
    pub(super) fn take_breaks(&mut self, out: &mut Lines) -> usize {
        self.settle(out, None);
        std::mem::take(&mut self.breaks)
    }

    /// Writes the hard breaks owed, each ending its line.
    #[inline]
    pub(super) fn write_breaks(&mut self, out: &mut Lines) {
        if self.breaks > 0 {
            self.write_breaks_now(out);
        }
    }

    #[cold]
    fn write_breaks_now(&mut self, out: &mut Lines) {
        for _ in 0..std::mem::take(&mut self.breaks) {
            self.ending_space = false;
            out.hard_break();
            self.end_line();
        }
    }

    /// Whether nothing is written yet on a line of the document: edge spaces and a hard break
    /// show nothing there.
    fn at_line_start(&self, out: &Lines) -> bool {
        out.at_line_start() && self.pending_run.is_none()
    }

    /// Starts a piece that shows `next` first and writes `source` first: writes the pending
    /// run, then the waiting word boundary if it is a space. Returns whether a space was written.
    fn start_piece(&mut self, out: &mut Lines, next: char, source: Option<char>) -> bool {
        let boundary_space = self.take_boundary_space(next);
        self.write_owed(out, boundary_space, source)
    }

    /// Whether the waiting word boundary is a space before `next`; takes it, so it answers once.
    fn take_boundary_space(&mut self, next: char) -> bool {
        // a hard break owed is whitespace
        std::mem::take(&mut self.boundary_after)
            && self.breaks == 0
            && boundary_is_space(self.last_visible, next)
    }

    /// Writes the pending run (followed by `source`), the hard breaks or space owed, then the
    /// word boundary's space if `boundary_space`; whether a space was written.
    fn write_owed(&mut self, out: &mut Lines, boundary_space: bool, source: Option<char>) -> bool {
        self.settle(out, if boundary_space { Some(' ') } else { source });
        self.write_breaks(out);
        let spaced = self.ending_space;
        self.write_ending_space(out);
        (boundary_space && self.write_boundary_space(out)) || spaced
    }

    #[cold]
    fn write_boundary_space(&mut self, out: &mut Lines) -> bool {
        let mut wrote = false;
        out.edit_line(|text, _| {
            let len = text.len();
            push_block_space(text);
            wrote = text.len() != len;
        });
        self.written = LastWritten::Char(' ');
        wrote
    }

    /// Holds the run until what follows is known: edges go outside its delimiters, and a run of
    /// the same kind right after (or past one space) merges with it.
    #[inline(never)]
    fn push_emphasis(&mut self, out: &mut Lines, kind: Emphasis, content: &str, first: char) {
        // whitespace alone takes no delimiters
        if content.trim().is_empty() {
            return self.push_blank_emphasis(out, kind, content, first);
        }
        let edges = EmphasisEdges::of(content);
        // a hard break at the start goes before the run
        if edges.hard_start && (!self.at_line_start(out) || self.breaks > 0) {
            self.breaks += 1;
        }
        let boundary_space = self.take_boundary_space(first);
        if let Some(run) = &mut self.pending_run {
            let spaced = boundary_space || run.trail == " " || !edges.lead.is_empty();
            if !edges.hard_start && run.merges_with(kind, edges.core, spaced) {
                run.merge(boundary_space, &edges);
                return self.write_if_hard_end(out, &edges);
            }
        }
        // what the pending run is followed by: an edge is whitespace, and a run of the same
        // kind fuses with it anyway (`LastWritten::delimiters_fit`)
        let same_kind = self
            .pending_run
            .as_ref()
            .is_some_and(|run| run.kind == kind);
        let next = if same_kind || edges.hard_start || !edges.lead.is_empty() {
            None
        } else {
            Some(OTHER_RUN)
        };
        self.write_owed(out, boundary_space, next);
        // spaces show nothing at a line start; a no-break space is content
        if !self.at_line_start(out) || !edges.lead.trim_ascii().is_empty() {
            out.edit_line(|text, _| push_edge_space(text, edges.lead));
            if let Some(c) = edges.lead.chars().next_back() {
                self.written = LastWritten::Char(c);
            }
        }
        let content = std::mem::take(&mut self.spare);
        self.pending_run = Some(EmphasisRun::new(kind, content, self.written, &edges));
        self.write_if_hard_end(out, &edges);
    }

    /// A run that ends in a hard break waits on nothing, so it is written at once.
    fn write_if_hard_end(&mut self, out: &mut Lines, edges: &EmphasisEdges) {
        if !edges.hard_end.is_empty() {
            self.settle(out, Some('\n'));
            self.breaks += 1;
        }
    }

    /// Writes a run of only whitespace, which takes no delimiters: its hard breaks and spaces.
    fn push_blank_emphasis(&mut self, out: &mut Lines, kind: Emphasis, content: &str, first: char) {
        if content.contains('\n') {
            // hard breaks, between the no-break spaces around them; each `  ` before a
            // `\n` is a break's marker, but spaces after the last one are content
            let mut lines = content.split('\n');
            let last = lines.next_back().unwrap_or_default();
            for line in lines.map(|l| l.trim_end_matches(' ')) {
                if !line.trim_ascii().is_empty() {
                    self.push_emphasis(out, kind, line, first);
                }
                self.hard_break(out);
            }
            if !last.trim_ascii().is_empty() {
                self.push_emphasis(out, kind, last, first);
            }
        } else if let Some(run) = &mut self.pending_run {
            run.trail.push_str(content);
        } else if content.bytes().all(|b| b == b' ') {
            self.ending_space |= !self.at_line_start(out) && !out.as_str().ends_with(['\n', ' ']);
        } else {
            // spaces show nothing at a line start; a no-break space is content
            let content = if self.at_line_start(out) {
                content.trim_start_matches(' ')
            } else {
                content
            };
            self.start_piece(out, first, content.chars().next());
            out.push_str(content);
            if let Some(c) = content.chars().next_back() {
                self.written = LastWritten::Char(c);
            }
        }
    }

    /// Writes the pending run, with delimiters if they fit before `after`, else as html.
    #[inline]
    fn settle(&mut self, out: &mut Lines, after: Option<char>) {
        if let Some(run) = self.pending_run.take() {
            self.write_pending(out, run, after);
        }
    }

    #[inline(never)]
    fn write_pending(&mut self, out: &mut Lines, mut run: EmphasisRun, after: Option<char>) {
        let after = run.trail.chars().next().or(after);
        let (kind, start) = (run.kind, run.start);
        if start.delimiters_fit(kind, &run.content, after) {
            let (marker, delim) = (kind.marker(), kind.delimiter());
            out.push_str(marker);
            out.push_lines(&run.content);
            out.push_str(marker);
            self.written = LastWritten::Closing {
                kind,
                char_before: run.content.trim_end_matches(delim).chars().next_back(),
                nested: run.content.ends_with(delim),
            };
        } else {
            let tag = kind.tag();
            out.edit_line(|text, _| {
                text.push('<');
                text.push_str(tag);
                text.push('>');
            });
            out.push_lines(&run.content);
            out.edit_line(|text, _| {
                text.push_str("</");
                text.push_str(tag);
                text.push('>');
            });
            self.written = LastWritten::Edge;
        }
        // spaces at the end are owed like a text's
        let trail = run.trail.trim_end_matches(' ');
        self.ending_space = trail.len() < run.trail.len();
        if let Some(c) = trail.chars().next_back() {
            out.push_str(trail);
            self.written = LastWritten::Char(c);
        }
        run.content.clear();
        self.spare = run.content;
    }

    /// Records a written piece: its visible ends and what the text now ends with.
    fn wrote(&mut self, first: char, last: char, written: LastWritten) {
        self.first_visible.get_or_insert(first);
        self.last_visible = Some(last);
        self.written = written;
    }

    fn shown_run(&mut self, run: &InlineWriter) {
        self.first_visible = self.first_visible.or(run.first_visible);
        self.last_visible = run.last_visible;
        self.boundary_after = run.boundary_after;
    }
}

impl LastWritten {
    /// Whether delimiters of `kind` around `content` fit after this and before `after`; an
    /// opener touching a closer of the same character extends its delimiter run.
    fn delimiters_fit(self, kind: Emphasis, content: &str, after: Option<char>) -> bool {
        let (before, touching) = match self {
            Self::Edge => (None, false),
            Self::Char(c) => (Some(c), false),
            Self::Closing {
                kind: k,
                char_before,
                nested,
            } if k.delimiter() == kind.delimiter() => {
                // `**a***b*` works; `**a****b**`, `***a*****b**` and `**a*****b**c*` don't
                if k == kind || nested || content.starts_with(kind.delimiter()) {
                    return false;
                }
                (char_before, true)
            }
            Self::Closing { kind: k, .. } => (Some(k.delimiter()), false),
        };
        kind.fits(before, touching, content, after)
    }
}

impl EmphasisRun {
    /// A run of `kind` holding `edges`, in `content`, an empty buffer.
    fn new(kind: Emphasis, mut content: String, start: LastWritten, edges: &EmphasisEdges) -> Self {
        content.push_str(edges.core);
        let mut run = Self {
            kind,
            content,
            start,
            trail: String::new(),
        };
        run.set_trail(edges);
        run
    }

    /// Whether a run of `kind` starting with `core` merges with this one, past one space when
    /// `spaced`.
    fn merges_with(&self, kind: Emphasis, core: &str, spaced: bool) -> bool {
        self.kind == kind
            && (self.trail.is_empty() || self.trail == " ")
            && kind.can_merge(&self.content, core, spaced)
    }

    /// Appends the content of a run that merges with this one, after the space between them.
    fn merge(&mut self, boundary_space: bool, edges: &EmphasisEdges) {
        let gap = std::mem::take(&mut self.trail);
        push_edge_space(&mut self.content, &gap);
        if boundary_space {
            push_edge_space(&mut self.content, " ");
        }
        push_edge_space(&mut self.content, edges.lead);
        // an unescaped `!` before `[` would make image syntax
        if edges.core.starts_with('[') {
            escape_trailing_bang(&mut self.content);
        }
        self.content.push_str(edges.core);
        self.set_trail(edges);
    }

    fn set_trail(&mut self, edges: &EmphasisEdges) {
        push_edge_space(&mut self.trail, edges.trail);
    }
}

/// The spacing rule: a word boundary is a space unless whitespace or closing punctuation follows,
/// or whitespace, an opening bracket or a `<br>` (`'\n'`) precedes.
fn boundary_is_space(last: Option<char>, next: char) -> bool {
    let closing = |c: char| {
        c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}')
    };
    let opening = |c: char| c.is_whitespace() || matches!(c, '(' | '[' | '{');
    !closing(next) && !last.is_some_and(opening)
}

/// Pushes `space`, collapsing ASCII spaces into one that `acc` doesn't already end with.
fn push_edge_space(acc: &mut String, space: &str) {
    for c in space.chars() {
        if c != ' ' || !acc.ends_with(' ') {
            acc.push(c);
        }
    }
}

/// A run's content split for writing: edge spaces (`lead`) and a hard break at the start go
/// before the delimiters, `core` between them, and `trail` then `hard_end` after them.
struct EmphasisEdges<'a> {
    hard_start: bool,
    lead: &'a str,
    core: &'a str,
    trail: &'a str,
    hard_end: &'a str,
}

impl<'a> EmphasisEdges<'a> {
    fn of(content: &'a str) -> Self {
        let hard_start = starts_with_hard_break(content);
        let rest = if hard_start {
            content.get(3..).unwrap_or_default()
        } else {
            content
        };
        let core = rest.trim_start_matches(is_edge_space);
        // spaces before a hard break stay with it
        let lead = if core.starts_with('\n') {
            ""
        } else {
            rest.strip_suffix(core).unwrap_or_default()
        };
        let rest = rest.strip_prefix(lead).unwrap_or(rest);
        let (rest, hard_end) = rest
            .strip_suffix("  \n")
            .map_or((rest, ""), |r| (r, "  \n"));
        let core = rest.trim_end_matches(is_edge_space);
        let trail = rest.strip_prefix(core).unwrap_or_default();
        Self {
            hard_start,
            lead,
            core,
            trail,
            hard_end,
        }
    }
}

/// `CommonMark` whitespace (`Zs`, tab, form feed, carriage return), which is moved outside
/// the run; a newline stays with its hard break.
fn is_edge_space(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\n' | '\x0b' | '\u{85}' | '\u{2028}' | '\u{2029}')
}
