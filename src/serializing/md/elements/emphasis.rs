use html5ever::{QualName, local_name};

use crate::NodeId;

use super::super::block::{Block, BlockStart, BlockWriter, EmphasisParagraph, Separator};
use super::super::ctx::{EmphasisScope, FormatOpts};
use super::super::host::MDHost;
use super::super::inline::Inline;
use super::code::ends_with_code_span;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::serializing::md) enum Emphasis {
    Bold,
    Italic,
    Strike,
}

impl Emphasis {
    pub(in crate::serializing::md) const fn of(name: &QualName) -> Option<Self> {
        match name.local {
            local_name!("strong") | local_name!("b") => Some(Self::Bold),
            local_name!("em") | local_name!("i") => Some(Self::Italic),
            local_name!("del") | local_name!("s") | local_name!("strike") => Some(Self::Strike),
            _ => None,
        }
    }

    pub(in crate::serializing::md) const fn marker(self) -> &'static str {
        match self {
            Self::Bold => "**",
            Self::Italic => "*",
            Self::Strike => "~~",
        }
    }

    pub(in crate::serializing::md) const fn delimiter(self) -> char {
        match self {
            Self::Strike => '~',
            _ => '*',
        }
    }

    pub(in crate::serializing::md) const fn tag(self) -> &'static str {
        match self {
            Self::Bold => "strong",
            Self::Italic => "em",
            Self::Strike => "del",
        }
    }

    const fn scope(self) -> EmphasisScope {
        match self {
            Self::Bold => EmphasisScope::BOLD,
            Self::Italic => EmphasisScope::ITALIC,
            Self::Strike => EmphasisScope::STRIKE,
        }
    }

    /// Whether delimiters fit around `content`, past its nested ones, between source chars `before`
    /// and `after` (`None`: an edge); `touching`: the opener touches a closing delimiter before it.
    pub(in crate::serializing::md) fn fits(
        self,
        before: Option<char>,
        touching: bool,
        content: &str,
        after: Option<char>,
    ) -> bool {
        let delim = self.delimiter();
        if fuses_ambiguously(touching, content, delim) {
            return false;
        }
        let first = content.trim_start_matches(delim).chars().next();
        let last = content.trim_end_matches(delim).chars().next_back();
        first.is_none_or(|first| !first.is_whitespace() && opens(before, first))
            && last.is_none_or(|last| !last.is_whitespace() && closes(last, after))
    }

    /// Whether a run of this kind starting with `right` can merge into one ending with `left`,
    /// past one space when `spaced`, without making the delimiters between them ambiguous.
    pub(in crate::serializing::md) fn can_merge(
        self,
        left: &str,
        right: &str,
        spaced: bool,
    ) -> bool {
        let star = matches!(self, Self::Italic) || (spaced && matches!(self, Self::Bold));
        let escapes = left.bytes().rev().take_while(|&b| b == b'\\').count();
        // two code spans that touch would merge their backticks
        let code_spans = ends_with_code_span(left) && right.starts_with('`');
        !(star && left.ends_with('*'))
            && escapes % 2 == 0
            && (spaced || (!code_spans && junction_fits(left, right)))
    }
}

/// Whether a delimiter can close between `last` and `after`.
fn closes(last: char, after: Option<char>) -> bool {
    last.is_alphanumeric() || after.is_none_or(is_space_or_punctuation)
}

/// Whether a delimiter can open between `before` and `first`.
fn opens(before: Option<char>, first: char) -> bool {
    first.is_alphanumeric() || before.is_none_or(is_space_or_punctuation)
}

/// Whether a delimiter opening `content` would fuse with a nested one, or with a closing one
/// before it (`touching`), into a run that a delimiter inside `content` closes early.
fn fuses_ambiguously(touching: bool, content: &str, delim: char) -> bool {
    if !touching && !content.starts_with(delim) {
        return false;
    }
    // not touching, the first run closes the nested one, not the outer, by the multiple-of-3 rule
    // for runs that open and close (CommonMark 6.2, rules 9 and 10): `***a**b*`
    let (mut prev, mut escaped, mut first_run) = (None, false, !touching);
    // `prev` is a literal, so a delimiter after it starts a run
    let mut prev_escaped = false;
    let mut chars = content.chars();
    while let Some(c) = chars.next() {
        // a delimiter run starts here, after `before`
        let before = prev.filter(|&p| c == delim && !escaped && (p != delim || prev_escaped));
        if let Some(before) = before {
            let after = chars.clone().find(|&n| n != delim);
            if !first_run && after.is_some_and(|after| opens_and_closes(before, after)) {
                return true;
            }
            first_run = false;
        }
        prev_escaped = escaped;
        escaped = c == '\\' && !escaped;
        prev = Some(c);
    }
    false
}

/// Whether a delimiter run between `before` and `after` can both open and close: neither is
/// whitespace, and both are punctuation or neither is (`x**y`, `.**.`).
fn opens_and_closes(before: char, after: char) -> bool {
    !before.is_whitespace()
        && !after.is_whitespace()
        && is_space_or_punctuation(before) == is_space_or_punctuation(after)
}

