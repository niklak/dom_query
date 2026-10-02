use std::cell::Ref;

use html5ever::{QualName, local_name};
use tendril::StrTendril;

use crate::{Element, NodeId, TreeNodeOps};

use crate::node::{NodeData, NodeRef, ancestor_nodes, child_nodes, descendant_nodes};
use crate::node::{SerializeOp, TreeNode};

use super::constants::{
    CODE_LANGUAGE_ATTRIBUTES, CODE_LANGUAGE_PREFIX, DEFAULT_SKIP_TAGS, LIST_OFFSET_BASE,
};

use super::text_utils::{
    add_linebreaks, push_code_text,push_emphasis, push_md_url, push_normalized_text, push_title, sanitize_attr_value,
    trim_right_tendril_space,
};

use super::opts::{EmphasisScope, FormatOpts};

struct ListContext<'a> {
    opts: FormatOpts,
    linebreak: &'a str,
    indent: &'a str,
    prefix: &'a str,
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
        text.into()
    }

    fn write(&self, text: &mut String, root_id: NodeId, opts: FormatOpts) {
        let linebreak = linebreak(opts.br);
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
                    match &node.data {
                        NodeData::Text { contents } => {
                            push_normalized_text(text, contents, opts);
                        }
                        NodeData::Element(e) => {
                            if self.skip_tags.contains(&e.name.local.as_ref()) {
                                continue;
                            }

                            let double_br = linebreak.repeat(2);

                            if !opts.ignore_linebreak && is_md_block(&e.name) {
                                add_linebreaks(text, linebreak, &double_br);
                            }

                            if let Some(prefix) = md_prefix(&e.name) {
                                text.push_str(prefix);
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
                    let double_br = linebreak.repeat(2);

                    if text.ends_with(&double_br) {
                        continue;
                    }
                    if !opts.ignore_linebreak && is_md_block(name) {
                        add_linebreaks(text, linebreak, &double_br);
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

            text.drain(..text.len() - text.trim_start().len());
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
            if let NodeData::Text { contents } = &node.data {
                push_normalized_text(text, contents, opts);
            } else if let NodeData::Element(_e) = &node.data {
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
                self.write_list(text, tree_node, list_prefix, opts);
            }
            local_name!("ol") => self.write_list(text, tree_node, "1. ", opts),
            local_name!("a") => self.write_link(text, tree_node),
            local_name!("img") => Self::write_img(text, tree_node),
            local_name!("pre") => self.write_pre(text, tree_node),
            local_name!("blockquote") => self.write_blockquote(text, tree_node),
            local_name!("table") => self.write_table(text, tree_node),
            local_name!("code") => self.write_code(text, tree_node),
            local_name!("strong") | local_name!("b") | local_name!("em") | local_name!("i") => {
                self.write_emphasis(text, tree_node, opts);
            }
            _ => matched = false,
        }
        matched
    }

    fn write_emphasis(&self, text: &mut String, emphasis_node: &TreeNode, opts: FormatOpts) {
        let node = NodeRef::new(emphasis_node.id, self.root_node.tree);
        let Some(emphasis) = emphasis_node
            .as_element()
            .map(|el| &el.name)
            .and_then(|name| md_emphasis(name))
        else {
            return;
        };

        let cur_scope = EmphasisScope::from(emphasis);
        let em_opts = opts.include_node().inline().emphasis_scope(cur_scope);
        let mut emphasis_text = String::new();

        for child_node in node.children_it(false) {
            self.write(&mut emphasis_text, child_node.id, em_opts);
        }

        if emphasis_text.is_empty() {
            return;
        }

        let marker = if opts.emphasis_scope.contains(cur_scope) {
            ""
        } else {
            emphasis
        };
        push_emphasis(text, &mut emphasis_text, marker);
    }

    fn write_list_item(&self, text: &mut String, node_id: NodeId, ctx: &ListContext) {
        trim_right_tendril_space(text);
        text.push_str(ctx.indent);
        text.push_str(ctx.prefix);
        self.write(text, node_id, ctx.opts);
        text.push_str(ctx.linebreak);
    }

    fn write_list_item_blocks(&self, text: &mut String, node_id: NodeId, ctx: &ListContext) {
        let child_node = NodeRef::new(node_id, self.root_node.tree);

        let block_indent = " ".repeat(ctx.prefix.len());
        trim_right_tendril_space(text);
        text.push_str(ctx.indent);
        text.push_str(ctx.prefix);

        let mut is_first_block = true;
        for c in child_node.children_it(false) {
            let is_block = !node_is_list(&c) && node_is_md_block(&c);
            if is_block {
                if is_first_block {
                    is_first_block = false;
                } else {
                    text.push_str(&block_indent);
                }

                self.write(text, c.id, ctx.opts);
                text.push_str(ctx.linebreak);
                text.push_str(ctx.linebreak);
            } else {
                self.write(text, c.id, ctx.opts.include_node());
            }
        }
    }

    fn write_list(&self, text: &mut String, list_node: &TreeNode, prefix: &str, opts: FormatOpts) {
        let indent = " ".repeat(opts.offset * LIST_OFFSET_BASE);
        let ctx = ListContext {
            opts: opts.offset(opts.offset + 1),
            linebreak: linebreak(opts.br),
            indent: &indent,
            prefix,
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

            if is_list_item && has_blocks {
                self.write_list_item_blocks(text, child_id, &ctx);
            } else if is_list_item {
                self.write_list_item(text, child_id, &ctx);
            } else {
                self.write(text, child_id, FormatOpts::new().include_node());
            }
        }
    }

    fn write_link(&self, text: &mut String, link_node: &TreeNode) {
        let Some(el) = link_node.as_element() else {
            return;
        };

        let default_opts = FormatOpts::new();

        let Some(href) = el.attr("href") else {
            // no href: fallback to serializing node contents
            self.write(text, link_node.id, default_opts);
            return;
        };

        let mut link_text = String::new();
        let has_img = self.has_descendant_img(&link_node.id);

        let mut is_md_body = false;
        // try to collect plain text if there is no image
        if !has_img {
            let link_opts = FormatOpts::new().include_node();
            self.write_text(&mut link_text, link_node.id, link_opts);
        }
        // serialize as markdown if body contains images or non-text elements
        if has_img || link_text.is_empty() {
            self.write(&mut link_text, link_node.id, default_opts);
            is_md_body = true;
        }

        if link_text.is_empty() {
            return;
        }

        text.push('[');
        if is_md_body {
            text.push_str(&link_text);
        } else {
            push_normalized_text(text, &link_text, default_opts);
        }

        text.push_str("](");
        push_md_url(text, &href);

        if let Some(title) = el.attr_ref(local_name!("title")) {
            push_title(text, title);
        }

        text.push(')');
    }

    fn has_descendant_img(&self, id: &NodeId) -> bool {
        descendant_nodes(Ref::clone(&self.nodes), id).any(|child_id| {
            matches!(
                &self.nodes[child_id.value].data,
                NodeData::Element(e) if e.name.local == local_name!("img")
            )
        })
    }

    fn write_img(text: &mut String, img_node: &TreeNode) {
        let Some(el) = img_node.as_element() else {
            return;
        };

        let src = el
            .attr("src")
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                el.attr("srcset").and_then(|s| {
                    s.split_ascii_whitespace()
                        .next()
                        .map(|u| u.trim_end_matches(','))
                        .filter(|u| !u.is_empty())
                        .map(StrTendril::from_slice)
                })
            })
            .or_else(|| el.attr("data-src").filter(|s| !s.trim().is_empty()));
        if let Some(src) = src {
            text.push_str("![");

            if let Some(alt) = el.attr_ref(local_name!("alt")) {
                push_normalized_text(text, alt, FormatOpts::new());
            }
            text.push(']');
            text.push('(');
            push_md_url(text, &src);
            if let Some(title) = el.attr_ref(local_name!("title")) {
                push_title(text, title);
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
        text.push_str("\n```");
        if let Some(lang) = self.find_code_language(pre_node) {
            text.push_str(&lang);
        }
        text.push('\n');
        text.push_str(&TreeNodeOps::text_of(Ref::clone(&self.nodes), pre_node.id));
        text.push_str("\n```\n");
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

        push_code_text(text, &code_text);

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
        let mut headings = vec![];
        for th_ref in table_ref.find(&["tr", "th"]) {
            let mut th_text = String::new();
            self.write(&mut th_text, th_ref.id, opts);
            headings.push(th_text);
        }
        let mut rows = vec![];
        for tr_ref in table_ref.find(&["tr"]) {
            let mut row = vec![];
            for td_ref in tr_ref.find(&["td"]) {
                let mut td_text = String::new();
                self.write(&mut td_text, td_ref.id, opts);
                row.push(td_text);
            }
            if !row.is_empty() {
                rows.push(row);
            }
        }

        while headings.len() < rows[0].len() {
            headings.push(" ".into());
        }

        text.push_str("\n| ");

        let heading = headings.join(" | ");
        text.push_str(&heading);
        text.push_str(" |\n");
        text.push_str("| ");

        text.push_str(
            headings
                .iter()
                .map(|s| "-".repeat(s.len().max(1)))
                .collect::<Vec<_>>()
                .join(" | ")
                .as_str(),
        );
        text.push_str(" |\n");

        for row in rows {
            text.push_str("| ");
            text.push_str(&row.join(" | "));
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
    )
}

fn node_is_md_block(node: &NodeRef) -> bool {
    node.qual_name_ref().is_some_and(|name| is_md_block(&name))
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
        local_name!("hr") => "---",
        _ => "",
    };

    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

const fn md_emphasis(name: &QualName) -> Option<&'static str> {
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
        let curr_cell_count = row.find(&["td"]).len();
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
