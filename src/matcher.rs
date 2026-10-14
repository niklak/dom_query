use std::fmt;

use bit_set::BitSet;
use cssparser::{CowRcStr, ParseError, ToCss};
use html5ever::Namespace;
use selectors::context::SelectorCaches;
use selectors::parser::{self, Component, LocalName, SelectorList, SelectorParseErrorKind};
use selectors::{Element, context, matching};

use crate::Tree;
use crate::css::{CssLocalName, CssString};
use crate::node::{DescendantNodes, NodeRef};
/// CSS selector.
#[derive(Clone, Debug)]
pub struct Matcher {
    selector_list: SelectorList<InnerSelector>,
    // set when every selector in the list is a bare type selector (`p`, `ul,ol`),
    // so an element matches iff its local name is one of these
    local_names: Option<Vec<LocalName<InnerSelector>>>,
}

impl Matcher {
    /// creates a new CSS matcher.
    ///
    /// # Errors
    /// - [`cssparser::ParseError`] if the selector string is invalid.
    pub fn new(sel: &str) -> Result<Self, ParseError<SelectorParseErrorKind>> {
        let mut parser = cssparser::Parser::new(sel);
        selectors::parser::SelectorList::parse(
            &InnerSelectorParser,
            &mut parser,
            parser::ParseRelative::No,
        )
        .map(|selector_list| Self {
            local_names: type_selector_names(&selector_list),
            selector_list,
        })
    }

    /// Checks if an element matches Matcher's selection.
    pub fn match_element<E>(&self, element: &E) -> bool
    where
        E: Element<Impl = InnerSelector>,
    {
        let mut caches = context::SelectorCaches::default();
        self.match_element_with_caches(element, &mut caches)
    }

    /// Checks if an element matches Matcher's selection.
    pub fn match_element_with_caches<E>(&self, element: &E, caches: &mut SelectorCaches) -> bool
    where
        E: Element<Impl = InnerSelector>,
    {
        if let Some(names) = &self.local_names {
            return names.iter().any(|n| {
                element.has_local_name(matching::select_name(element, &n.name, &n.lower_name))
            });
        }
        let mut ctx = get_matching_context(caches);
        matching::matches_selector_list(&self.selector_list, element, &mut ctx)
    }
}

fn type_selector_names(
    list: &SelectorList<InnerSelector>,
) -> Option<Vec<LocalName<InnerSelector>>> {
    list.slice()
        .iter()
        .map(
            |selector| match selector.iter_raw_match_order().as_slice() {
                [Component::LocalName(name)] => Some(name.clone()),
                _ => None,
            },
        )
        .collect()
}

pub struct DescendantMatches<'a, 'b> {
    iter: DescendantNodes<'a>,
    matcher: &'b Matcher,
    caches: SelectorCaches,
    tree: &'a Tree,
}

impl<'a, 'b> DescendantMatches<'a, 'b> {
    pub fn new(root_node: NodeRef<'a>, matcher: &'b Matcher) -> Self {
        // Optimized for single-root node scenario - no duplicate checking needed
        let tree = root_node.tree;
        Self {
            iter: tree.descendant_ids_of_it(&root_node.id),
            matcher,
            caches: SelectorCaches::default(),
            tree,
        }
    }
}

impl<'a> Iterator for DescendantMatches<'a, '_> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for node_id in self.iter.by_ref() {
            let node = NodeRef::new(node_id, self.tree);
            if node.is_element()
                && self
                    .matcher
                    .match_element_with_caches(&node, &mut self.caches)
            {
                return Some(node);
            }
        }
        None
    }
}

pub struct Matches<'a, 'b> {
    nodes: Vec<NodeRef<'a>>,
    matcher: &'b Matcher,
    seen: BitSet,
    caches: SelectorCaches,
}

