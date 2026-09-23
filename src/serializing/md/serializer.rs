use std::cell::Ref;

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{Element, NodeId, TreeNodeOps};

use crate::node::{NodeData, NodeRef, ancestor_nodes, child_nodes, descendant_nodes};
use crate::node::{SerializeOp, TreeNode};

use super::constants::{
    CODE_LANGUAGE_ATTRIBUTES, CODE_LANGUAGE_PREFIX, DEFAULT_SKIP_TAGS, LIST_OFFSET_BASE,
};

/// `CommonMark` ordered-list markers have at most nine digits; a longer number
/// turns the marker into plain paragraph text.
const MAX_LIST_NUMBER: u64 = 999_999_999;

/// Escapes an unescaped trailing `!` in the output buffer so the `[` or `![`
/// emitted right after it cannot turn the preceding text into (linked) image
/// syntax (`Wow![x](u)` parses as `Wow` + an image).
fn escape_trailing_bang(text: &mut String) {
    if text
        .strip_suffix('!')
        .is_some_and(|rest| !ends_with_escape(rest))
    {
        text.pop();
        text.push_str("\\!");
    }
}

/// True when `text` ends with an odd run of backslashes, which escapes the
/// character written next. `a\\` ends with an escaped backslash, not an
/// escape.
fn ends_with_escape(text: &str) -> bool {
    (text.len() - text.trim_end_matches('\\').len()) % 2 == 1
}

use super::text_utils::{
    add_linebreaks, join_tendril_strings, push_normalized_text, sanitize_attr_value,
    trim_right_tendril_space,
};

#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy)]
struct FormatOpts {
    include_node: bool,
    ignore_linebreak: bool,
    skip_escape: bool,
    /// Indentation (in columns) of list markers written by this call.
    offset: usize,
    br: bool,
}

struct ListContext<'a> {
    opts: FormatOpts,
    linebreak: &'a str,
    indent: &'a str,
    prefix: String,
    /// Marker number of the next `<li>` for ordered lists; `ul` is `None`.
    next_number: Option<u64>,
}

impl ListContext<'_> {
    /// Options for the current item's content. A nested list must start at
    /// the item's content column, or `100. a` followed by a child list at
    /// four columns turns the child into an indented code block.
    fn item_opts(&self) -> FormatOpts {
        let marker_width = self.prefix.len().max(LIST_OFFSET_BASE);
        self.opts.offset(self.indent.len() + marker_width)
    }
}

impl FormatOpts {
    fn new() -> Self {
        Self::default()
    }

    const fn include_node(mut self) -> Self {
        self.include_node = true;
        self
    }

    const fn ignore_linebreak(mut self) -> Self {
        self.ignore_linebreak = true;
        self
    }

    const fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    const fn skip_escape(mut self) -> Self {
        self.skip_escape = true;
        self
    }
    const fn br(mut self) -> Self {
        self.br = true;
        self
    }
}

pub struct MDSerializer<'a> {
    root_node: &'a NodeRef<'a>,
    nodes: Ref<'a, Vec<TreeNode>>,
    skip_tags: &'a [&'a str],
}

