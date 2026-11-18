use html5ever::{QualName, local_name};

use super::constants::LIST_OFFSET_BASE;
use super::ctx::FormatOpts;
use super::elements::emphasis::starts_with_hard_break;
use super::inline::{Fragment, Inline, InlineWriter};
use super::lines::{Lines, PrefixMark};
use super::text_utils::push_block_space;

// Limited set of elements treated as block-level in Markdown output.
pub(super) const fn is_md_block(name: &QualName) -> bool {
    if is_block_wrapper(name) {
        return true;
    }
    matches!(
        name.local,
        local_name!("blockquote")
            | local_name!("p")
            | local_name!("h1")
            | local_name!("h2")
            | local_name!("h3")
            | local_name!("h4")
            | local_name!("h5")
            | local_name!("h6")
            | local_name!("ul")
            | local_name!("ol")
            | local_name!("dl")
            | local_name!("table")
            | local_name!("hr")
            | local_name!("figure")
            | local_name!("figcaption")
            | local_name!("legend")
            | local_name!("dt")
            | local_name!("dd")
    )
}

/// An element that is written as its own line, or table cell, outside link text.
pub(super) fn is_link_block(name: &QualName) -> bool {
    // TODO: add the other HTML5 block elements missing from `is_md_block`.
    is_md_block(name)
        || matches!(
            name.local,
            local_name!("li")
                | local_name!("tr")
                | local_name!("th")
                | local_name!("td")
                | local_name!("caption")
        )
}

/// A `<div>`-like container with no Markdown syntax of its own.
pub(super) const fn is_block_wrapper(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("div")
            | local_name!("section")
            | local_name!("article")
            | local_name!("header")
            | local_name!("footer")
            | local_name!("main")
            | local_name!("nav")
            | local_name!("aside")
            | local_name!("address")
            | local_name!("details")
            | local_name!("summary")
            | local_name!("hgroup")
    )
}

/// An inline element written apart from the text around it, like the options of a `<select>`
/// or an `<svg>` icon.
pub(super) const fn is_spaced_inline(name: &QualName) -> bool {
    matches!(name.local, local_name!("option") | local_name!("svg"))
}

pub(super) const fn is_list(name: &QualName) -> bool {
    matches!(name.local, local_name!("ul") | local_name!("ol"))
}

/// What separates two pieces of output, weakest first: where two meet, the stronger wins.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(super) enum Separator {
    #[default]
    None,
    /// A space, unless whitespace is already there.
    Space,
    LineBreak,
    BlankLine,
}

impl Separator {
    fn write(self, lines: &mut Lines) {
        match self {
            Self::None => {}
            Self::Space => lines.edit_line(|text, _| push_block_space(text)),
            Self::LineBreak => lines.new_line(),
            Self::BlankLine => lines.blank_line(),
        }
    }
}

/// A leaf block, written as lines of its own by `BlockWriter::push_block`.
#[derive(Clone, Copy)]
pub(super) enum Block {
    /// lines of text, such as a caption
    Text,
    Heading,
    CodeFence,
    Rule,
    Table,
    /// a tag on a line of its own, such as `<em>` around blocks
    Html,
}

impl Block {
    /// How it starts and ends, for the gaps around it.
    const fn edges(self) -> (BlockStart, BlockEnd) {
        match self {
            Self::Text => (BlockStart::Text, BlockEnd::Text),
            Self::Heading | Self::CodeFence | Self::Rule => (BlockStart::Block, BlockEnd::Closed),
            Self::Table => (BlockStart::Text, BlockEnd::Row),
            Self::Html => (BlockStart::Text, BlockEnd::Html),
        }
    }
}

/// How a block starts, seen from the line above it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockStart {
    Text,
    /// a `<p>`, set off by blank lines that keep its item loose
    Para,
    Quote,
    /// a heading, fence or rule, or a list item that can interrupt a paragraph
    Block,
    /// a list item that cannot interrupt a paragraph (`CommonMark` §5.2)
    List,
}

/// How a block ends, seen from the line below it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockEnd {
    /// a paragraph at this column: text after it continues it
    Text,
    Para,
    /// a `>` line at this column: text after it would join the quote lazily
    Quote,
    /// a line in a nested list: text after it would join it, but any block start ends it
    Lazy,
    /// a table row: `Lazy`, but no text from outside its item joins it
    Row,
    /// a heading, fence or rule, which ends at its own line end
    Closed,
    /// raw HTML, which runs on to the next blank line (`<` in text is escaped)
    Html,
}

