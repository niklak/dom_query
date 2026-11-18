//! HTML to Markdown.
//!
//! # How it writes
//!
//! `MDSerializer` walks the nodes and hands each element to its writer in `elements`. An
//! element writer only says what the element is: it pushes an `Inline` into the paragraph in
//! progress, pushes a `Block`, or opens a `LinePrefix` (a quote or list item) around its
//! children. `BlockWriter` turns those into Markdown, and `Lines` owns the output `String`.
//!
//! The one rule: **write nothing until it is known what it should be.** The separator between
//! blocks, the space a word boundary (an svg or option) leaves, a trailing space, hard breaks
//! and an emphasis run are owed, and settled when the next visible content arrives. A line
//! break, once written, stays: nothing re-indents or re-breaks the lines before it. Only the
//! end of the current line is read or edited (`Lines::edit_line`), as when a `!` before a link
//! is escaped (`escape_trailing_bang`) or a code span is kept off the one before
//! (`ends_with_code_span`). Content is buffered only when a decision needs all of it (an
//! emphasis run or link label for its visible edges, a table cell, a heading), each in a
//! `BlockWriter::fragment` of its own (a heading, in a `BlockWriter::document` through
//! `MDHost::write_buf`); a table cell and a heading drop their trailing `<br>` before placing
//! it.
//!
//! # Where to look
//!
//! | Symptom | Rule |
//! |---|---|
//! | a space wrongly added or missing at a word boundary (an svg or option) | `inline::boundary_is_space` |
//! | a space lost or doubled at an emphasis edge | `EmphasisEdges::of`, `inline::push_edge_space`; in text, `text_utils::push_words` |
//! | `**` printed literally, or `<strong>` where `**` would do | `Emphasis::fits`, `Emphasis::can_merge`, `LastWritten::delimiters_fit`, `emphasis::fuses_ambiguously` |
//! | too many or too few blank lines between blocks | `block::block_gap`, `Block::edges`, `BlockWriter::close_prefix` (an item's end seen from outside) |
//! | a list loose when it should be tight, or the reverse | `block::item_gap`, `BlockWriter::list_gap` |
//! | a lost `> ` or list indent | `BlockWriter::start_line`, `BlockWriter::item_indent`; `Lines` is the only writer of line breaks |
//! | a table row split over two lines | `BlockWriter::owe` (what a cell caps a separator to), `Lines::new_line` (writes the `<br>` in a cell) |
//! | a missing or extra hard break | `BlockWriter::breaks_before`, `BlockWriter::write_breaks_before`, `BlockWriter::close_prefix`, `InlineWriter::hard_break` |
//! | a missing or stray backslash | `text_utils` |
//!
//! # Adding an element
//!
//! Map it to an `Inline`, a `Block` or a `LinePrefix` in its writer. A new kind of output gets a
//! new variant, handled in `InlineWriter::push` or `BlockWriter::push_block`; an element writer
//! never writes to `Lines` itself.
//!
//! `md()` must not panic on any input, so slicing and indexing are denied in this module.
#![deny(clippy::string_slice, clippy::indexing_slicing)]

mod block;
mod constants;
mod ctx;
mod elements;
mod ext;
mod heading;
mod host;
mod inline;
mod lines;
mod serializer;
mod text_utils;

use tendril::StrTendril;

use crate::node::NodeRef;

pub fn serialize_md(
    root_node: &NodeRef,
    include_node: bool,
    skip_tags: Option<&[&str]>,
) -> StrTendril {
    serializer::MDSerializer::new(root_node, skip_tags).serialize(include_node)
}

#[cfg(test)]
mod tests;