impl<'a> MDSerializer<'a> {
    pub fn new(root_node: &'a NodeRef, skip_tags: Option<&'a [&'a str]>) -> Self {
        let skip_tags = skip_tags.unwrap_or(&DEFAULT_SKIP_TAGS);
        let nodes = root_node.tree.nodes.borrow();
        MDSerializer {
            root_node,
            nodes,
            skip_tags,
        }
    }

    pub fn serialize(&self, include_node: bool) -> StrTendril {
        let mut text = String::new();
        let opts = FormatOpts {
            include_node,
            ..Default::default()
        };
        self.write(&mut text, self.root_node.id, opts);
        StrTendril::from(text)
    }

    fn write(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        // nested writes (list items, link fallbacks) share the caller's
        // buffer, which may already hold output and delimiter offsets
        let owns_buffer = text.is_empty();
        let linebreak = linebreak(opts.br);
        // Start offsets of the opening delimiters of emphasis elements that are
        // still open, in document order (`strong`/`b`/`em`/`i`).
        let mut delim_starts: Vec<usize> = Vec::new();
        // Start offset, end offset and delimiter of the last emphasis element
        // closed with its delimiters written.
        let mut last_closed: Option<(usize, usize, &'static str)> = None;
        let mut ops = if opts.include_node {
            vec![SerializeOp::Open(root_id)]
        } else {
            child_nodes(Ref::clone(&self.nodes), &root_id, true)
                .map(SerializeOp::Open)
                .collect()
        };
        while let Some(op) = ops.pop() {
            match op {
                SerializeOp::Open(id) => {
                    let node = &self.nodes[id.value];
                    match node.data {
                        NodeData::Text { ref contents } => {
                            push_normalized_text(text, contents.as_ref(), !opts.skip_escape);
                        }
                        NodeData::Element(ref e) => {
                            if self.skip_tags.contains(&e.name.local.as_ref()) {
                                continue;
                            }

                            let double_br = linebreak.repeat(2);

                            if is_md_block(&e.name) {
                                if opts.br && opts.ignore_linebreak {
                                    // table-cell mode: block boundaries join
                                    // with a single cell linebreak
                                    add_cell_block_break(text, linebreak);
                                } else if !opts.ignore_linebreak {
                                    add_linebreaks(text, linebreak, &double_br);
                                }
                            }

                            if let Some(prefix) = md_prefix(&e.name) {
                                if is_emphasis_delim(&e.name) {
                                    open_emphasis(
                                        text,
                                        prefix,
                                        &mut delim_starts,
                                        &mut last_closed,
                                    );
                                } else {
                                    text.push_str(prefix);
                                }
                            }

                            if self.write_element(text, e, node, opts) {
                                continue;
                            }

                            ops.push(SerializeOp::Close(&e.name));

                            ops.extend(
                                child_nodes(Ref::clone(&self.nodes), &id, true)
                                    .map(SerializeOp::Open),
                            );
                        }
                        _ => {}
                    }
                }
                SerializeOp::Close(name) => {
                    if let Some(suffix) = md_suffix(name) {
                        match delim_starts.pop() {
                            Some(start) => {
                                if let Some(end) = push_delimiter(text, start, suffix) {
                                    last_closed = Some((start, end, suffix));
                                }
                            }
                            None => text.push_str(suffix),
                        }
                    }
                    let double_br = linebreak.repeat(2);

                    if text.ends_with(&double_br) {
                        continue;
                    }
                    if is_md_block(name) {
                        if opts.br && opts.ignore_linebreak {
                            add_cell_block_break(text, linebreak);
                        } else if !opts.ignore_linebreak {
                            add_linebreaks(text, linebreak, &double_br);
                        }
                    } else if matches!(
                        name.local,
                        local_name!("br") | local_name!("li") | local_name!("tr")
                    ) {
                        // <br> handled as "   \n".
                        // **Fallback**: if `li` and `tr` are handled outside their context.
                        trim_right_tendril_space(text);
                        text.push_str("  ");
                        text.push_str(linebreak);
                    }
                }
            }
        }

        if !opts.include_node {
            text.truncate(text.trim_end().len());
            // trimming the front of a shared buffer would shift the caller's
            // recorded byte offsets of open emphasis delimiters
            if owns_buffer {
                text.drain(..text.len() - text.trim_start().len());
            }
        }
    }

    fn write_text(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        let mut ops = if opts.include_node {
            vec![root_id]
        } else {
            child_nodes(Ref::clone(&self.nodes), &root_id, true).collect()
        };

        while let Some(id) = ops.pop() {
            let node = &self.nodes[id.value];
            if let NodeData::Text { ref contents } = node.data {
                push_normalized_text(text, contents.as_ref(), !opts.skip_escape);
            } else if let NodeData::Element(ref _e) = node.data {
                ops.extend(child_nodes(Ref::clone(&self.nodes), &id, true));
            }
        }
    }

    fn write_element(
        &self,
        text: &mut String,
        e: &Element,
        tree_node: &TreeNode,
        opts: FormatOpts,
    ) -> bool {
        let mut matched = true;

        match e.name.local {
            local_name!("ul") => {
                let list_prefix = if opts.br { "+ " } else { "- " };
                self.write_list(text, tree_node, list_prefix, opts, None);
            }
            local_name!("ol") => {
                let start = e
                    .attr("start")
                    .and_then(|v| parse_list_number(&v))
                    .unwrap_or(1);
                self.write_list(text, tree_node, "1. ", opts, Some(start));
            }
            local_name!("a") => self.write_link(text, tree_node),
            local_name!("img") => Self::write_img(text, tree_node),
            local_name!("pre") => self.write_pre(text, tree_node),
            local_name!("blockquote") => self.write_blockquote(text, tree_node),
            local_name!("table") => self.write_table(text, tree_node),
            local_name!("code") => self.write_code(text, tree_node),
            _ => matched = false,
        }
        matched
    }

    fn write_list_item(&self, text: &mut String, node_id: NodeId, ctx: &mut ListContext) {
        advance_list_number(ctx);
        trim_right_tendril_space(text);
        text.push_str(ctx.indent);
        text.push_str(&ctx.prefix);
        self.write(text, node_id, ctx.item_opts());
        text.push_str(ctx.linebreak);
    }

    fn write_list_item_blocks(&self, text: &mut String, node_id: NodeId, ctx: &mut ListContext) {
        advance_list_number(ctx);
        let child_node = NodeRef::new(node_id, self.root_node.tree);

        // continuation lines of the item's blocks sit under the marker line,
        // so they carry the list's own indent as well as the marker width
        let block_indent = format!("{}{}", ctx.indent, " ".repeat(ctx.prefix.len()));
        trim_right_tendril_space(text);
        text.push_str(ctx.indent);
        text.push_str(&ctx.prefix);

        let mut is_first_block = true;
        let mut seen_inline = false;
        for c in child_node.children_it(false) {
            let is_block = !node_is_list(&c) && node_is_md_block(&c);
            if is_block {
                if is_first_block {
                    is_first_block = false;
                    if seen_inline {
                        // inline lead-in content is on the marker line; the
                        // first block child starts after a blank line, or it
                        // is a lazy continuation of the lead-in paragraph
                        trim_right_tendril_space(text);
                        text.push_str(ctx.linebreak);
                        if !ctx.opts.br {
                            text.push_str(ctx.linebreak);
                        }
                        text.push_str(&block_indent);
                    }
                } else {
                    text.push_str(&block_indent);
                }

                self.write(text, c.id, ctx.item_opts());
                text.push_str(ctx.linebreak);
                text.push_str(ctx.linebreak);
            } else {
                let is_invisible = match &self.nodes[c.id.value].data {
                    NodeData::Text { contents } => contents.trim().is_empty(),
                    _ => false,
                };
                if !node_is_list(&c) && !is_invisible {
                    seen_inline = true;
                }
                self.write(text, c.id, ctx.item_opts().include_node());
            }
        }
    }

    fn write_list(
        &self,
        text: &mut String,
        list_node: &TreeNode,
        prefix: &str,
        opts: FormatOpts,
        next_number: Option<u64>,
    ) {
        let indent = " ".repeat(opts.offset);
        let mut ctx = ListContext {
            opts,
            linebreak: linebreak(opts.br),
            indent: &indent,
            prefix: prefix.to_string(),
            next_number,
        };

        for child_id in child_nodes(Ref::clone(&self.nodes), &list_node.id, false) {
            let child_node = NodeRef::new(child_id, self.root_node.tree);

            let is_list_item = child_node.query_or(false, |t| {
                t.as_element()
                    .is_some_and(|e| e.name.local == local_name!("li"))
            });

            let has_blocks = child_node
                .children_it(false)
                .any(|n| !node_is_list(&n) && node_is_md_block(&n));

            if is_list_item && ctx.next_number.is_some() {
                // `<li value>` renumbers this item and the ones after it
                if let NodeData::Element(e) = &self.nodes[child_id.value].data {
                    if let Some(v) = e.attr("value").and_then(|v| parse_list_number(&v)) {
                        ctx.next_number = Some(v);
                    }
                }
            }

            if is_list_item && has_blocks {
                self.write_list_item_blocks(text, child_id, &mut ctx);
            } else if is_list_item {
                self.write_list_item(text, child_id, &mut ctx);
            } else {
                self.write(text, child_id, FormatOpts::new().include_node());
            }
        }
    }

    fn write_link(&self, text: &mut String, link_node: &TreeNode) {
        let Some(el) = link_node.as_element() else {
            return;
        };
        // escape once, when the text is pushed into the link body below;
        // collecting it escaped here would double every backslash now that
        // `ALWAYS_ESCAPED` contains `\\`
        let link_opts = FormatOpts::new().include_node().skip_escape();
        if let Some(href) = el.attr("href") {
            let mut link_text = String::new();
            let body_is_md = if self.has_descendant_img(&link_node.id) {
                // a wrapped image that will be written (it has a source):
                // the full subtree below becomes the body,
                // so skip collecting plain text up front
                true
            } else {
                self.write_text(&mut link_text, link_node.id, link_opts);
                link_text.is_empty()
            };
            if body_is_md {
                // The link body has no text (e.g. a wrapped image) or mixes
                // text with elements: serialize the full subtree as the link
                // body, so linked images don't disappear entirely. The body
                // is already Markdown and must not be escaped again.
                let mut full_text = String::new();
                // write the link's children, not the link itself, or `a`
                // handling would recurse
                self.write(&mut full_text, link_node.id, FormatOpts::new());
                // a card-style link wraps blocks (`<a><div><img></div><p>`);
                // a blank line inside `[...]` would end the paragraph and
                // leave the brackets as literal text
                link_text = join_lines(&full_text);
            }
            if !link_text.is_empty() {
                // an unescaped trailing `!` would turn `[x](u)` into image syntax
                escape_trailing_bang(text);
                text.push('[');
                if body_is_md {
                    text.push_str(&link_text);
                } else {
                    push_normalized_text(text, &link_text, true);
                }
                text.push(']');
                text.push('(');
                text.push_str(&md_link_destination(&href));
                if let Some(title) = el.attr("title") {
                    text.push_str(" \"");
                    push_normalized_text(text, &title, true);
                    text.push('"');
                }
                text.push(')');
            }
        } else {
            self.write(text, link_node.id, FormatOpts::default());
        }
    }

    fn has_descendant_img(&self, id: &NodeId) -> bool {
        descendant_nodes(Ref::clone(&self.nodes), id).any(|child_id| {
            matches!(
                &self.nodes[child_id.value].data,
                NodeData::Element(e) if e.name.local == local_name!("img") && img_src(e).is_some()
            )
        })
    }

    fn write_img(text: &mut String, img_node: &TreeNode) {
        let Some(el) = img_node.as_element() else {
            return;
        };
        if let Some(src) = img_src(el) {
            // same hazard as in `write_link`: `Wow!` followed by `![` would
            // swallow the text's own `!` into the image syntax
            escape_trailing_bang(text);
            text.push_str("![");
            if let Some(alt) = el.attr("alt") {
                // alt text is inline content: `]` would end it, a final `\`
                // would escape the `]`, and `*y*` would lose its asterisks
                push_normalized_text(text, &alt, true);
            }
            text.push(']');
            text.push('(');
            text.push_str(&md_link_destination(&src));
            if let Some(title) = el.attr("title") {
                text.push_str(" \"");
                push_normalized_text(text, &title, true);
                text.push('"');
            }
            text.push(')');
        }
    }

    /// Tries to find the language label in the given node using a heuristic.
    ///
    /// Pages may use custom `data-` attributes on the tag itself.
    fn find_code_language(&self, node: &TreeNode) -> Option<String> {
        // Check the current node
        if let Some(language) = find_code_lang_attribute(node) {
            return Some(language);
        }

        ancestor_nodes(Ref::clone(&self.nodes), &node.id, Some(3))
            .find_map(|id| find_code_lang_attribute(&self.nodes[id.value]))
            .or_else(|| self.find_code_language_css_class(node))
    }

    /// Tries to find the language from the CSS class of the first `<code>` element child of the `<pre>` block.
    fn find_code_language_css_class(&self, pre_node: &TreeNode) -> Option<String> {
        let code_elem = child_nodes(Ref::clone(&self.nodes), &pre_node.id, false).find_map(|id| {
            let node = &self.nodes[id.value];
            node.as_element()
                .filter(|el| el.name.local == local_name!("code"))
        });

        code_elem?
            .class()?
            .split_ascii_whitespace()
            .find_map(|class| class.strip_prefix(CODE_LANGUAGE_PREFIX))
            .map(sanitize_attr_value)
    }

    /// Transforms a `<pre>` code block, possibly with an associated language label that the resulting
    /// block is annotated with.
    fn write_pre(&self, text: &mut String, pre_node: &TreeNode) {
        let content = TreeNodeOps::text_of(Ref::clone(&self.nodes), pre_node.id);
        // The fence must be longer than any backtick run in the content,
        // otherwise an interior fence-length line terminates the block early
        // (CommonMark §fenced-code-blocks).
        let fence_len = max_backtick_run(&content).max(2) + 1;
        let fence = "`".repeat(fence_len);
        text.push('\n');
        text.push_str(&fence);
        if let Some(lang) = self.find_code_language(pre_node) {
            text.push_str(&lang);
        }
        text.push('\n');
        text.push_str(&content);
        text.push('\n');
        text.push_str(&fence);
        text.push('\n');
    }

    /// Writes the content of the `<code>` block. Generally a `<code>` tag is used inline, but unfortunately
    /// it's also used instead of a `<pre>` block. In case the `<code>` block contains multiline
    /// text, it's handled as a `<pre>` code block.
    fn write_code(&self, text: &mut String, code_node: &TreeNode) {
        let is_multiline = descendant_nodes(Ref::clone(&self.nodes), &code_node.id)
            .map(|id| &self.nodes[id.value])
            .filter_map(|t| match t.data {
                NodeData::Text { ref contents } => Some(contents),
                _ => None,
            })
            .any(|text| text.trim().contains('\n'));

        if is_multiline {
            return self.write_pre(text, code_node);
        }
        let mut code_text = String::new();
        self.write(
            &mut code_text,
            code_node.id,
            FormatOpts::new().skip_escape(),
        );
        // Backslash escapes are not interpreted inside code spans, so content
        // containing backticks cannot be serialized with escaped backticks.
        // Wrap it in a delimiter run longer than any backtick run it contains.
        let backtick_run = max_backtick_run(&code_text);
        if backtick_run == 0 {
            text.push('`');
            text.push_str(&code_text);
            text.push('`');
        } else {
            let fence = "`".repeat(backtick_run + 1);
            text.push_str(&fence);
            text.push(' ');
            text.push_str(&code_text);
            text.push(' ');
            text.push_str(&fence);
        }
    }

    fn write_blockquote(&self, text: &mut String, quote_node: &TreeNode) {
        let opts = FormatOpts::new();
        let mut quote_buf = String::new();
        self.write(&mut quote_buf, quote_node.id, opts);

        if quote_buf.is_empty() {
            return;
        }

        while !text.ends_with("\n\n") {
            text.push('\n');
        }

        for line in quote_buf.lines() {
            text.push_str("> ");
            text.push_str(line);
            text.push('\n');
        }

        text.push('\n');
    }

    fn write_table(&self, text: &mut String, table_node: &TreeNode) {
        let table_ref = NodeRef::new(table_node.id, self.root_node.tree);

        if !is_table_node_writable(&table_ref) {
            self.write(text, table_node.id, FormatOpts::default());
            return;
        }

        let opts = FormatOpts::new().ignore_linebreak().br();
        // A first row made only of `th` cells is the table header; otherwise
        // the header row is left blank. `th` cells in other rows are kept as
        // first-class cells; previously every `th` in the table was
        // collected into the header row, losing the row labels.
        let mut rows = vec![];
        let mut has_header_row = false;
        for tr_ref in table_ref.find(&["tr"]) {
            let mut row = vec![];
            let mut all_th = true;
            // iterate the row's element children directly: `find` chains
            // selectors as descendant steps, so th and td need separate
            // handling, and document order within the row matters
            for cell_id in child_nodes(Ref::clone(&self.nodes), &tr_ref.id, false) {
                let is_cell = matches!(
                    &self.nodes[cell_id.value].data,
                    NodeData::Element(e) if matches!(
                        e.name.local,
                        local_name!("th") | local_name!("td")
                    )
                );
                if is_cell {
                    all_th &= matches!(
                        &self.nodes[cell_id.value].data,
                        NodeData::Element(e) if e.name.local == local_name!("th")
                    );
                    let mut cell_text = String::new();
                    self.write(&mut cell_text, cell_id, opts);
                    trim_trailing_cell_break(&mut cell_text);
                    row.push(escape_cell_pipes(&cell_text));
                }
            }
            if !row.is_empty() {
                if rows.is_empty() {
                    has_header_row = all_th;
                }
                rows.push(row);
            }
        }

        if rows.is_empty() {
            self.write(text, table_node.id, FormatOpts::default());
            return;
        }

        let headings = if has_header_row {
            rows.remove(0)
        } else {
            vec![" ".into(); rows[0].len()]
        };

        text.push_str("\n| ");

        let heading = join_tendril_strings(&headings, " | ");
        text.push_str(&heading);
        text.push_str(" |\n");
        text.push_str("| ");

        text.push_str(
            headings
                .iter()
                // an empty heading cell must still produce a delimiter cell,
                // otherwise the row is not a valid delimiter row
                .map(|s| "-".repeat(s.len().max(1)))
                .collect::<Vec<_>>()
                .join(" | ")
                .as_str(),
        );
        text.push_str(" |\n");

        for row in rows {
            text.push_str("| ");
            text.push_str(&join_tendril_strings(&row, " | "));
            text.push_str(" |\n");
        }

        text.push('\n');
    }
}

// Limited set of elements treated as block-level in Markdown output.
const fn is_md_block(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("article")
            | local_name!("blockquote")
            | local_name!("section")
            | local_name!("div")
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
            // figcaption always follows a block-level image inside `figure`;
            // dt/dd are the block children of `dl`
            | local_name!("figcaption")
            | local_name!("dt")
            | local_name!("dd")
    )
}

fn node_is_md_block(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_md_block(&name))
}