/// The separator between two blocks of an item, from how the first ends and the next
/// starts (`CommonMark` §4.8, §5.1). Only the pairs listed can share lines.
fn block_gap(prev: BlockEnd, next: BlockStart) -> Separator {
    match (prev, next) {
        // the blank lines around a paragraph keep its item loose
        (BlockEnd::Para, _) | (_, BlockStart::Para) => Separator::BlankLine,
        // a heading, fence or rule ends at its own line end, a `>` line or block start
        // interrupts a paragraph, and any item start (§5.2) ends a quote, table or nested item
        (BlockEnd::Closed, _)
        | (BlockEnd::Text, BlockStart::Quote | BlockStart::Block)
        | (BlockEnd::Quote, BlockStart::Block | BlockStart::List)
        | (
            BlockEnd::Lazy | BlockEnd::Row,
            BlockStart::Quote | BlockStart::Block | BlockStart::List,
        ) => Separator::LineBreak,
        // raw HTML runs on to a blank line, two quotes in a row would merge, and text
        // would continue the paragraph, table or nested item above it
        _ => Separator::BlankLine,
    }
}

/// The separator between items: a marker starts the next one after any line, so a line
/// break, unless a paragraph at either side keeps the blank line that makes the list loose.
fn item_gap(prev: BlockEnd, next: BlockStart) -> Separator {
    if prev == BlockEnd::Para || next == BlockStart::Para {
        Separator::BlankLine
    } else {
        Separator::LineBreak
    }
}

/// The separator the lists around require, and how they see the content (`list_gap`).
struct ListGap {
    sep: Separator,
    /// how the block before ended, when `block_gap` decided `sep`
    prev: Option<BlockEnd>,
    /// how the block the content opens starts
    start: BlockStart,
}

/// What precedes a list item in its list.
#[derive(Clone, Copy)]
pub(super) enum Preceding {
    /// the item is separated as the list's first block
    Nothing,
    Item(BlockEnd),
    /// content directly in the list
    Stray(BlockEnd),
}

/// `CommonMark`'s container blocks (block quote, list item), open while their child blocks are
/// written: each prefixes every line inside. A leaf block is a `Block`, written at once.
pub(super) enum LinePrefix {
    /// Each line starts with `> `.
    Quote,
    /// The first line starts with the marker, the others with as many spaces.
    ListItem {
        marker: String,
        preceding: Preceding,
    },
}

/// One paragraph inside an emphasis element, buffered apart so it gets a run of its own, and
/// how it was set off.
pub(super) struct EmphasisParagraph {
    pub(super) before: Separator,
    pub(super) start: BlockStart,
    text: String,
    pub(super) writer: InlineWriter,
}

impl EmphasisParagraph {
    const fn new(before: Separator, start: BlockStart) -> Self {
        Self {
            before,
            start,
            text: String::new(),
            writer: InlineWriter::new(),
        }
    }

    #[inline]
    pub(super) fn content(&self) -> Fragment<'_> {
        Fragment {
            text: &self.text,
            writer: &self.writer,
        }
    }
}

/// A quote or list item being written.
struct Frame {
    prefix: LinePrefix,
    /// its first line, with its marker, is still to be written
    marker_due: bool,
    /// the prefix to restore on close; re-taken at the marker line, once the outer frames due
    /// have added their parts
    outer_prefix: PrefixMark,
    /// list item: how its last block ended
    item_end: BlockEnd,
    /// list item: its first line holds a nested item, so its nested lists sit at its content
    /// column
    starts_with_list: bool,
}

impl Frame {
    fn is_item(&self) -> bool {
        matches!(self.prefix, LinePrefix::ListItem { .. })
    }
}

