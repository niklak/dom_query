use std::cell::Ref;

use html5ever::{local_name, QualName};
use tendril::StrTendril;

use crate::{Element, NodeId, TreeNodeOps};

use crate::node::{child_nodes, NodeData, NodeRef};
use crate::node::{SerializeOp, TreeNode};

static LIST_OFFSET_BASE: usize = 4;

#[derive(Default, Clone, Copy)]
struct Opts<'a> {
    include_node: bool,
    ignore_linebreak: bool,
    offset: usize,
    prefix: &'a str,
}

impl <'a>Opts<'a> {
    fn new() -> Opts<'a> {
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

    fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = prefix;
        self
    }
}

struct MDFormatter<'a> {
    root_node: &'a NodeRef<'a>,
    nodes: Ref<'a, Vec<TreeNode>>,
    include_node: bool,
}

impl<'a> MDFormatter<'a> {
    fn new(root_node: &'a NodeRef, include_node: bool) -> MDFormatter<'a> {
        let nodes = root_node.tree.nodes.borrow();
        MDFormatter {
            root_node,
            nodes,
            include_node,
        }
    }

    fn format(&self, include_node: bool) -> StrTendril {
        let mut text = StrTendril::new();
        let opts = Opts{include_node, ..Default::default()};
        self.write(&mut text, self.root_node.id, opts);
        text
    }

    fn write(&self, text: &mut StrTendril, root_id: NodeId, opts: Opts) {
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
                    let node = match self.nodes.get(id.value) {
                        Some(node) => node,
                        None => continue,
                    };
                    match node.data {
                        NodeData::Text { ref contents } => {
                            if !opts.prefix.is_empty()  && !contents.trim().is_empty(){
                                text.push_slice(opts.prefix);
                            }
                            push_normalized_text(text, contents.as_ref());
                        }
                        NodeData::Element(ref e) => {

                            if !opts.ignore_linebreak && !(text.is_empty() || text.ends_with("\n")) && elem_require_linebreak(&e.name) {
                                text.push_char('\n');
                            }
                    
                            if let Some(prefix) = md_prefix(&e.name) {
                                text.push_slice(prefix);
                            }
                    
                            if self.write_element(text, e, id, opts.offset) {
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
                    trim_right_tendril_space(text);
                    if let Some(suffix) = md_suffix(name) {
                        text.push_slice(suffix);
                    }
                    if !opts.ignore_linebreak && elem_require_linebreak(name) {
                        text.push_slice("\n\n");
                    }

                    if matches!(name.local,local_name!("br") | local_name!("hr")) {
                        // TODO: Careful!
                        text.push_char('\n');
                    }
                }
            }
        }
        if !opts.include_node {
            while !text.is_empty() && text.ends_with(char::is_whitespace) {
                text.pop_back(1);
            }
        }
    }

    fn write_element(
        &self,
        text: &mut StrTendril,
        e: &Element,
        e_node_id: NodeId,
        offset: usize,
    ) -> bool {

        let mut matched = true;

        match e.name.local {
            local_name!("ul") => self.write_list(text, e_node_id, "- ", offset),
            local_name!("ol") => self.write_list(text, e_node_id, "1. ", offset),
            local_name!("a") => self.write_link(text, e_node_id),
            local_name!("img") => self.write_img(text, e_node_id),
            local_name!("pre") => self.write_pre(text, e_node_id),
            local_name!("blockquote") => self.write_blockquote(text, e_node_id),
            _ => matched = false,
        }
        matched
    }

    fn write_list(&self, text: &mut StrTendril, ul_node_id: NodeId, prefix: &str, offset: usize) {
        for child_id in child_nodes(Ref::clone(&self.nodes), &ul_node_id, false) {
            let child_node = self.nodes.get(child_id.value).unwrap();
            let child_ref = NodeRef::new(child_id, self.root_node.tree);
            if let NodeData::Element(ref e) = child_node.data {
                if e.name.local == local_name!("li") {
                    trim_right_tendril_space(text);
                    text.push_slice(&" ".repeat(offset * LIST_OFFSET_BASE));
                    text.push_slice(prefix);
                    self.write(text, child_id, Opts::new().offset(offset + 1));
                    text.push_char('\n');
                    continue;
                }
            }
            text.push_tendril(&format_md(&child_ref, true));
        }
        text.push_char('\n');
    }

    fn write_link(&self, text: &mut StrTendril, a_node_id: NodeId) {
        let link_node = self.nodes.get(a_node_id.value).unwrap();
        if let NodeData::Element(ref e) = link_node.data {
            if let Some(href) = e.attr("href") {
                let link_text = TreeNodeOps::text_of(Ref::clone(&self.nodes), a_node_id);
                if !link_text.is_empty() {
                    text.push_char('[');
                    push_normalized_text(text, &link_text);
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
                self.write(text, a_node_id, Default::default());
            }
        }
    }

    fn write_img(&self, text: &mut StrTendril, img_node_id: NodeId) {
        let img_node = self.nodes.get(img_node_id.value).unwrap();
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
            } else {
                self.write(text, img_node_id, Default::default());
            }
        }
    }