/// In table-cell mode (`br` linebreaks, no blank lines allowed), block-level
/// children of a cell are joined with a single linebreak instead of the
/// blank-line separation used in normal flow.
fn add_cell_block_break(text: &mut String, linebreak: &str) {
    trim_right_tendril_space(text);
    if text.is_empty() || text.ends_with(linebreak) || text.ends_with('\n') {
        return;
    }
    text.push_str(linebreak);
}

/// Drops trailing cell linebreaks left by the last block child of a cell.
fn trim_trailing_cell_break(text: &mut String) {
    while let Some(rest) = text.strip_suffix("<br>") {
        // `\<br>` is escaped cell text, not a linebreak
        if ends_with_escape(rest) {
            break;
        }
        text.truncate(rest.len());
        trim_right_tendril_space(text);
    }
}

/// Escapes every `|` of a cell that is not escaped yet. Text pipes already
/// are, but code span content and link destinations are written raw, and a
/// bare `|` there still splits the cell: GFM finds cell boundaries before
/// parsing inline content.
///
/// A cell holding a line ending (a fenced code block) has already broken
/// out of the table row, and a backslash there would show up literally, so
/// it is left alone.
fn escape_cell_pipes(cell: &str) -> String {
    if cell.contains('\n') {
        return cell.to_string();
    }
    let mut out = String::with_capacity(cell.len());
    for c in cell.chars() {
        if c == '|' && !ends_with_escape(&out) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

const fn is_list(name: &QualName) -> bool {
    matches!(name.local, local_name!("ul") | local_name!("ol"))
}

fn node_is_list(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_list(&name))
}

const fn md_prefix(name: &QualName) -> Option<&'static str> {
    let prefix = match name.local {
        local_name!("h1") => "# ",
        local_name!("h2") => "## ",
        local_name!("h3") => "### ",
        local_name!("h4") => "#### ",
        local_name!("h5") => "##### ",
        local_name!("h6") => "###### ",
        local_name!("strong") | local_name!("b") => "**",
        local_name!("em") | local_name!("i") => "*",
        local_name!("hr") => "---",
        _ => "",
    };

    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

const fn md_suffix(name: &QualName) -> Option<&'static str> {
    match name.local {
        local_name!("strong") | local_name!("b") => Some("**"),
        local_name!("em") | local_name!("i") => Some("*"),
        _ => None,
    }
}