/// Writes Markdown blocks: inline content goes to the paragraph in progress, and a block goes
/// after the separator it owes, which is written only once content follows.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct BlockWriter {
    lines: Lines,
    inline: InlineWriter,
    opts: FormatOpts,
    /// The separator owed before the next content, written only once content follows.
    owed: Separator,
    /// A `<p>` opened and nothing written in it yet.
    paragraph_opened: bool,
    /// Content may continue the current line without a line break first.
    line_open: bool,
    frames: Vec<Frame>,
    /// The separator owed is written as it is, past the gap rules of the lists around.
    owed_exactly: bool,
    /// Counts the separators settled before content; only `starts_line_or_block` reads it.
    starts_written: usize,
    /// Something past one inline run was written: a block, a line break, a marker or a
    /// paragraph kept apart.
    wrote_block: bool,
    /// The separator owed before the first content, which a fragment leaves to its parent.
    owed_before: Separator,
    /// Some: each paragraph is kept apart instead of separated, the last one being written.
    paragraphs: Option<Vec<EmphasisParagraph>>,
}

impl BlockWriter {
    /// Text a parent places inside a line of its own, such as an emphasis run, a link label
    /// or a cell; `buf` is an empty buffer to write it to.
    #[inline]
    pub(super) fn fragment(buf: String, opts: FormatOpts) -> Self {
        Self {
            lines: Lines::new(buf, opts.table_cell),
            inline: InlineWriter::new(),
            opts,
            owed: Separator::None,
            paragraph_opened: false,
            line_open: true,
            frames: Vec::new(),
            owed_exactly: false,
            starts_written: 0,
            wrote_block: false,
            owed_before: Separator::None,
            paragraphs: None,
        }
    }

    /// Keeps each paragraph apart, for emphasis to give each a run of its own.
    #[inline]
    pub(super) fn keep_paragraphs_apart(&mut self) {
        self.paragraphs = Some(Vec::new());
    }

    /// The paragraphs kept apart, if a line break came before or between them; what is owed
    /// after them comes last, as an empty paragraph.
    #[inline]
    pub(super) fn take_paragraphs(&mut self) -> Option<Vec<EmphasisParagraph>> {
        let set_off = |ps: &Vec<EmphasisParagraph>| {
            ps.first().is_some_and(|p| p.before >= Separator::LineBreak)
        };
        if !self
            .paragraphs
            .as_ref()
            .is_some_and(|ps| self.wrote_block || self.inline.breaks() > 1 || set_off(ps))
        {
            return None;
        }
        self.inline.settle_at_end(&mut self.lines);
        // a space at the end goes after the last run, as in `finish_fragment`
        self.inline.write_ending_space(&mut self.lines);
        let owed = std::mem::take(&mut self.owed);
        let mut paragraphs = self.paragraphs.take()?;
        self.keep_paragraph(&mut paragraphs);
        paragraphs.push(EmphasisParagraph::new(owed, BlockStart::Text));
        Some(paragraphs)
    }

    /// A block other than a paragraph opens: the paragraphs kept apart are joined as if never
    /// kept apart, and stay joined.
    pub(super) fn join_paragraphs(&mut self) {
        let Some(mut paragraphs) = self.paragraphs.take() else {
            return;
        };
        if paragraphs.is_empty() {
            return;
        }
        self.keep_paragraph(&mut paragraphs);
        // what is owed after the last paragraph stays owed after it
        let after = self.owed;
        for (i, p) in paragraphs.into_iter().enumerate() {
            if i > 0 {
                self.owed = p.before;
                let _ = self.write_separator(p.start, None);
            }
            self.lines.push_lines(&p.text);
            self.inline = p.writer;
        }
        self.owed = after;
    }

    /// Moves the paragraph written to the last of `paragraphs`.
    fn keep_paragraph(&mut self, paragraphs: &mut Vec<EmphasisParagraph>) {
        let text = self.lines.take();
        let writer = std::mem::replace(&mut self.inline, InlineWriter::new());
        match paragraphs.last_mut() {
            Some(last) => (last.text, last.writer) = (text, writer),
            None => paragraphs.push(EmphasisParagraph {
                text,
                writer,
                ..EmphasisParagraph::new(Separator::None, BlockStart::Text)
            }),
        }
    }

    /// A document of its own, which starts at a line start.
    pub(super) fn document(buf: String, opts: FormatOpts) -> Self {
        let mut doc = Self::fragment(buf, opts);
        doc.line_open = false;
        doc
    }

    /// The text written, without the separators still owed; what an included node or a link
    /// label writes keeps its edge spaces.
    pub(super) fn finish(mut self) -> String {
        let trim = !self.opts.include_node && !self.opts.link_body;
        self.end_inline(trim);
        self.lines.into_string()
    }

