use std::cell::Ref;

use html5ever::{local_name, QualName};
use tendril::StrTendril;

use crate::{Element, NodeId, TreeNodeOps};

use crate::node::{ancestor_nodes, child_nodes, descendant_nodes, NodeData, NodeRef};
use crate::node::{SerializeOp, TreeNode};

const LIST_OFFSET_BASE: usize = 4;
const ESCAPE_CHARS: &[char] = &[
    '`', '*', '_', '{', '}', '[', ']', '<', '>', '(', ')', '#', '+', '.', '!', '|',
];
const DEFAULT_SKIP_TAGS: [&str; 4] = ["script", "style", "meta", "head"];
const CODE_LANGUAGE_ATTRIBUTES: [&str; 2] = ["data-lang", "data-language"];

#[derive(Default, Clone, Copy)]
struct Opts {
    include_node: bool,
    ignore_linebreak: bool,
    skip_escape: bool,
    offset: usize,
    br: bool,
}

impl Opts {
    fn new() -> Opts {
        Default::default()
    }

    fn include_node(mut self) -> Self {
        self.include_node = true;
        self
    }

    fn ignore_linebreak(mut self) -> Self {
        self.ignore_linebreak = true;
        self
    }

    fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    fn skip_escape(mut self) -> Self {
        self.skip_escape = true;
        self
    }
    fn br(mut self) -> Self {
        self.br = true;
        self
    }
}

pub(crate) struct MDSerializer<'a> {
    root_node: &'a NodeRef<'a>,
    nodes: Ref<'a, Vec<TreeNode>>,
    skip_tags: &'a [&'a str],
}

