use std::cell::Ref;
use html5ever::local_name;

use crate::{node::TreeNode, Element, NodeId, NodeRef, TreeNodeOps};
use super::{parse_selector_list, parser::parse_mini_selector};

static SELECTOR_WHITESPACE: &[char] = &[' ', '\t', '\n', '\r', '\x0C'];

#[derive(Debug, PartialEq)]
pub(crate) enum AttrOperator {
    Equals,    // =
    Includes,  // ~=
    DashMatch, // |=
    Prefix,    // ^=
    Suffix,    // $=
    Substring, // *=
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum Combinator {
    Descendant, // " "
    Child,      // ">"
    Adjacent,   // "+"
    Sibling,    // "~"
}

#[derive(Debug, PartialEq)]
pub(crate) struct AttrValue<'a> {
    pub op: AttrOperator,
    pub value: &'a str,
}

impl AttrValue<'_> {
    pub(crate) fn is_match(&self, elem_value: &str) -> bool {
        if elem_value.is_empty() {
            return false;
        }
        let e = elem_value.as_bytes();
        let s = self.value.as_bytes();

        match self.op {
            AttrOperator::Equals => e == s,
            AttrOperator::Includes => elem_value
                .split(SELECTOR_WHITESPACE)
                .any(|part| part.as_bytes() == s),
            AttrOperator::DashMatch => {
                e == s
                    || (e.starts_with(s) && e.len() > s.len() && &e[s.len()..s.len() + 1] == b"-")
            }
            AttrOperator::Prefix => e.starts_with(s),
            AttrOperator::Suffix => e.ends_with(s),
            AttrOperator::Substring => elem_value.contains(self.value),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Attribute<'a> {
    pub key: &'a str,
    pub value: Option<AttrValue<'a>>,
}

/// Current support of CSS is limited: it supports only the `child` (`>`) and `descendant` (` `) combinators.
/// It does not support the `selector list` combinator (`,`) or any pseudo-classes.
#[derive(Debug, PartialEq)]
pub struct MiniSelector<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) id: Option<&'a str>,
    pub(crate) classes: Option<Vec<&'a str>>,
    pub(crate) attrs: Option<Vec<Attribute<'a>>>,
    pub(crate) combinator: Combinator,
}

impl<'a> MiniSelector<'a> {
    /// Parses a single CSS selector string and returns a [`MiniSelector`] representing the parsed selector.
    ///
    /// # Arguments
    ///
    /// * `css_sel` - The CSS selector string to parse.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `MiniSelector` if the CSS selector string is valid, or an [nom::Err] if it is not.
    pub fn new(css_sel: &'a str) -> Result<Self, nom::Err<nom::error::Error<&'a str>>> {
        let (_, sel) = parse_mini_selector(css_sel)?;
        Ok(sel)
    }
}

impl MiniSelector<'_> {
    pub(crate) fn match_tree_node(&self, t: &TreeNode) -> bool {
        if let Some(el) = t.as_element() {
            self.match_name(el)
                && self.match_id_attr(el)
                && self.match_classes(el)
                && self.match_attrs(el)
        } else {
            false
        }
    }

    /// Checks if a `NodeRef` matches the `MiniSelector`.
    ///
    /// # Arguments
    ///
    /// * `node_ref` - The `NodeRef` to check.
    ///
    /// # Returns
    ///
    /// `true` if `node_ref` matches the `MiniSelector`, `false` otherwise.
    pub fn match_node(&self, node_ref: &NodeRef) -> bool {
        let nodes = node_ref.tree.nodes.borrow();
        let tree_node = &nodes[node_ref.id.value];
        self.match_tree_node(tree_node)
    }

    fn match_name(&self, el: &Element) -> bool {
        self.name.map_or(true, |name| &el.name.local == name)
    }

    fn match_id_attr(&self, el: &Element) -> bool {
        let Some(id) = self.id else {
            return true;
        };
        el.attr_ref(local_name!("id")).is_some_and(|v| v == id)
    }

    fn match_classes(&self, el: &Element) -> bool {
        let Some(ref classes) = self.classes else {
            return true;
        };
        let Some(class_val) = el.attr_ref(local_name!("class")) else {
            return false;
        };
        classes
            .iter()
            .all(|class| class_val.split_ascii_whitespace().any(|c| c == *class))
    }

