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
                                self.write_ul(text, id);
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
                        text.push_slice("\n");
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

    fn write_ul(&self, text: &mut StrTendril, ul_node_id: NodeId) {

        for child_id in child_nodes(Ref::clone(&self.nodes), &ul_node_id, false) {
            let child_node = self.nodes.get(child_id.value).unwrap();
            let child_ref = NodeRef::new(child_id, self.root_node.tree);
            if let NodeData::Element(ref e) = child_node.data {
                if e.name.local == local_name!("li") {
                    trim_right_tendril_space(text);
                    text.push_slice("- ");
                    self.write(text, child_id, false);
                    text.push_char('\n');
                    continue;
                }
            }
            text.push_tendril(&format_md(&child_ref, true));
        }
        text.push_char('\n');
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
    matches!(
        name.local,
        local_name!("article")
            | local_name!("blockquote")
            | local_name!("section")
            | local_name!("div")
            | local_name!("p")
            | local_name!("pre")
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

    #[test]
    fn test_headings() {

        let contents = r"<h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
        <h4>Heading 4</h4>
        <h5>Heading 5</h5>
        <h6>Heading 6</h6>";
        let doc = Document::from(contents);
        
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, true);
        let expected = "# Heading 1\n\
        ## Heading 2\n\
        ### Heading 3\n\
        #### Heading 4\n\
        ##### Heading 5\n\
        ###### Heading 6\n";

        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_italic() {

        let contents = r"<h4><i>Italic Text</i></h4>";
        let doc = Document::from(contents);
        
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, true);
        let expected = "#### *Italic Text*\n";

        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_span_italic() {

        let contents = r"<span>It`s like <i>that</i></span>";
        let doc = Document::from(contents);
        
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, true);
        let expected = "It`s like *that*";

        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_bold_italic() {

        let contents = r"<span>It`s like <b><i>that</i></b></span>";
        let doc = Document::from(contents);
        
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, true);
        let expected = "It`s like ***that***";

        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_simple_code() {

        let contents = r"<span>It`s like <code>that</code></span>";
        let doc = Document::from(contents);
        
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, true);
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
        let doc = Document::from(contents);
        
        let body_sel = doc.select_single("body");
        let body_node = body_sel.nodes().first().unwrap();

        let md_text = format_md(body_node, true);
        let expected = "### Pizza Margherita Ingredients\n\
        - Pizza Dough\n\
        - Mozzarella cheese\n\
        - Tomatoes\n\
        - Olive Oil\n\
        - *Basil*\n\
        - **Salt**\n\n";

        assert_eq!(md_text.as_ref(), expected);
    }

}

// TOOO: escape characters