impl<'a> MDSerializer<'a> {
    pub fn new(root_node: &'a NodeRef, skip_tags: Option<&'a [&'a str]>) -> MDSerializer<'a> {
        let skip_tags = skip_tags.unwrap_or(&DEFAULT_SKIP_TAGS);
        let nodes = root_node.tree.nodes.borrow();
        MDSerializer {
            root_node,
            nodes,
            skip_tags,
        }
    }

    pub fn serialize(&self, include_node: bool) -> StrTendril {
        let mut text = StrTendril::new();
        let opts = Opts {
            include_node,
            ..Default::default()
        };
        self.write(&mut text, self.root_node.id, opts);
        text
    }

    fn write(&self, text: &mut StrTendril, root_id: NodeId, opts: Opts) {
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
                    match node.data {
                        NodeData::Text { ref contents } => {
                            push_normalized_text(text, contents.as_ref(), !opts.skip_escape);
                        }
                        NodeData::Element(ref e) => {
                            if self.skip_tags.contains(&e.name.local.as_ref()) {
                                continue;
                            }

                            let double_br = linebreak.repeat(2);

                            if !opts.ignore_linebreak && elem_require_double_linebreak(&e.name) {
                                add_linebreaks(text, linebreak, &double_br);
                            }

                            if let Some(prefix) = md_prefix(&e.name) {
                                text.push_slice(prefix);
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
                        text.push_slice(suffix);
                    }
                    let double_linebreak = linebreak.repeat(2);

                    if text.ends_with(&double_linebreak) {
                        continue;
                    }
                    if !opts.ignore_linebreak && elem_require_double_linebreak(name) {
                        add_linebreaks(text, linebreak, &double_linebreak);
                    } else if matches!(
                        name.local,
                        local_name!("br")
                            | local_name!("li")
                            | local_name!("tr")
                    ) {
                        // <br> handled as "   \n".
                        // **Fallback**: if `li` and `tr` are handled outside their context.
                        trim_right_tendril_space(text);
                        text.push_slice("  ");
                        text.push_slice(linebreak);
                    }
                }
            }
        }
        
        if !opts.include_node {
            while !text.is_empty() && text.ends_with(char::is_whitespace) {
                text.pop_back(1);
            }
            while !text.is_empty() && text.starts_with(char::is_whitespace) {
                text.pop_front(1);
            }
        }
    }

    fn write_text(&self, text: &mut StrTendril, root_id: NodeId, opts: Opts) {
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
        text: &mut StrTendril,
        e: &Element,
        tree_node: &TreeNode,
        opts: Opts,
    ) -> bool {
        let mut matched = true;

        match e.name.local {
            local_name!("ul") => {
                let list_prefix = if opts.br { "+ " } else { "- " };
                self.write_list(text, tree_node, list_prefix, opts)
            }
            local_name!("ol") => self.write_list(text, tree_node, "1. ", opts),
            local_name!("a") => self.write_link(text, tree_node),
            local_name!("img") => self.write_img(text, tree_node),
            local_name!("pre") => self.write_pre(text, tree_node),
            local_name!("blockquote") => self.write_blockquote(text, tree_node),
            local_name!("table") => self.write_table(text, tree_node),
            local_name!("code") => self.write_code(text, tree_node),
            _ => matched = false,
        }
        matched
    }

    fn write_list(&self, text: &mut StrTendril, list_node: &TreeNode, prefix: &str, opts: Opts) {
        let inline_opts = opts;
        let offset = opts.offset;
        let linebreak = linebreak(opts.br);
        let indent = " ".repeat(offset * LIST_OFFSET_BASE);
        for child_id in child_nodes(Ref::clone(&self.nodes), &list_node.id, false) {
            let child_node = &self.nodes[child_id.value];
            if let NodeData::Element(ref e) = child_node.data {
                if e.name.local == local_name!("li") {
                    trim_right_tendril_space(text);
                    text.push_slice(&indent);
                    text.push_slice(prefix);
                    self.write(text, child_id, inline_opts.offset(offset + 1));
                    text.push_slice(linebreak);
                    continue;
                }
            }
            self.write(text, child_id, Opts::new().include_node());
        }
    }

    fn write_link(&self, text: &mut StrTendril, link_node: &TreeNode) {
        let link_opts = Opts::new().include_node();
        if let NodeData::Element(ref e) = link_node.data {
            if let Some(href) = e.attr("href") {
                let mut link_text = StrTendril::new();
                self.write_text(&mut link_text, link_node.id, link_opts);
                if !link_text.is_empty() {
                    text.push_char('[');
                    push_normalized_text(text, &link_text, true);
                    text.push_char(']');
                    text.push_char('(');
                    text.push_tendril(&href);
                    if let Some(title) = e.attr("title") {
                        text.push_slice(" \"");
                        text.push_tendril(&title);
                        text.push_slice("\"");
                    }
                    text.push_char(')');
                }
            } else {
                self.write(text, link_node.id, Default::default());
            }
        }
    }

    fn write_img(&self, text: &mut StrTendril, img_node: &TreeNode) {
        if let NodeData::Element(ref e) = img_node.data {
            if let Some(src) = e.attr("src") {
                text.push_slice("![");
                if let Some(alt) = e.attr("alt") {
                    text.push_tendril(&alt);
                }
                text.push_char(']');
                text.push_char('(');
                text.push_tendril(&src);
                if let Some(title) = e.attr("title") {
                    text.push_slice(" \"");
                    text.push_tendril(&title);
                    text.push_slice("\"");
                }
                text.push_char(')');
            }
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

        ancestor_nodes(Ref::clone(&self.nodes), &node.id, Some(1))
            .find_map(|id| find_code_lang_attribute(&self.nodes[id.value]))
    }

    /// Transforms a `<pre>` code block, possibly with an associated language label that the resulting
    /// block is annotated with.
    fn write_pre(&self, text: &mut StrTendril, pre_node: &TreeNode) {
        text.push_slice("\n```");
        if let Some(lang) = self.find_code_language(pre_node) {
            text.push_slice(&lang);
        }
        text.push_char('\n');
        text.push_tendril(&TreeNodeOps::text_of(Ref::clone(&self.nodes), pre_node.id));
        text.push_slice("\n```\n");
    }

    /// Writes the content of the `<code>` block. Generally a `<code>` tag is used inline, but unfortunately
    /// it's also used instead of a `<pre>` block. In case the `<code>` block contains multiline
    /// text, it's handled as a `<pre>` code block.
    fn write_code(&self, text: &mut StrTendril, code_node: &TreeNode) {
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
        text.push_char('`');
        let mut code_text = StrTendril::new();
        self.write(&mut code_text, code_node.id, Opts::new().skip_escape());
        text.push_tendril(&code_text);
        text.push_char('`');
    }

    fn write_blockquote(&self, text: &mut StrTendril, quote_node: &TreeNode) {
        let opts = Opts::new();
        let mut quote_buf = StrTendril::new();
        self.write(&mut quote_buf, quote_node.id, opts);

        if quote_buf.is_empty() {
            return;
        }

        if !text.ends_with("\n\n") {
            text.push_slice("\n\n");
        }

        for line in quote_buf.lines() {
            text.push_slice("> ");
            text.push_slice(line);
            text.push_char('\n');
        }

        text.push_char('\n');
    }

    fn write_table(&self, text: &mut StrTendril, table_node: &TreeNode) {
        let table_ref = NodeRef::new(table_node.id, self.root_node.tree);

        if !is_table_node_writable(&table_ref) {
            self.write(text, table_node.id, Default::default());
            return;
        }

        let opts = Opts::new().ignore_linebreak().br();
        let mut headings = vec![];
        for th_ref in table_ref.find(&["tr", "th"]) {
            let mut th_text = StrTendril::new();
            self.write(&mut th_text, th_ref.id, opts);
            headings.push(th_text);
        }
        let mut rows = vec![];
        for tr_ref in table_ref.find(&["tr"]) {
            let mut row = vec![];
            for td_ref in tr_ref.find(&["td"]) {
                let mut td_text = StrTendril::new();
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

        text.push_slice("\n");
        text.push_slice("| ");

        let heading = join_tendril_strings(&headings, " | ");
        text.push_slice(&heading);
        text.push_slice(" |\n");

        text.push_slice("| ");

        text.push_slice(
            headings
                .iter()
                .map(|s| "-".repeat(s.len()))
                .collect::<Vec<_>>()
                .join(" | ")
                .as_str(),
        );
        text.push_slice(" |\n");

        for row in rows {
            text.push_slice("| ");
            text.push_slice(&join_tendril_strings(&row, " | "));
            text.push_slice(" |\n");
        }

        text.push_slice("\n");
    }
}

fn push_normalized_text(text: &mut StrTendril, new_text: &str, escape_all: bool) {
    let follows_newline = text.ends_with(['\n', ' ']) || text.is_empty();
    let push_start_whitespace = !follows_newline && new_text.starts_with(char::is_whitespace);
    let push_end_whitespace = new_text.ends_with(char::is_whitespace);

    let mut result = StrTendril::with_capacity(new_text.len() as u32);
    let mut iter = new_text.split_whitespace();

    if let Some(first) = iter.next() {
        if push_start_whitespace {
            result.push_char(' ');
        }
        push_escaped_chunk(&mut result, first, escape_all);
        for word in iter {
            result.push_char(' ');
            push_escaped_chunk(&mut result, word, escape_all);
        }
    }
    if result.is_empty() && follows_newline {
        return;
    }

    text.push_tendril(&result);

    if push_end_whitespace && !text.ends_with(char::is_whitespace) {
        text.push_char(' ');
    }
}

fn push_escaped_chunk(text: &mut StrTendril, chunk: &str, escape_all: bool) {
    let should_escape = if escape_all {
        |c: char| ESCAPE_CHARS.contains(&c)
    } else {
        |c: char| c == '`'
    };
    let mut prev_escape = false;
    for c in chunk.chars() {
        if should_escape(c) && !prev_escape {
            text.push_char('\\');
        }
        prev_escape = c == '\\';
        text.push_char(c);
    }
}

fn trim_right_tendril_space(s: &mut StrTendril) {
    while !s.is_empty() && s.ends_with(' ') {
        s.pop_back(1);
    }
}

fn elem_require_double_linebreak(name: &QualName) -> bool {
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

fn md_prefix(name: &QualName) -> Option<&'static str> {
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

fn md_suffix(name: &QualName) -> Option<&'static str> {
    let suffix = match name.local {
        local_name!("strong") | local_name!("b") => "**",
        local_name!("em") | local_name!("i") => "*",
        _ => "",
    };

    if suffix.is_empty() {
        None
    } else {
        Some(suffix)
    }
}

fn join_tendril_strings(seq: &[StrTendril], sep: &str) -> StrTendril {
    let mut result = StrTendril::new();
    let mut iter = seq.iter();

    if let Some(first) = iter.next() {
        result.push_tendril(first);
    }

    for tendril in iter {
        result.push_slice(sep);
        result.push_tendril(tendril);
    }
    result
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

fn linebreak(br: bool) -> &'static str {
    if br {
        "<br>"
    } else {
        "\n"
    }
}

fn add_linebreaks(text: &mut StrTendril, linebreak: &str, end: &str) {
    trim_right_tendril_space(text);
    while !text.ends_with(&end) {
        text.push_slice(linebreak);
    }
}

fn find_code_lang_attribute(node: &TreeNode) -> Option<String> {
    node.as_element()?
        .attrs
        .iter()
        .find(|attr| CODE_LANGUAGE_ATTRIBUTES.contains(&attr.name.local.as_ref()))
        .map(|attr| sanitize_attr_value(&attr.value))
}

/// Keep only the first whitespace‑delimited token and a conservative set of characters.
fn sanitize_attr_value(raw: &str) -> String {
    let token = raw.split_ascii_whitespace().next().unwrap_or("");
    token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.' | '#'))
        .collect()
}

pub(crate) fn serialize_md(
    root_node: &NodeRef,
    include_node: bool,
    skip_tags: Option<&[&str]>,
) -> StrTendril {
    MDSerializer::new(root_node, skip_tags).serialize(include_node)
}

#[cfg(test)]
mod tests {

    use tendril::StrTendril;

    use crate::Document;

    use super::*;

    #[track_caller]
    fn html_2md_compare(html_contents: &str, expected: &str) {
        let doc = Document::fragment(html_contents);
        let root_node = &doc.root();
        let md_text = serialize_md(root_node, false, None);
        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_escape_text() {
        let t = r"Some text with characters to be escaped: \,`,*,_,{,},[,],<,>,(,),#,+,.,!,|";
        let mut text = StrTendril::new();
        push_normalized_text(&mut text, t, true);
        assert_eq!(
            text.as_ref(),
            r"Some text with characters to be escaped: \,\`,\*,\_,\{,\},\[,\],\<,\>,\(,\),\#,\+,\.,\!,\|"
        );

        // test with escape_all: false
        let mut text = StrTendril::new();
        push_normalized_text(&mut text, t, false);
        assert_eq!(
            text.as_ref(),
            r"Some text with characters to be escaped: \,\`,*,_,{,},[,],<,>,(,),#,+,.,!,|"
        );
    }

    #[test]
    fn test_headings() {
        // when passing include_node: true, leading and trailing whitespaces will be kept.
        let contents = r"<h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
        <h4>Heading 4</h4>
        <h5>Heading 5</h5>
        <h6>Heading 6</h6>
        <h3><span>III.</span> Heading With Span</h3>
        <h3><span></span>Early years (2006–2009)</h3>
        <h3><span> </span> Early years (2006–2009)</h3>
        <hr>";

        let expected = "\n\n# Heading 1\n\n\
        ## Heading 2\n\n\
        ### Heading 3\n\n\
        #### Heading 4\n\n\
        ##### Heading 5\n\n\
        ###### Heading 6\n\n\
        ### III\\. Heading With Span\n\n\
        ### Early years \\(2006–2009\\)\n\n\
        ### Early years \\(2006–2009\\)\n\n\
        ---\n\n";

        let doc = Document::from(contents);
        let body_sel = &doc.select("body");
        let body_node = body_sel.nodes().first().unwrap();
        let md_text = serialize_md(body_node, true, None);
        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_italic() {
        let contents = r"<h4><i>Italic Text</i></h4>";
        let expected = "#### *Italic Text*";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_span_italic() {
        let contents = r"<span>It`s like <i>that</i></span>";
        let expected = r"It\`s like *that*";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_bold_italic() {
        let contents = r"<span>It`s like <b><i>that</i></b></span>";
        let expected = r"It\`s like ***that***";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_simple_code() {
        let contents = r"<span>It`s like <code>that</code></span>";
        let expected = r"It\`s like `that`";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_false_multiline_code() {
        let contents = 
        r"<span>
        It`s like 
        <code>
        that
        </code>
        </span>";
        let expected = r"It\`s like `that`";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_multiline_code() {
        let contents = r"<code>$ cargo new hello
    Created binary (application) `hello` package

$ cd hello</code>";
        let expected = r"```
$ cargo new hello
    Created binary (application) `hello` package

$ cd hello
```";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_ul() {
        let contents = "<h3>Pizza Margherita Ingredients</h3>\
        <ul>\
            <li>Pizza Dough</li>\
            <li>Mozzarella cheese</li>\
            <li>Tomatoes</li>\
            <li>Olive Oil</li>\
            <li><i>Basil</i></li>\
            <li><b>Salt</b></li>\
        </ul>";

        let expected = "### Pizza Margherita Ingredients\n\n\
        - Pizza Dough\n\
        - Mozzarella cheese\n\
        - Tomatoes\n\
        - Olive Oil\n\
        - *Basil*\n\
        - **Salt**";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_ol() {
        let contents = "<h3>Pizza Margherita Ingredients</h3>\
        <ol>\
            <li>Pizza Dough</li>\
            <li>Mozzarella cheese</li>\
            <li>Tomatoes</li>\
            <li>Olive Oil</li>\
            <li><i>Basil</i></li>\
            <li><b>Salt</b></li>\
        </ol>";

        let expected = "### Pizza Margherita Ingredients\n\n\
        1. Pizza Dough\n\
        1. Mozzarella cheese\n\
        1. Tomatoes\n\
        1. Olive Oil\n\
        1. *Basil*\n\
        1. **Salt**";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_bad_ol() {
        let contents = "<h3>Pizza Margherita Ingredients</h3>\
        <ol>\
            <li>Pizza Dough</li>\
            <li>Mozzarella cheese</li>\
            <li>Tomatoes</li>\
            <li>Olive Oil</li>\
            <div><i>Basil</i></div>\
            <li><b>Salt</b></li>\
        </ol>";

        let expected = "### Pizza Margherita Ingredients\n\n\
        1. Pizza Dough\n\
        1. Mozzarella cheese\n\
        1. Tomatoes\n\
        1. Olive Oil\n\
        \n*Basil*\n\n\
        1. **Salt**";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_list_inline() {
        let contents = "
        <ol>\
            <li>One</li>\
            <li>Two</li>\
            <li>Tree\
                <div>\
                    <ol>\
                        <li>One</li>\
                        <li>Two</li>\
                        <li>Tree\
                            <ol>\
                                <li>One</li>\
                                <li>Two</li>\
                                <li>Tree</li>\
                            </ol>
                        </li>\
                    </ol>\
                </div>\
            </li>\
        </ol>";

        let expected = "1. One
1. Two
1. Tree

    1. One
    1. Two
    1. Tree

        1. One
        1. Two
        1. Tree";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_paragraphs() {
        let contents =
            "<p>To create paragraphs, use a blank line to separate one or more lines of text.</p>
        <p>I really like using <span>Markdown</span><span>  text</span>.</p>

        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected =
            "To create paragraphs, use a blank line to separate one or more lines of text\\.\n\n\
        I really like using Markdown text\\.\n\n\
        I think I'll use it to format all of my documents from now on\\.";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_links() {
        let simple_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com">Duck Duck Go</a>.</p>"#;
        let simple_expected =
            r"My favorite search engine is [Duck Duck Go](https://duckduckgo.com)\.";
        html_2md_compare(simple_contents, simple_expected);

        // link with title attribute
        let title_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com" title="Duck Duck Go">Duck Duck Go</a>.</p>"#;
        let title_expected = r#"My favorite search engine is [Duck Duck Go](https://duckduckgo.com "Duck Duck Go")\."#;
        html_2md_compare(title_contents, title_expected);

        let bold_contents = r#"<p>My favorite search engine is <b><a href="https://duckduckgo.com">Duck Duck Go</a></b>.</p>"#;
        let bold_expected =
            r"My favorite search engine is **[Duck Duck Go](https://duckduckgo.com)**\.";
        html_2md_compare(bold_contents, bold_expected);

        // bold inside of link is not supported.
        let bold_ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><b>Duck Duck Go</b></a>.</p>"#;
        let bold_ignored_expected =
            r"My favorite search engine is [Duck Duck Go](https://duckduckgo.com)\.";
        html_2md_compare(bold_ignored_contents, bold_ignored_expected);

        // any elements inside `a` elements are also ignored,
        // html5ever transforms a > div to div > a, and there is no way to determine how it was.
        // This is an open question.
        let ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><div>Duck Duck Go</div></a>.</p>"#;
        let ignored_expected =
            "My favorite search engine is\n\n[Duck Duck Go](https://duckduckgo.com)\n\n\\.";
        html_2md_compare(ignored_contents, ignored_expected);

        let no_href_contents = r#"<p>My favorite search engine is <a>Duck Duck Go</a>.</p>"#;
        let no_href_expected = "My favorite search engine is Duck Duck Go\\.";
        html_2md_compare(no_href_contents, no_href_expected);
    }

    #[test]
    fn test_images() {
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text"></p>"#;
        let simple_expected = "Image: ![Alt text](/path/to/img.jpg)";
        html_2md_compare(simple_contents, simple_expected);

        // with title
        let simple_contents =
            r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text" title="Title"></p>"#;
        let simple_expected = r#"Image: ![Alt text](/path/to/img.jpg "Title")"#;
        html_2md_compare(simple_contents, simple_expected);

        // without alt
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg"></p>"#;
        let simple_expected = r#"Image: ![](/path/to/img.jpg)"#;
        html_2md_compare(simple_contents, simple_expected);

        // no img
        let simple_contents = r#"<p>Image:  <img alt="Alt text" title="Title"></p>"#;
        let simple_expected = "Image:";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code() {
        let simple_contents = "<pre>\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\
</pre>";
        let simple_expected = "```
fn main() {
    println!(\"Hello, World!\");
}
```";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code_with_data_lang_attribute() {
        let simple_contents = "<pre data-lang=\"rust\">\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\n\
</pre>";
        let simple_expected = "```rust
fn main() {
    println!(\"Hello, World!\");
}

```";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code_with_data_lang_attribute_in_parent_tag() {
        let simple_contents = "<div data-lang=\"rust\"><pre>\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\n\
</pre></div>";
        let simple_expected = "```rust
fn main() {
    println!(\"Hello, World!\");
}

```";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_blockquote() {
        let simple_contents = "<blockquote><p>Quoted text</p></blockquote>";
        let simple_expected = "> Quoted text";
        html_2md_compare(simple_contents, simple_expected);

        let complex_contents = "<blockquote>
<p>
Who has seen the wind?<br>
Neither I nor you:<br>
But when the leaves hang trembling,<br>
The wind is passing through.
</p>
<p>
Who has seen the wind?<br>
Neither you nor I:<br>
But when the trees bow down their heads,<br>
The wind is passing by.
</p>
</blockquote>
<p><i>Christina Rossetti</i></p>";
        let complex_expected = r"> Who has seen the wind?  
> Neither I nor you:  
> But when the leaves hang trembling,  
> The wind is passing through\.
> 
> Who has seen the wind?  
> Neither you nor I:  
> But when the trees bow down their heads,  
> The wind is passing by\.

*Christina Rossetti*";
        html_2md_compare(complex_contents, complex_expected);

        let empty_contents = "<blockquote></blockquote>";
        let empty_expected = "";
        html_2md_compare(empty_contents, empty_expected);
    }

    #[test]
    fn test_inline_blockquote() {
        let contents = "<blockquote>
<p>
Who has seen the wind?<br>
Neither I nor you:<br>
But when the leaves hang trembling,<br>
The wind is passing through.
</p>
<blockquote>
<p>
Who has seen the wind?<br>
Neither you nor I:<br>
But when the trees bow down their heads,<br>
The wind is passing by.
</p>
</blockquote>
</blockquote>";
        let expected = r"> Who has seen the wind?  
> Neither I nor you:  
> But when the leaves hang trembling,  
> The wind is passing through\.
> 
> > Who has seen the wind?  
> > Neither you nor I:  
> > But when the trees bow down their heads,  
> > The wind is passing by\.";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table() {
        let contents = "<table>
    <tr>
        <th>Column 1</th>
        <th>Column 2</th>
        <th>Column 3</th>
    </tr>
    <tr>
        <td>R 1, <i>C 1</i></td>
        <td>R 1, <i>C 2</i></td>
        <td>R 1, <i>C 3</i></td>
    </tr>
    <tr>
        <td>R 2, <i>C 1</i></td>
        <td>R 2, <i>C 2</i></td>
        <td>R 2, <i>C 3</i></td>
    </tr>
</table>";
        let expected = "| Column 1 | Column 2 | Column 3 |
| -------- | -------- | -------- |
| R 1, *C 1* | R 1, *C 2* | R 1, *C 3* |
| R 2, *C 1* | R 2, *C 2* | R 2, *C 3* |";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_inside_table() {
        let contents = "<table>
    <tr>
        <td>
            <table>
                <tr>
                    <th>Column 1</th>
                    <th>Column 2</th>
                    <th>Column 3</th>
                </tr>
                <tr>
                    <td>R 1, <i>C 1</i></td>
                    <td>R 1, <i>C 2</i></td>
                    <td>R 1, <i>C 3</i></td>
                </tr>
                <tr>
                    <td>R 2, <i>C 1</i></td>
                    <td>R 2, <i>C 2</i></td>
                    <td>R 2, <i>C 3</i></td>
                </tr>
            </table>
        </td>
    </tr>
</table>";
        let expected = "| Column 1 | Column 2 | Column 3 |
| -------- | -------- | -------- |
| R 1, *C 1* | R 1, *C 2* | R 1, *C 3* |
| R 2, *C 1* | R 2, *C 2* | R 2, *C 3* |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_without_headings() {
        let contents = "<table>
    <tr>
        <td>R 1, <i>C 1</i></td>
        <td>R 1, <i>C 2</i></td>
        <td>R 1, <i>C 3</i></td>
    </tr>
    <tr>
        <td>R 2, <i>C 1</i></td>
        <td>R 2, <i>C 2</i></td>
        <td>R 2, <i>C 3</i></td>
    </tr>
</table>";
        let expected = "|   |   |   |
| - | - | - |
| R 1, *C 1* | R 1, *C 2* | R 1, *C 3* |
| R 2, *C 1* | R 2, *C 2* | R 2, *C 3* |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_skip() {
        let contents = "<table>
    <tr>
        <td>R 1, <i>C 1</i></td>
        <td>R 1, <i>C 2</i></td>
        <td>R 1, <i>C 3</i></td>
    </tr>
    <tr>
        <td>R 2, <i>C 1</i></td>
        <td>R 2, <i>C 2</i></td>
    </tr>
</table>";
        let expected = "R 1, *C 1* R 1, *C 2* R 1, *C 3*  
R 2, *C 1* R 2, *C 2*";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_empty() {
        let contents = "<table>
    <tr></tr>
    <tr></tr>
</table>";
        let expected = "";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_with_list() {
        let contents = "<table>
    <tr>
        <td>1</td>
        <td>
            <ul><li>Lemon</li></ul>
            <ul><li>Lime</li></ul>
            <ul><li>Grapefruit</li></ul>
            <ul><li>Orange</li></ul>
        </td>
    </tr>
</table>";
        let expected =
            "|   |   |\n| - | - |\n| 1 | + Lemon<br>+ Lime<br>+ Grapefruit<br>+ Orange<br> |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_skip_tags_default() {
        // By default, formatter will skip ["script", "style", "meta", "head"]
        let contents = "
        <style>p {color: blue;}</style>
        <p>I really like using <b>Markdown</b>.</p>

        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected = "I really like using **Markdown**\\.\n\n\
        I think I'll use it to format all of my documents from now on\\.";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_skip_tags() {
        // If you need all text content of the elements, you need to pass Some(&vec![]) to `md`.
        // If you pass a structure like this into `Document::from`, the html5ever will create html > head > style.
        // If you want to preserve order use `Document::fragment`.
        let contents = "<style>p {color: blue;}</style>\
        <div><h1>Content Heading<h1></div>\
        <p>I really like using Markdown.</p>\
        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected = "p \\{color: blue;\\}\n\n\
        I really like using Markdown\\.\n\n\
        I think I'll use it to format all of my documents from now on\\.";

        let doc = Document::fragment(contents);
        let html_node = &doc.root();
        let md_text = serialize_md(html_node, false, Some(&["div"]));
        assert_eq!(md_text.as_ref(), expected);
    }
    #[test]
    fn test_linebreak_after_lists() {
        let contents = r#"Influenced
        <ul>
         <li>Idris (programming language)</li>
         <li>Project Verona</li>
         <li>Spark</li>
         <li>Swift</li>
         <li>V</li>
         <li>Zig</li>
        </ul>
        <p><b>Rust</b> is a general-purpose programming language</p>"#;
        let expected = "Influenced\n\n\
- Idris \\(programming language\\)
- Project Verona
- Spark
- Swift
- V
- Zig

**Rust** is a general-purpose programming language";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_pre_code_without_new_line() {
        let simple_contents = r#"<pre>
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>
<span>    </span><span>println!</span><span>(</span><span>"Hello, World!"</span><span>);</span>
<span>}</span></pre>"#;
        let simple_expected = "```
fn main() {
    println!(\"Hello, World!\");
}
```";
        html_2md_compare(simple_contents, simple_expected);
    }
}