fn is_table_node_writable(table_node: &NodeRef) -> bool {
    if table_node.is("table:has(table)") {
        // if table has inline table then ignore this table
        return false;
    }
    let mut common_cell_count: usize = 0;
    for row in table_node.find(&["tr"]) {
        let curr_cell_count = row.find(&["th"]).len() + row.find(&["td"]).len();
        if common_cell_count == 0 {
            common_cell_count = curr_cell_count;
        } else if common_cell_count != curr_cell_count {
            return false;
        }
    }
    if common_cell_count == 0 {
        return false;
    }
    true
}

const fn is_emphasis_delim(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("strong") | local_name!("b") | local_name!("em") | local_name!("i")
    )
}

/// Writes the opening delimiter of an emphasis element and records its start
/// offset.
///
/// `<b>a</b><b>b</b>` would naively serialize as `**a****b**`, one delimiter
/// run that parses back as neither element. When the buffer ends with the
/// closing delimiter of an element of the same type, that element is
/// continued instead (`**ab**`). Adjacent runs of different types
/// (`**a***b*`, `*a***b**`) parse back into two elements and need nothing.
fn open_emphasis(
    text: &mut String,
    delim: &'static str,
    delim_starts: &mut Vec<usize>,
    last_closed: &mut Option<(usize, usize, &'static str)>,
) {
    match *last_closed {
        Some((start, end, closed)) if end == text.len() && closed == delim => {
            text.truncate(text.len() - delim.len());
            delim_starts.push(start);
            *last_closed = None;
        }
        _ => {
            delim_starts.push(text.len());
            text.push_str(delim);
        }
    }
}

/// Writes the closing delimiter of an inline emphasis element, keeping
/// whitespace at the element's boundaries outside the delimiter run.
///
/// `CommonMark` requires an opening delimiter to be followed, and a closing
/// delimiter to be preceded, by a non-whitespace character. Without this,
/// `<strong>text </strong>` serializes as `**text **`, which every Markdown
/// renderer displays as literal asterisks.
///
/// Returns the end offset of the closing delimiter, or `None` when the
/// element had no content and its delimiters were dropped.
fn push_delimiter(text: &mut String, start: usize, delim: &str) -> Option<usize> {
    let delim_len = delim.len();

    // The element has no visible content (`<strong></strong>`,
    // `<strong> </strong>`): drop the delimiters entirely, a delimiter run
    // surrounded by whitespace is not emphasis anyway.
    if text.len() <= start + delim_len || text[start + delim_len..].chars().all(|c| c == ' ') {
        if text.len() > start {
            text.replace_range(start..text.len().min(start + delim_len), "");
        }
        return None;
    }

    // Leading boundary: `**␣text` → `␣**text`. The run can be longer than
    // one space when a nested element already moved its own leading space
    // out (`<b> <i> y</i></b>`).
    let spaces = text[start + delim_len..]
        .bytes()
        .take_while(|&b| b == b' ')
        .count();
    if spaces > 0 {
        // the delimiter run shifts right past the spaces
        let moved = format!("{}{delim}", " ".repeat(spaces));
        text.replace_range(start..start + delim_len + spaces, &moved);
    }

    // Trailing boundary: `**text␣` → `**text**␣`.
    let len_before = text.len();
    trim_right_tendril_space(text);
    let trimmed = len_before != text.len();
    text.push_str(delim);
    let end = text.len();
    if trimmed {
        text.push(' ');
    }
    Some(end)
}

/// The URL an `<img>` is serialized with. Lazy-load pages keep the URL in
/// `srcset` or `data-src` instead; an empty placeholder `src=""` counts as
/// missing. For `srcset` take the first candidate URL (the leading token).
fn img_src(el: &Element) -> Option<StrTendril> {
    el.attr("src")
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            el.attr("srcset").and_then(|s| {
                // per the HTML srcset algorithm: skip leading whitespace
                // and commas, then strip the trailing comma a descriptor-less
                // first candidate leaves behind (`srcset="a.png, b.png 2x"`)
                s.trim_start_matches(|c: char| c == ',' || c.is_ascii_whitespace())
                    .split_ascii_whitespace()
                    .next()
                    .map(|u| u.trim_end_matches(','))
                    .filter(|u| !u.is_empty())
                    .map(StrTendril::from_slice)
            })
        })
        .or_else(|| el.attr("data-src").filter(|s| !s.trim().is_empty()))
}