    fn match_attrs(&self, el: &Element) -> bool {
        let Some(ref attrs) = self.attrs else {
            return true;
        };
        let mut is_ok = true;
        for attr in attrs {
            let key = attr.key;
            is_ok = match &attr.value {
                Some(attr_value) => el
                    .attrs
                    .iter()
                    .any(|a| &a.name.local == key && attr_value.is_match(&a.value)),
                _ => el.has_attr(key),
            };
            if !is_ok {
                break;
            }
        }
        is_ok
    }
}

pub struct MiniSelectorList<'a>(pub Vec<MiniSelector<'a>>);

impl<'a> MiniSelectorList<'a> {
    /// Parses a string with a list of CSS selector and returns a [`MiniSelectorList`] representing the parsed selector list.
    ///
    /// # Arguments
    ///
    /// * `css_sel` - The CSS selector string to parse.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the parsed `MiniSelectorList` if the CSS selector string is valid, or an [nom::Err] if it is not.
    pub fn new(css_sel: &'a str) -> Result<Self, nom::Err<nom::error::Error<&'a str>>> {
        let (_, mut sel) = parse_selector_list(css_sel)?;
        sel.reverse();
        Ok(MiniSelectorList(sel))
    }
}

impl MiniSelectorList<'_> {
    pub fn match_tree_node(&self, node_id: NodeId, nodes: &Ref<Vec<TreeNode>>) -> bool {
        let mut cur_node_id = Some(node_id);
        let mut matched = false;
        let mut is_direct = false;
        for selector in self.0.iter() {
            while let Some(ref id) = cur_node_id {
                let Some(t) = nodes.get(id.value).filter(|t| t.is_element()) else {
                    return false;
                };
                matched = selector.match_tree_node(t);
                if matched {
                    cur_node_id = match selector.combinator {
                        Combinator::Child | Combinator::Descendant => t.parent,
                        Combinator::Adjacent | Combinator::Sibling => {
                            TreeNodeOps::prev_element_sibling_of(nodes, id)
                        }
                    };
                    break;
                }
                if node_id == t.id || is_direct {
                    return false;
                }
                cur_node_id = t.parent;
            }
            is_direct = matches!(
                selector.combinator,
                Combinator::Child | Combinator::Adjacent
            );
        }
        matched
    }

    pub fn match_node(&self, node: &NodeRef) -> bool {
        let nodes = node.tree.nodes.borrow();
        self.match_tree_node(node.id, &nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn test_selector_list_match() {
        let contents = r#"<div>
            <p>Some text <span><a id="main-link" href="https://example.com/main-page/" target>Example</a></span></p>
        </div>"#;
        let doc = Document::fragment(contents);
        let link_sel = doc.select_single(r#"a[id]"#);
        let link_node = link_sel.nodes().first().unwrap();

        let css_path_0 = "div > p > span > a";
        let selector_list_0 = MiniSelectorList::new(css_path_0).unwrap();
        assert!(selector_list_0.match_node(&link_node));

        let css_path_1 = "div > p a";
        let selector_list_1 = MiniSelectorList::new(css_path_1).unwrap();
        assert!(selector_list_1.match_node(&link_node));
    }

    #[test]
    fn test_selector_list_combinator_match() {
        let contents = r#"<ul>
            <li><a id="item-1">Item 1</a></li>
            <li><a id="item-2">Item 2</a></li>
            <li><a id="item-3">Item 3</a></li>
            <li><a id="item-4">Item 4</a></li>
            <li><a id="item-5">Item 6</a></li>
        </ul>"#;
        let doc = Document::fragment(contents);
        let link_sel = doc.select(r#"a[id]"#);
        let link_node = link_sel.nodes().get(2).unwrap();

        let css_path_0 = "a";
        let selector_list_0 = MiniSelectorList::new(css_path_0).unwrap();
        assert!(selector_list_0.match_node(&link_node));

        let css_path_1 = "ul li a";
        let selector_list_1 = MiniSelectorList::new(css_path_1).unwrap();
        assert!(selector_list_1.match_node(&link_node));

        let css_path_2 = "ul li + li a";
        let selector_list_2 = MiniSelectorList::new(css_path_2).unwrap();
        assert!(selector_list_2.match_node(&link_node));
    }
}