/// Whether the delimiters of runs nested at the end of `left` and the start of `right`
/// still close and open once the two touch.
fn junction_fits(left: &str, right: &str) -> bool {
    debug_assert!(!left.is_empty() && !right.is_empty(), "only run cores meet");
    let (Some(l), Some(r)) = (left.chars().next_back(), right.chars().next()) else {
        return true;
    };
    let is_delimiter = |c: char| matches!(c, '*' | '~');
    match (is_delimiter(l), is_delimiter(r)) {
        (true, true) => false,
        (true, false) => left
            .trim_end_matches(l)
            .chars()
            .next_back()
            .is_none_or(|last| closes(last, Some(r))),
        (false, true) => right
            .trim_start_matches(r)
            .chars()
            .next()
            .is_none_or(|first| opens(Some(l), first)),
        (false, false) => true,
    }
}

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    out: &mut BlockWriter,
    kind: Emphasis,
    id: NodeId,
    opts: FormatOpts,
) {
    let em_opts = opts.include_node().inline().emphasis_scope(kind.scope());
    // inside the same emphasis, its delimiters would close the outer run
    if opts.emphasis_scope.contains(kind.scope()) {
        return h.write_children(out, id, em_opts);
    }
    // a heading's content is a link body or a cell's
    let may_hold_blocks = !opts.link_body && !opts.table_cell;
    let mut run = BlockWriter::fragment(h.take_buf(), em_opts);
    // a run can't span a blank line: each paragraph inside gets its own
    if may_hold_blocks {
        run.keep_paragraphs_apart();
    }
    h.write_children(&mut run, id, em_opts);
    if let Some(paragraphs) = run.take_paragraphs() {
        for p in &paragraphs {
            write_paragraph(out, kind, p);
        }
        return h.put_buf(run.into_buf());
    }
    let blocks = run.finish_fragment() && may_hold_blocks;
    let content = run.content();
    if blocks {
        // no run wraps another block: each tag is an html block of its own
        let tag = kind.tag();
        out.owe(Separator::BlankLine);
        out.push_block(Block::Html, &format!("<{tag}>"));
        out.owe(Separator::BlankLine);
        out.push_block(Block::Text, content.text.trim_matches([' ', '\n']));
        out.owe(Separator::BlankLine);
        out.push_block(Block::Html, &format!("</{tag}>"));
        out.owe(Separator::BlankLine);
    } else {
        // a stray `<li>` starts a line of its own inside the run too
        out.owe(run.owed_before());
        out.push(Inline::Emphasis { kind, content });
        out.owe(run.owed());
    }
    h.put_buf(run.into_buf());
}

/// Writes a paragraph kept apart as a run of its own, set off as it was.
fn write_paragraph(out: &mut BlockWriter, kind: Emphasis, p: &EmphasisParagraph) {
    let para = p.start == BlockStart::Para;
    if para {
        out.open_paragraph();
    }
    out.owe(p.before);
    let content = p.content();
    out.push(Inline::Emphasis { kind, content });
    for _ in 0..p.writer.breaks() {
        out.hard_break();
    }
    if para {
        out.close_paragraph();
    }
}

/// A hard break at the start goes before the run, unless a blank line follows it.
pub(in crate::serializing::md) fn starts_with_hard_break(content: &str) -> bool {
    content
        .strip_prefix("  \n")
        .is_some_and(|rest| !rest.trim_start_matches(' ').starts_with('\n'))
}

/// Common non-ASCII punctuation blocks; anything else (a letter, a mark, a
/// format character such as a soft hyphen) falls back to html.
const PUNCTUATION: &[(char, char)] = &[
    ('\u{a1}', '\u{ac}'),
    ('\u{ae}', '\u{bf}'),
    ('\u{2010}', '\u{2027}'),
    ('\u{2030}', '\u{205e}'),
    ('\u{20a0}', '\u{20c0}'),
    ('\u{2190}', '\u{2426}'),
    ('\u{2500}', '\u{2bff}'),
    ('\u{2e00}', '\u{2e5d}'),
    ('\u{3001}', '\u{3003}'),
    ('\u{3008}', '\u{3011}'),
    ('\u{ff01}', '\u{ff0f}'),
    ('\u{ff1a}', '\u{ff20}'),
    ('\u{ff3b}', '\u{ff40}'),
    ('\u{ff5b}', '\u{ff65}'),
    ('\u{1f000}', '\u{1faff}'),
];

/// Whitespace or punctuation. The spec's Unicode whitespace is `Zs`, tab, LF, FF and CR, so
/// unlike `char::is_whitespace` it leaves out U+000B, U+0085, U+2028 and U+2029.
fn is_space_or_punctuation(c: char) -> bool {
    (c.is_whitespace() && !matches!(c, '\u{b}' | '\u{85}' | '\u{2028}' | '\u{2029}'))
        || c.is_ascii_punctuation()
        || !c.is_alphanumeric() && PUNCTUATION.iter().any(|&(lo, hi)| (lo..=hi).contains(&c))
}