impl<'a, 'b> Matches<'a, 'b> {
    pub fn new<I: Iterator<Item = NodeRef<'a>>>(root_nodes: I, matcher: &'b Matcher) -> Self {
        // Used for multiple root nodes where duplicate checking is necessary
        let nodes: Vec<NodeRef<'a>> = root_nodes
            .flat_map(|node| node.children_it(true).filter(|n| n.is_element()))
            .collect();
        // every visited node is marked, so size the set for the whole tree once
        let tree_len = nodes.first().map_or(0, |n| n.tree.len() + 1);

        Self {
            nodes,
            matcher,
            seen: BitSet::with_capacity(tree_len),
            caches: SelectorCaches::default(),
        }
    }
}

impl<'a> Iterator for Matches<'a, '_> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.nodes.pop() {
            // a node reached again through a nested root was already visited with its subtree
            if !self.seen.insert(node.id.value()) {
                continue;
            }
            self.nodes
                .extend(node.children_it(true).filter(|n| n.is_element()));

            if self
                .matcher
                .match_element_with_caches(&node, &mut self.caches)
            {
                return Some(node);
            }
        }
        None
    }
}

pub(crate) struct InnerSelectorParser;

impl<'i> parser::Parser<'i> for InnerSelectorParser {
    type Impl = InnerSelector;
    type Error = parser::SelectorParseErrorKind;

    fn parse_is_and_where(&self) -> bool {
        true
    }

    fn parse_has(&self) -> bool {
        true
    }