/// Joins the non-empty lines of `text` with single spaces.
fn join_lines(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(line);
    }
    out
}

fn max_backtick_run(text: &str) -> usize {
    let mut max = 0;
    let mut current = 0;
    for c in text.chars() {
        current = if c == '`' { current + 1 } else { 0 };
        max = max.max(current);
    }
    max
}

/// Parses an `ol start` or `li value` attribute the way HTML does: leading
/// whitespace, an optional sign, then digits, ignoring anything after them
/// (`" 5abc"` is 5). A Markdown list number cannot be negative, so negative
/// values become 0, and values above the nine-digit limit are clamped.
fn parse_list_number(value: &str) -> Option<u64> {
    let value = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (negative, value) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    if negative {
        return Some(0);
    }
    let n = value[..digits].parse::<u64>().unwrap_or(u64::MAX);
    Some(n.min(MAX_LIST_NUMBER))
}

/// Writes the current marker into `ctx.prefix` and advances the counter.
fn advance_list_number(ctx: &mut ListContext) {
    if let Some(n) = ctx.next_number {
        ctx.prefix = format!("{n}. ");
        ctx.next_number = Some(n.saturating_add(1).min(MAX_LIST_NUMBER));
    }
}

/// Formats a link/image destination so it survives as one destination.
///
/// A destination containing a space, a control character (line endings,
/// tabs), `<`, or unbalanced parentheses cannot be written as a bare
/// `(...)` destination: a stray `)` ends it early and a space makes the
/// whole thing plain text. Such destinations are angle-wrapped; `<` and `>`
/// inside are escaped because `>` closes the wrapper, and line endings are
/// percent-encoded because they are not allowed inside the wrapper at all.
/// Backslashes are doubled in both forms, since `\*` or a final `\>` would
/// otherwise be read as an escape.
fn md_link_destination(dest: &str) -> String {
    let mut balance: i32 = 0;
    let needs_wrap = dest.chars().any(|c| match c {
        '(' => {
            balance += 1;
            false
        }
        ')' => {
            balance -= 1;
            balance < 0
        }
        ' ' | '<' => true,
        c => c.is_ascii_control(),
    }) || balance != 0;
    let mut out = String::with_capacity(dest.len() + 2);
    if needs_wrap {
        out.push('<');
    }
    for c in dest.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '<' | '>' if needs_wrap => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            c => out.push(c),
        }
    }
    if needs_wrap {
        out.push('>');
    }
    out
}

const fn linebreak(br: bool) -> &'static str {
    if br { "<br>" } else { "\n" }
}

fn find_code_lang_attribute(node: &TreeNode) -> Option<String> {
    node.as_element()?
        .attrs
        .iter()
        .find(|attr| CODE_LANGUAGE_ATTRIBUTES.contains(&attr.name.local.as_ref()))
        .map(|attr| sanitize_attr_value(&attr.value))
}