    fn write_pre(&self, text: &mut StrTendril, pre_node_id: NodeId) {
        text.push_slice("\n```\n");
        text.push_tendril(&TreeNodeOps::text_of(Ref::clone(&self.nodes), pre_node_id));
        text.push_slice("\n```\n");
    }

    fn write_blockquote(&self, text: &mut StrTendril,quote_node_id: NodeId) {
        let opts = Opts::new().ignore_linebreak().prefix("> ");
        let mut children = child_nodes(Ref::clone(&self.nodes), &quote_node_id, false);
        let node = self.nodes.get(quote_node_id.value).unwrap();
        let mut require_linebreak = false;
        while let Some(child_id) = children.next() {
            if require_linebreak && node.last_child != Some(child_id) {
                require_linebreak = false;
                text.push_slice("\n>");
            }
            text.push_char('\n');
            self.write(text, child_id, opts);
            let element = self.nodes.get(child_id.value).and_then(|n| n.as_element());
            if let Some(el) = element {
                if elem_require_linebreak(&el.name) {
                    require_linebreak = true;
                }
            }
            
        }
    }
}

pub(crate) fn format_md(root_node: &NodeRef, include_node: bool) -> StrTendril {
    MDFormatter::new(root_node, include_node).format(include_node)
}

fn push_normalized_text(text: &mut StrTendril, new_text: &str) {
    let follows_newline = text.ends_with(['\n', ' ']) || text.is_empty();
    let push_start_whitespace = !follows_newline && new_text.starts_with(char::is_whitespace);
    let push_end_whitespace = new_text.ends_with(char::is_whitespace);

    let mut result = StrTendril::with_capacity(new_text.len() as u32);
    let mut iter = new_text.split_whitespace();

    if let Some(first) = iter.next() {
        if push_start_whitespace {
            result.push_char(' ');
        }
        result.push_slice(first);
        for word in iter {
            result.push_char(' ');
            result.push_slice(word);
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

fn trim_right_tendril_space(s: &mut StrTendril) {
    while !s.is_empty() && s.ends_with(' ') {
        s.pop_back(1);
    }
}

fn adjust_element_offset(text: &mut StrTendril, name: &QualName) {
    if elem_require_linebreak(name) {
        trim_right_tendril_space(text);
        text.push_slice("\n\n");
    } else if matches!(
        name.local,
        local_name!("br") | local_name!("hr") | local_name!("li") | local_name!("tr")
    ) {
        trim_right_tendril_space(text);
        text.push_char('\n');
    } else if matches!(name.local, local_name!("td") | local_name!("th"))
        && !text.ends_with(['\n', ' '])
    {
        text.push_char(' ');
    }
}

fn elem_require_linebreak(name: &QualName) -> bool {
    // TODO: since div is a very common element it is a very special element.
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
        local_name!("code") => "`",
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
        local_name!("code") => "`",
        _ => "",
    };

    if suffix.is_empty() {
        None
    } else {
        Some(suffix)
    }
}

#[cfg(test)]
mod tests {

    use crate::Document;

    use super::format_md;

    fn html_2md_compare(html_contents: &str, expected: &str) {
        let doc = Document::from(html_contents);
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();
        let md_text = format_md(body_node, false);
        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_headings() {
        let contents = r"<h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
        <h4>Heading 4</h4>
        <h5>Heading 5</h5>
        <h6>Heading 6</h6>";

        let expected = "# Heading 1\n\n\
        ## Heading 2\n\n\
        ### Heading 3\n\n\
        #### Heading 4\n\n\
        ##### Heading 5\n\n\
        ###### Heading 6";

        html_2md_compare(&contents, expected);
    }

    #[test]
    fn test_italic() {
        let contents = r"<h4><i>Italic Text</i></h4>";
        let expected = "#### *Italic Text*";

        html_2md_compare(&contents, expected);
    }

    #[test]
    fn test_span_italic() {
        let contents = r"<span>It`s like <i>that</i></span>";
        let expected = "It`s like *that*";

        html_2md_compare(&contents, expected);
    }

    #[test]
    fn test_bold_italic() {
        let contents = r"<span>It`s like <b><i>that</i></b></span>";
        let expected = "It`s like ***that***";

        html_2md_compare(&contents, expected);
    }

    #[test]
    fn test_simple_code() {
        let contents = r"<span>It`s like <code>that</code></span>";
        let doc = Document::from(contents);

        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, false);
        let expected = "It`s like `that`";

        assert_eq!(md_text.as_ref(), expected);
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

        html_2md_compare(&contents, expected);
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

        html_2md_compare(&contents, expected);
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

        html_2md_compare(&contents, expected);
    }

    #[test]
    fn test_paragraphs() {
        let contents = "<p>I really like using Markdown.</p>

        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected = "I really like using Markdown.\n\n\
        I think I'll use it to format all of my documents from now on.";

        html_2md_compare(&contents, expected);
    }

    #[test]
    fn test_links() {
        let simple_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com">Duck Duck Go</a>.</p>"#;
        let simple_expected =
            "My favorite search engine is [Duck Duck Go](https://duckduckgo.com).";
        html_2md_compare(&simple_contents, simple_expected);

        // link with title attribute
        let title_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com" title="Duck Duck Go">Duck Duck Go</a>.</p>"#;
        let title_expected = r#"My favorite search engine is [Duck Duck Go](https://duckduckgo.com "Duck Duck Go")."#;
        html_2md_compare(&title_contents, title_expected);

        let bold_contents = r#"<p>My favorite search engine is <b><a href="https://duckduckgo.com">Duck Duck Go</a></b>.</p>"#;
        let bold_expected =
            "My favorite search engine is **[Duck Duck Go](https://duckduckgo.com)**.";
        html_2md_compare(&bold_contents, bold_expected);

        // bold inside of link is not supported.
        let bold_ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><b>Duck Duck Go</b></a>.</p>"#;
        let bold_ignored_expected =
            "My favorite search engine is [Duck Duck Go](https://duckduckgo.com).";
        html_2md_compare(&bold_ignored_contents, bold_ignored_expected);

        // any elements inside `a` elements are also ignored,
        // html5ever transforms a > div to div > a, and there is no way to determine how it was.
        // This is an open question.
        let ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><div>Duck Duck Go</div></a>.</p>"#;
        let ignored_expected =
            "My favorite search engine is\n\n[Duck Duck Go](https://duckduckgo.com)\n\n.";
        html_2md_compare(&ignored_contents, ignored_expected);
    }

    #[test]
    fn test_images() {
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text"></p>"#;
        let simple_expected = "Image: ![Alt text](/path/to/img.jpg)";
        html_2md_compare(&simple_contents, simple_expected);

        // with title
        let simple_contents =
            r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text" title="Title"></p>"#;
        let simple_expected = r#"Image: ![Alt text](/path/to/img.jpg "Title")"#;
        html_2md_compare(&simple_contents, simple_expected);

        // without alt
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg"></p>"#;
        let simple_expected = r#"Image: ![](/path/to/img.jpg)"#;
        html_2md_compare(&simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code() {
        let simple_contents = "<pre>\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\
</pre>";
        let simple_expected = "
```
fn main() {
    println!(\"Hello, World!\");
}
```";
        html_2md_compare(&simple_contents, simple_expected);
    }


    #[test]
    fn test_blockquote() {
        let simple_contents = "<blockquote><p>Quoted text</p></blockquote>";
        let simple_expected = "\n> Quoted text";
        html_2md_compare(&simple_contents, simple_expected);


        let complex_contents = 
"<blockquote>
<p>
Who has seen the wind?<br>
Neither I nor you:<br>
But when the leaves hang trembling,<br>
The wind is passing through.<br> 
</p>
<p>
Who has seen the wind?<br>
Neither you nor I:<br>
But when the trees bow down their heads,<br>
The wind is passing by.<br>
</p>
</blockquote>";
        let complex_expected = 
"
> Who has seen the wind?
> Neither I nor you:
> But when the leaves hang trembling,
> The wind is passing through.
>
> Who has seen the wind?
> Neither you nor I:
> But when the trees bow down their heads,
> The wind is passing by.";
        html_2md_compare(&complex_contents, complex_expected);


    }


    #[test]
    fn test_inline_blockquote() {
        // TODO: inline blockquote elements currently are not supported
        let contents = 
"<blockquote>
<p>
Who has seen the wind?<br>
Neither I nor you:<br>
But when the leaves hang trembling,<br>
The wind is passing through.<br> 
</p>
<blockquote>
<p>
Who has seen the wind?<br>
Neither you nor I:<br>
But when the trees bow down their heads,<br>
The wind is passing by.<br>
</p>
</blockquote>
</blockquote>";
        let expected = 
"
> Who has seen the wind?
> Neither I nor you:
> But when the leaves hang trembling,
> The wind is passing through.
>
> Who has seen the wind?
> Neither you nor I:
> But when the trees bow down their heads,
> The wind is passing by.";
        html_2md_compare(&contents, expected);


    }
    
}

// TODO: escape characters
// TODO: table