    fn parse_non_ts_pseudo_class(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<NonTSPseudoClass, ParseError<Self::Error>> {
        if name.eq_ignore_ascii_case("any-link") {
            Ok(NonTSPseudoClass::AnyLink)
        } else if name.eq_ignore_ascii_case("link") {
            Ok(NonTSPseudoClass::Link)
        } else if name.eq_ignore_ascii_case("visited") {
            Ok(NonTSPseudoClass::Visited)
        } else if name.eq_ignore_ascii_case("active") {
            Ok(NonTSPseudoClass::Active)
        } else if name.eq_ignore_ascii_case("focus") {
            Ok(NonTSPseudoClass::Focus)
        } else if name.eq_ignore_ascii_case("hover") {
            Ok(NonTSPseudoClass::Hover)
        } else if name.eq_ignore_ascii_case("enabled") {
            Ok(NonTSPseudoClass::Enabled)
        } else if name.eq_ignore_ascii_case("disabled") {
            Ok(NonTSPseudoClass::Disabled)
        } else if name.eq_ignore_ascii_case("checked") {
            Ok(NonTSPseudoClass::Checked)
        } else if name.eq_ignore_ascii_case("indeterminate") {
            Ok(NonTSPseudoClass::Indeterminate)
        } else if name.eq_ignore_ascii_case("only-text") {
            Ok(NonTSPseudoClass::OnlyText)
        } else {
            Err(ParseError::custom(
                SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }

    fn parse_non_ts_functional_pseudo_class(
        &self,
        name: CowRcStr<'i>,
        parser: &mut cssparser::Parser<'i>,
        _after_part: bool,
    ) -> Result<<Self::Impl as parser::SelectorImpl>::NonTSPseudoClass, ParseError<Self::Error>>
    {
        if name.eq_ignore_ascii_case("has-text") {
            let s = parser.expect_string()?.as_ref();
            Ok(NonTSPseudoClass::HasText(CssString::from(s)))
        } else if name.eq_ignore_ascii_case("contains") {
            {
                let s = parser.expect_string()?.as_ref();
                Ok(NonTSPseudoClass::Contains(CssString::from(s)))
            }
        } else {
            Err(ParseError::custom(
                SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InnerSelector;

impl parser::SelectorImpl for InnerSelector {
    type ExtraMatchingData<'a> = ();
    type AttrValue = CssString;
    type Identifier = CssLocalName;
    type LocalName = CssLocalName;
    type NamespaceUrl = Namespace;
    type NamespacePrefix = CssLocalName;
    type BorrowedLocalName = CssLocalName;
    type BorrowedNamespaceUrl = Namespace;

    type NonTSPseudoClass = NonTSPseudoClass;
    type PseudoElement = PseudoElement;
}

/// Non-tree-structural pseudo-classes.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum NonTSPseudoClass {
    /// `:any-link` means one of the a, area, or link elements that has an href attribute.
    AnyLink,
    /// `:link` means same as `:any-link`
    Link,
    Visited,
    Active,
    Focus,
    Hover,
    Enabled,
    Disabled,
    Checked,
    Indeterminate,
    /// `:only-text` pseudo-class allows selecting a node with no child elements except a single **text** child node.
    OnlyText,
    /// `:has-text` pseudo-class represents a selection for the element or one of its descendant element that contains the specified text.
    HasText(CssString),
    /// `:contains` pseudo-class represents a selection for the element that contains the specified text (it's own text and text of all his descendant elements).
    Contains(CssString),
}

impl ToCss for NonTSPseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match self {
            Self::AnyLink => dest.write_str(":any-link"),
            Self::Link => dest.write_str(":link"),
            Self::Visited => dest.write_str(":visited"),
            Self::Active => dest.write_str(":active"),
            Self::Focus => dest.write_str(":focus"),
            Self::Hover => dest.write_str(":hover"),
            Self::Enabled => dest.write_str(":enabled"),
            Self::Disabled => dest.write_str(":disabled"),
            Self::Checked => dest.write_str(":checked"),
            Self::Indeterminate => dest.write_str(":indeterminate"),
            Self::OnlyText => dest.write_str(":only-text"),
            Self::HasText(s) => {
                dest.write_str(":has-text(")?;
                s.to_css(dest)?;
                dest.write_str(")")
            }
            Self::Contains(s) => {
                dest.write_str(":contains(")?;
                s.to_css(dest)?;
                dest.write_str(")")
            }
        }
    }
}

impl parser::NonTSPseudoClass for NonTSPseudoClass {
    fn is_active_or_hover(&self) -> bool {
        false
    }

    fn is_user_action_state(&self) -> bool {
        false
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct PseudoElement;

impl ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str("")
    }
}

impl parser::PseudoElement for PseudoElement {
    fn accepts_state_pseudo_classes(&self) -> bool {
        false
    }

    fn valid_after_slotted(&self) -> bool {
        false
    }
}

fn get_matching_context(
    caches: &mut context::SelectorCaches,
) -> matching::MatchingContext<'_, InnerSelector> {
    matching::MatchingContext::new(
        matching::MatchingMode::Normal,
        None,
        caches,
        matching::QuirksMode::NoQuirks,
        matching::NeedsSelectorFlags::No,
        context::MatchingForInvalidation::No,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn type_selector_fast_path_agrees_with_the_selectors_crate() {
        let doc = Document::from(
            r##"<div class="a"><p id="x">t<a href="#">l</a></p><UL><li>1</li><li>2</li></UL>
            <svg><foreignObject><p>f</p></foreignObject><circle/></svg>
            <TABLE><tr><td>c</td><Td>d</Td></tr></TABLE><img><iframe></iframe></div><p>outside</p>"##,
        );
        let selectors = [
            "p",
            "P",
            "ul,ol",
            "img,object,embed,iframe",
            "circle",
            "foreignObject",
            "FOREIGNOBJECT",
            "svg, circle",
            "*",
            "p.a",
            "div p",
            "p:first-child",
            "div, p.a",
        ];
        for sel in selectors {
            let fast = Matcher::new(sel).unwrap();
            let slow = Matcher {
                selector_list: fast.selector_list.clone(),
                local_names: None,
            };
            let mut caches = SelectorCaches::default();
            for node in doc.root().descendants_it().filter(NodeRef::is_element) {
                assert_eq!(
                    fast.match_element_with_caches(&node, &mut caches),
                    slow.match_element_with_caches(&node, &mut caches),
                    "{sel} on {:?}",
                    node.node_name()
                );
            }
        }
        for sel in ["p", "ul,ol", "foreignObject", "svg, circle"] {
            assert!(Matcher::new(sel).unwrap().local_names.is_some(), "{sel}");
        }
        for sel in ["*", "p.a", "div p", "p:first-child", "#x", "div, p.a"] {
            assert!(Matcher::new(sel).unwrap().local_names.is_none(), "{sel}");
        }
    }
}
