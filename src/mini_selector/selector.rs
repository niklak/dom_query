use nom::IResult;

use crate::{node::TreeNode, Element, NodeRef};

use super::parser::parse_single_selector;

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

impl AttrOperator {
    fn match_attr(&self, elem_value: &str, value: &str) -> bool {
        if elem_value.is_empty() || value.is_empty() {
            return false;
        }
        let e = elem_value.as_bytes();
        let s = value.as_bytes();

        match self {
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
            AttrOperator::Substring => elem_value.contains(value),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Attribute<'a> {
    pub key: &'a str,
    pub op: Option<AttrOperator>,
    pub value: Option<&'a str>,
}

/// Current support of CSS is limited: it supports only the `child` (`>`) and `descendant` (` `) combinators.
/// It does not support the `selector list` combinator (`,`) or any pseudo-classes.
/// Each selector in the chain may contain at most one attribute selector.
#[derive(Debug, PartialEq)]
pub struct MiniSelector<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) id: Option<&'a str>,
    pub(crate) classes: Option<Vec<&'a str>>,
    pub(crate) attr: Option<Attribute<'a>>,
    pub(crate) combinator: Combinator,
}

impl MiniSelector<'_> {
    /// Parses a single CSS selector string and returns a `MiniSelector` representing the parsed selector.
    ///
    /// # Arguments
    ///
    /// * `css_sel` - The CSS selector string to parse.
    ///
    /// # Returns
    ///
    /// A nom `IResult` containing the parsed `MiniSelector` if the CSS selector string is valid, or an error if it is not.
    pub fn new(css_sel: &str) -> IResult<&str, MiniSelector> {
        parse_single_selector(css_sel)
    }

    pub(crate) fn match_tree_node(&self, t: &TreeNode) -> bool {
        if let Some(el) = t.as_element() {
            self.match_name(el)
                && self.match_id_attr(el)
                && self.match_classes(el)
                && self.match_attr(el)
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
        if let Some(id) = self.id {
            if let Some(id_attr) = el.id() {
                return id_attr.as_ref() == id;
            }else {
                return false;
            }
        }
        true
    }
    fn match_classes(&self, el: &Element) -> bool {
        let Some(ref classes) = self.classes else {
            return true;
        };
        classes.iter().all(|class| el.has_class(class))
    }

    fn match_attr(&self, el: &Element) -> bool {
        if let Some(Attribute { key, ref op, value }) = self.attr {
            if let (Some(op), Some(v)) = (op, value) {
                return el
                    .attrs
                    .iter()
                    .any(|a| &a.name.local == key && op.match_attr(&a.value, v));
            } else {
                return el.has_attr(key);
            }
        }
        true
    }
}
