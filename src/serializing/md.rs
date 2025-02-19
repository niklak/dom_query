use std::cell::Ref;

use html5ever::{local_name, QualName};
use tendril::StrTendril;

use crate::{NodeId, Tree, TreeNodeOps};

use crate::node::{SerializeOp, TreeNode};
use crate::node::{child_nodes, NodeData, NodeRef};



struct MDFormatter<'a> {
    root_node: &'a NodeRef<'a>,
    nodes: Ref<'a, Vec<TreeNode>>,
    include_node: bool
}

impl <'a>MDFormatter<'a> {
    fn new(root_node: &'a NodeRef, include_node: bool) -> MDFormatter<'a> {
        let nodes = root_node.tree.nodes.borrow();
        MDFormatter { root_node, nodes, include_node }
    }

    fn format(&self) -> StrTendril {
        let mut text = StrTendril::new();
        self.write(&mut text, self.root_node.id, self.include_node);
        text
    }

    fn write(&self, text: &mut StrTendril, root_id: NodeId, include_node: bool) {
        let mut ops = if include_node {
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
                            push_normalized_text(text, contents.as_ref());
                        }
                        NodeData::Element(ref e) => {
                            if !(text.is_empty() || text.ends_with("\n"))
                                && elem_require_linebreak(&e.name)
                            {
                                text.push_char('\n');
                            }
    
                            if let Some(prefix) = md_prefix(&e.name) {
                                text.push_slice(prefix);
                            }
    
                            if e.name.local == local_name!("ul") {
                                self.write_list(text, id, "- ");
                                continue;
                            }

                            if e.name.local == local_name!("ol") {
                                self.write_list(text, id, "1. ");
                                continue;
                            }

                            if e.name.local == local_name!("a") {
                                self.write_link(text, id);
                                continue;
                            }

                            if e.name.local == local_name!("img") {
                                self.write_img(text, id);
                                continue;
                            }

                            if e.name.local == local_name!("pre") {
                                self.write_pre(text, id);
                                continue;
                            }
    
                            ops.push(SerializeOp::Close(&e.name));
    
                            ops.extend(
                                child_nodes(Ref::clone(&self.nodes), &id, true).map(SerializeOp::Open),
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
                    if elem_require_linebreak(name) {
                        text.push_slice("\n\n");
                    }
                }
            }
        }
        if !include_node {
            while !text.is_empty() && text.ends_with(char::is_whitespace) {
                text.pop_back(1);
            }
        }
    }

    fn write_list(&self, text: &mut StrTendril, ul_node_id: NodeId, prefix: &str) {
        // TODO: what about ul inside ul
        for child_id in child_nodes(Ref::clone(&self.nodes), &ul_node_id, false) {
            let child_node = self.nodes.get(child_id.value).unwrap();
            let child_ref = NodeRef::new(child_id, self.root_node.tree);
            if let NodeData::Element(ref e) = child_node.data {
                if e.name.local == local_name!("li") {
                    trim_right_tendril_space(text);
                    text.push_slice(prefix);
                    self.write(text, child_id, false);
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
            }else {
                self.write(text, a_node_id, false);
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
            }else {
                self.write(text, img_node_id, false);
            }
        }

    }

    fn write_pre(&self, text: &mut StrTendril, pre_node_id: NodeId) {
        text.push_slice("\n```\n");
        text.push_tendril(&TreeNodeOps::text_of(Ref::clone(&self.nodes), pre_node_id));
        text.push_slice("\n```\n");
    }

}

pub(crate) fn format_md(root_node: &NodeRef, include_node: bool) -> StrTendril {
    MDFormatter::new(root_node, include_node).format()  
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
        local_name!("blockquote") => "> ",
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
        println!("body node: {}", body_node.text());
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
        let simple_expected = "My favorite search engine is [Duck Duck Go](https://duckduckgo.com).";
        html_2md_compare(&simple_contents, simple_expected);

        // link with title attribute
        let title_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com" title="Duck Duck Go">Duck Duck Go</a>.</p>"#;
        let title_expected = r#"My favorite search engine is [Duck Duck Go](https://duckduckgo.com "Duck Duck Go")."#;
        html_2md_compare(&title_contents, title_expected);

        let bold_contents = r#"<p>My favorite search engine is <b><a href="https://duckduckgo.com">Duck Duck Go</a></b>.</p>"#;
        let bold_expected = "My favorite search engine is **[Duck Duck Go](https://duckduckgo.com)**.";
        html_2md_compare(&bold_contents, bold_expected);

        // bold inside of link is not supported.
        let bold_ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><b>Duck Duck Go</b></a>.</p>"#;
        let bold_ignored_expected = "My favorite search engine is [Duck Duck Go](https://duckduckgo.com).";
        html_2md_compare(&bold_ignored_contents, bold_ignored_expected);

        // any elements inside `a` elements are also ignored,
        // html5ever transforms a > div to div > a, and there is no way to determine how it was.
        // This is an open question.
        let ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><div>Duck Duck Go</div></a>.</p>"#;
        let ignored_expected = "My favorite search engine is\n\n[Duck Duck Go](https://duckduckgo.com)\n\n.";
        html_2md_compare(&ignored_contents, ignored_expected);
    }

    #[test]
    fn test_images() {
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text"></p>"#;
        let simple_expected = "Image: ![Alt text](/path/to/img.jpg)";
        html_2md_compare(&simple_contents, simple_expected);

        // with title
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text" title="Title"></p>"#;
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
        let simple_expected = 
"
```
fn main() {
    println!(\"Hello, World!\");
}
```";
        html_2md_compare(&simple_contents, simple_expected);
    }

}

// TODO: escape characters
// TODO: table
// TODO: blockquote