    /// Writes what the inline writer holds back, for a parent to place the fragment; whether it
    /// holds a block.
    pub(super) fn finish_fragment(&mut self) -> bool {
        // a hard break at the end belongs to the line the run is on; two end a paragraph
        self.inline.settle_at_end(&mut self.lines);
        self.wrote_block |=
            self.breaks_before(Separator::None, BlockStart::Text, false) == Separator::BlankLine;
        self.inline.write_breaks(&mut self.lines);
        self.inline.write_ending_space(&mut self.lines);
        self.wrote_block
    }

    /// What the fragment owes before its content, which its parent writes.
    pub(super) const fn owed_before(&self) -> Separator {
        self.owed_before
    }

    /// What the fragment still owes after its content.
    pub(super) const fn owed(&self) -> Separator {
        self.owed
    }

    /// The buffer written to, once its content is placed.
    #[inline]
    pub(super) fn into_buf(self) -> String {
        self.lines.into_string()
    }

    /// The inline content written, for a parent to place; see `finish_fragment`.
    #[inline]
    pub(super) fn content(&self) -> Fragment<'_> {
        Fragment {
            text: self.lines.as_str(),
            writer: &self.inline,
        }
    }

    /// Writes `inline` into the paragraph, after the separator owed if it shows content.
    pub(super) fn push(&mut self, inline: Inline) {
        if self.shows(&inline) {
            // a run's hard break at its start is one more before it
            let run_break = match &inline {
                Inline::Emphasis { content, .. } => starts_with_hard_break(content.text),
                _ => false,
            };
            let sep = self.breaks_before(self.owed, BlockStart::Text, run_break);
            if sep > self.owed {
                self.owe(sep);
                self.wrote_block = true;
            }
            if let (Inline::HtmlBreak, Separator::Space) = (&inline, self.owed) {
                self.owed = Separator::None;
            }
            self.start_content(BlockStart::Text, None);
        } else if match &inline {
            Inline::Text(..) => true,
            // a run of only whitespace at a block boundary shows nothing, like such text; a
            // hard break in it is still written
            Inline::Emphasis { content, .. } => {
                !content.text.is_empty() && !content.text.contains('\n')
            }
            _ => false,
        } {
            return;
        }
        match inline {
            Inline::Text(s, opts) => self.inline.push_text(&mut self.lines, s, opts),
            inline => self.inline.push(&mut self.lines, inline),
        }
    }

    /// Writes a text node; text that continues the line, the common case, skips `push`.
    #[inline]
    pub(super) fn push_text(&mut self, s: &str, opts: FormatOpts) {
        if self.content_needs_start() || self.inline.breaks() > 1 {
            return self.push(Inline::Text(s, opts));
        }
        self.inline.push_text(&mut self.lines, s, opts);
    }

    /// Whether `inline` shows content, which needs what is owed written before it.
    fn shows(&self, inline: &Inline) -> bool {
        // whitespace next to a block boundary shows nothing
        let blank_shows = || !self.content_needs_start() && self.inline.breaks() == 0;
        match inline {
            Inline::WordBoundary => false,
            Inline::Text(s, _) => !s.trim_ascii().is_empty() || blank_shows(),
            Inline::Emphasis { content: run, .. } => {
                !run.text.trim_ascii().is_empty() || (!run.text.is_empty() && blank_shows())
            }
            Inline::Link { label: run, .. } => !run.text.is_empty(),
            _ => true,
        }
    }

    /// Owes `sep` before the next content; a line break or more ends the paragraph.
    pub(super) fn owe(&mut self, sep: Separator) {
        let sep = if self.opts.link_body {
            sep.min(Separator::Space)
        } else if self.opts.table_cell {
            sep.min(Separator::LineBreak)
        } else {
            sep
        };
        // the first line break owed ends the paragraph; a later one owes no second end
        if sep >= Separator::LineBreak && self.owed < Separator::LineBreak {
            self.end_inline(true);
        } else if sep == Separator::Space {
            self.inline.settle_at_end(&mut self.lines);
        }
        self.owed = self.owed.max(sep);
    }

    /// Runs `write`; whether what it wrote started a line or block.
    pub(super) fn starts_line_or_block(&mut self, write: impl FnOnce(&mut Self)) -> bool {
        let starts = self.starts_written;
        write(self);
        self.starts_written != starts
    }

    /// Owes `sep` before content that is no block of an item, which the gap rules don't change.
    pub(super) fn owe_exactly(&mut self, sep: Separator) {
        self.owe(sep);
        self.owed_exactly = true;
    }

    /// Opens a `<p>`, which sits between blank lines.
    pub(super) fn open_paragraph(&mut self) {
        self.owe(Separator::BlankLine);
        self.paragraph_opened = true;
    }

    /// Closes a `<p>`; one that wrote nothing leaves no trace.
    pub(super) fn close_paragraph(&mut self) {
        if !std::mem::take(&mut self.paragraph_opened) {
            self.end_inline(true);
            self.set_item_end(BlockEnd::Para);
        }
        self.owe(Separator::BlankLine);
    }

    /// Writes `block`, each of its `lines` on an output line of its own.
    pub(super) fn push_block(&mut self, block: Block, lines: &str) {
        // a block that writes nothing leaves no trace, like an empty `<p>`; a fence's one empty
        // line is content
        if lines.is_empty() && !matches!(block, Block::CodeFence) {
            return;
        }
        self.join_paragraphs();
        // the fragment holds a block now, even if no line break is written before it
        self.wrote_block = true;
        let (start, end) = block.edges();
        let (first, rest) = lines
            .split_once('\n')
            .map_or((lines, None), |(f, r)| (f, Some(r)));
        self.owe(Separator::LineBreak);
        if first.is_empty() {
            let _ = self.write_separator(start, None);
            self.line_open = true;
        } else {
            self.start_content(start, Some(first));
            self.inline.push_raw(&mut self.lines, first);
        }
        // the lines after the first are not blocks of their own: they only break the line
        let (mut last, mut wrote_text) = (first, !first.is_empty());
        if let Some(rest) = rest {
            self.lines.push_following_lines(rest);
            last = rest.rsplit('\n').next().unwrap_or_default();
            wrote_text = wrote_text || rest.bytes().any(|b| b != b'\n');
        }
        // an empty last line still owes its line break; a written one's spaces are content
        if last.is_empty() {
            self.owed = Separator::LineBreak;
        } else {
            self.owed = Separator::None;
            self.lines.mark_line_start();
        }
        if wrote_text {
            self.inline.end_line();
            self.set_item_end(end);
        }
    }

    /// A hard break, written once more of the paragraph follows; two end it.
    pub(super) fn hard_break(&mut self) {
        if !self.line_ended() {
            self.inline.hard_break(&mut self.lines);
        }
    }

    /// A hard break unless one is owed already, for a line that ends some other way.
    pub(super) fn ensure_hard_break(&mut self) {
        if !self.line_ended() {
            self.inline.ensure_hard_break(&mut self.lines);
        }
    }

    /// A block before already ends the line, or the innermost quote or item has no content yet
    /// (in a cell, a break there is content).
    fn line_ended(&self) -> bool {
        !self.line_open
            || self.owed >= Separator::LineBreak
            || (self.marker_due() && !self.opts.table_cell)
    }

    /// Content written as it is, such as a task marker.
    pub(super) fn push_raw(&mut self, s: &str) {
        self.start_content(BlockStart::Text, None);
        self.inline.push_raw(&mut self.lines, s);
    }

    /// Opens a quote or list item: the lines written until `close_prefix` start with its prefix.
    pub(super) fn open_prefix(&mut self, prefix: LinePrefix) {
        // the line before ends here, without the space owed at its end
        self.end_inline(true);
        self.frames.push(Frame {
            prefix,
            marker_due: true,
            outer_prefix: self.lines.prefix_mark(),
            item_end: BlockEnd::Closed,
            starts_with_list: false,
        });
    }

    /// Closes the innermost quote or item; returns how its content ended, `None` if it had none.
    pub(super) fn close_prefix(&mut self) -> Option<BlockEnd> {
        let frame = self.frames.last()?;
        let end = match (&frame.prefix, frame.marker_due) {
            // an empty item still writes its marker
            (LinePrefix::ListItem { .. }, true) => {
                self.start_content(BlockStart::List, Some(""));
                Some(BlockEnd::Closed)
            }
            (_, true) => None,
            (LinePrefix::ListItem { .. }, false) => Some(frame.item_end),
            (LinePrefix::Quote, false) => Some(BlockEnd::Quote),
        };
        // hard breaks at a container's end leave no blank line inside it
        if self.inline.breaks() > 1 {
            self.inline.take_breaks(&mut self.lines);
            self.inline.hard_break(&mut self.lines);
        }
        self.end_inline(true);
        let frame = self.frames.pop()?;
        self.lines.restore_prefix(frame.outer_prefix);
        if let Some(end) = end {
            let nested = frame.is_item();
            // seen from outside the item, only a line that takes no text stays closed
            let end = match end {
                BlockEnd::Closed | BlockEnd::Row if nested => BlockEnd::Closed,
                _ if nested => BlockEnd::Lazy,
                end => end,
            };
            self.set_item_end(end);
        }
        end
    }

    /// Whether the next block is a block of a list item.
    pub(super) fn in_list_item(&self) -> bool {
        self.frames.last().is_some_and(Frame::is_item)
    }

    /// The innermost list item, if the next block is one of its blocks.
    fn innermost_list_item(&mut self) -> Option<&mut Frame> {
        self.frames.last_mut().filter(|f| f.is_item())
    }

    fn set_item_end(&mut self, end: BlockEnd) {
        if let Some(item) = self.innermost_list_item() {
            item.item_end = end;
        }
    }

    /// Ends the inline content in progress: what it holds back is written, its trailing spaces
    /// dropped.
    fn end_inline(&mut self, trim: bool) {
        self.inline.settle_at_end(&mut self.lines);
        if trim {
            self.inline.drop_ending_space();
        } else {
            self.inline.write_ending_space(&mut self.lines);
        }
        self.inline.end_line();
    }

    /// Writes what content starting as `start` needs first: the separator owed, a new line's
    /// prefix and the markers of the items it starts; `head` is its first line, `None` inline.
    fn start_content(&mut self, start: BlockStart, head: Option<&str>) {
        if self.content_needs_start() {
            self.start_block(start, head);
        }
    }

    /// The innermost quote or item has yet to write its first line. Frames write theirs from the
    /// outside in, so if any frame is due, the innermost is.
    fn marker_due(&self) -> bool {
        self.frames.last().is_some_and(|f| f.marker_due)
    }

    /// Content written now needs a separator, a line prefix or a marker before it.
    fn content_needs_start(&self) -> bool {
        self.owed != Separator::None || !self.line_open || self.marker_due()
    }

    fn start_block(&mut self, start: BlockStart, head: Option<&str>) {
        let start = if std::mem::take(&mut self.paragraph_opened) {
            BlockStart::Para
        } else {
            start
        };
        let sep = self.write_separator(start, head);
        let marker_due = self.marker_due();
        // text an item sets off with a blank line is a paragraph, which text after it can't join
        let para = !marker_due && start == BlockStart::Text && sep == Separator::BlankLine;
        if !self.line_open || marker_due {
            self.start_line(head);
        }
        self.set_item_end(if para { BlockEnd::Para } else { BlockEnd::Text });
    }

    fn write_separator(&mut self, start: BlockStart, head: Option<&str>) -> Separator {
        let owed = std::mem::take(&mut self.owed);
        let i = self.frames.iter().position(|f| f.marker_due);
        let marker = i.is_some_and(|i| self.frames.get(i).is_some_and(Frame::is_item));
        // the lists around decide the separator, unless it is owed exactly
        let gap = if std::mem::take(&mut self.owed_exactly) || self.frames.is_empty() {
            ListGap {
                sep: owed,
                prev: None,
                start,
            }
        } else {
            self.list_gap(i.unwrap_or(self.frames.len()), owed, start, head)
        };
        let mut sep = gap.sep;
        // a paragraph kept apart ends here instead
        if sep >= Separator::LineBreak {
            if let Some(paragraphs) = self.paragraphs.take() {
                return self.next_paragraph(paragraphs, sep, start);
            }
        }
        // an empty `>` line ends a quote's paragraph without the blank line
        if gap.prev == Some(BlockEnd::Quote)
            && gap.start == BlockStart::Text
            && !self.opts.table_cell
        {
            sep = Separator::LineBreak;
            self.lines.new_line();
            self.lines.push('>');
        }
        // hard breaks owed raise or replace it, and a line break drops them
        sep = self.breaks_before(sep, start, false);
        if sep >= Separator::LineBreak {
            self.write_breaks_before(sep, marker);
        }
        // a fragment leaves the separator before its first content to its parent
        if self.lines.as_str().is_empty() {
            self.owed_before = self.owed_before.max(sep);
            sep = Separator::None;
        }
        if sep == Separator::Space {
            // a space held back at the end of the text is the separator's
            self.inline.write_ending_space(&mut self.lines);
        }
        sep.write(&mut self.lines);
        if sep >= Separator::LineBreak {
            self.line_open = false;
            self.wrote_block = true;
        }
        self.starts_written += 1;
        sep
    }

    /// Keeps the paragraph written apart and starts the next, set off by `before`.
    fn next_paragraph(
        &mut self,
        mut paragraphs: Vec<EmphasisParagraph>,
        before: Separator,
        start: BlockStart,
    ) -> Separator {
        // nothing written yet means this is the first paragraph (`owe` wrote any pending run)
        if self.lines.as_str().is_empty() {
            // its hard breaks are already counted in `before`
            self.inline.take_breaks(&mut self.lines);
        } else {
            self.keep_paragraph(&mut paragraphs);
            self.wrote_block = true;
        }
        paragraphs.push(EmphasisParagraph::new(before, start));
        self.paragraphs = Some(paragraphs);
        self.line_open = true;
        self.starts_written += 1;
        Separator::None
    }

    /// The separator the lists around require before content starting as `start` (`head` its
    /// first line), else `owed`; frame `i` is the first whose marker line the content starts.
    fn list_gap(
        &self,
        i: usize,
        owed: Separator,
        start: BlockStart,
        head: Option<&str>,
    ) -> ListGap {
        // how the block starting at frame `j` starts: a quote's or item's own start, else the content's
        let block_start = |j: usize| match self.frames.get(j).map(|f| &f.prefix) {
            Some(LinePrefix::Quote) => BlockStart::Quote,
            Some(LinePrefix::ListItem { marker, .. }) if self.interrupts(j, marker, head) => {
                BlockStart::Block
            }
            Some(LinePrefix::ListItem { .. }) => BlockStart::List,
            _ => start,
        };
        // the block the content opens in frame `i`'s parent, and the first block inside frame `i`
        let (new_block, first_block) = (block_start(i), block_start(i + 1));
        let enclosing = i.checked_sub(1).and_then(|j| self.frames.get(j));
        let (sep, prev) = match (self.frames.get(i).map(|f| &f.prefix), enclosing) {
            (
                Some(LinePrefix::ListItem {
                    preceding: Preceding::Item(end),
                    ..
                }),
                _,
            ) => (item_gap(*end, first_block), None),
            (
                Some(LinePrefix::ListItem {
                    preceding: Preceding::Stray(end),
                    ..
                }),
                _,
            ) => (owed.max(block_gap(*end, new_block)), Some(*end)),
            // the content is a new block of the enclosing item, after a line break (opening a
            // quote or list owes one); text that continues the item's line keeps what is owed
            (_, Some(outer)) if outer.is_item() && owed >= Separator::LineBreak => {
                (block_gap(outer.item_end, new_block), Some(outer.item_end))
            }
            _ => (owed, None),
        };
        // a cell has no blank line between a list's blocks
        let sep = if self.opts.table_cell && prev.is_some() {
            sep.min(Separator::LineBreak)
        } else {
            sep
        };
        ListGap {
            sep,
            prev,
            start: new_block,
        }
    }

    /// `sep` after the breaks owed (`run_break`: one more), before `start`: two end the paragraph
    /// unless they end an item's content, one replaces a space, a cell's first text keeps them.
    fn breaks_before(&self, sep: Separator, start: BlockStart, run_break: bool) -> Separator {
        let breaks = self.inline.breaks() + usize::from(run_break);
        if breaks == 0 || (self.opts.table_cell && self.lines.as_str().is_empty()) {
            sep
        } else if breaks > 1 && !self.breaks_end_item(start) {
            sep.max(Separator::BlankLine)
        } else if sep < Separator::LineBreak {
            Separator::None
        } else {
            sep
        }
    }

    /// Whether the hard breaks owed end an item's content, where a block or marker follows them
    /// (outside a cell, which keeps them).
    fn breaks_end_item(&self, start: BlockStart) -> bool {
        !self.opts.table_cell
            && (start != BlockStart::Text || self.marker_due())
            && self
                .frames
                .iter()
                .rfind(|f| !f.marker_due)
                .is_some_and(Frame::is_item)
    }

    /// A block drops the hard breaks owed, except that a single one before a line break still
    /// ends its line (`  `), unless a `marker` starts the next outside a cell's first text.
    fn write_breaks_before(&mut self, sep: Separator, marker: bool) {
        let breaks = self.inline.take_breaks(&mut self.lines);
        let cell_start = self.opts.table_cell && self.lines.as_str().is_empty();
        if sep == Separator::LineBreak && breaks == 1 && (!marker || cell_start) {
            self.inline.drop_ending_space();
            self.lines.push_str("  ");
        }
    }

    /// Whether the marker line of the item at `i`, with `marker`, interrupts a paragraph: it
    /// holds content, `head`, and is a bullet or numbered 1.
    fn interrupts(&self, i: usize, marker: &str, head: Option<&str>) -> bool {
        let numbered = marker.starts_with(|c: char| c.is_ascii_digit());
        head != Some("")
            && !self.rule_like(i, head)
            && (!numbered || marker.starts_with("1.") || marker.starts_with("1)"))
    }

    /// Whether the markers of the items from `i` and `head` make a rule.
    fn rule_like(&self, i: usize, head: Option<&str>) -> bool {
        let Some(head) = head else {
            return false;
        };
        let markers = self.frames.iter().skip(i).filter_map(|f| match &f.prefix {
            LinePrefix::ListItem { marker: m, .. } if f.marker_due => Some(m.as_str()),
            _ => None,
        });
        let line = || {
            markers
                .clone()
                .flat_map(str::bytes)
                .chain(head.bytes())
                .filter(|&b| b != b' ')
        };
        (line().all(|b| b == b'-') || line().all(|b| b == b'*')) && line().count() >= 3
    }

    /// Writes the prefix of a new line, then the first-line parts of the quotes and items that
    /// start on it; markers that would make a rule with `head` go on a line alone.
    fn start_line(&mut self, head: Option<&str>) {
        let cell = self.opts.table_cell;
        let last = self.frames.len();
        let first = self
            .frames
            .iter()
            .position(|f| f.marker_due)
            .unwrap_or(last);
        // a cell indents only a marker line
        if !cell || first < last {
            self.lines.prefix_next();
        }
        for i in first..last {
            self.wrote_block = true;
            let indent = self.item_indent(i);
            let nested_item = self.frames.get(i + 1).is_some_and(Frame::is_item);
            // a rule is looked for at each level, so the markers from any one on can make it
            let rule = self.rule_like(i, head);
            let Some(f) = self.frames.get_mut(i) else {
                break;
            };
            // an item whose first line holds a nested item keeps its nested lists at its
            // content column, for `item_indent` on later lines
            f.starts_with_list |= nested_item && f.is_item();
            f.marker_due = false;
            f.outer_prefix = self.lines.prefix_mark();
            match &f.prefix {
                LinePrefix::Quote => self.lines.push_quote(),
                LinePrefix::ListItem { marker: m, .. } => {
                    let alone = rule || (head == Some("") && i + 1 == last);
                    let marker = if alone { m.trim_end() } else { m };
                    self.lines.push_marker(indent, marker, m.len());
                    if rule {
                        debug_assert!(!cell, "a cell's `+` markers make no rule");
                        self.lines.new_line();
                        self.lines.prefix_next();
                    }
                }
            }
        }
        self.line_open = true;
        self.lines.mark_line_start();
        self.inline.end_line();
    }

    /// The spaces before the marker of the item at `i`: a nested list sits past the content
    /// column, or at it in an item that a nested list started.
    fn item_indent(&self, i: usize) -> usize {
        if !self.frames.get(i).is_some_and(Frame::is_item) {
            return 0;
        }
        match i.checked_sub(1).and_then(|j| self.frames.get(j)) {
            Some(Frame {
                prefix: LinePrefix::ListItem { marker, .. },
                starts_with_list: false,
                ..
            }) => LIST_OFFSET_BASE.saturating_sub(marker.len()),
            _ => 0,
        }
    }
}
