use std::ops::Deref;

use html5ever::{local_name, namespace_url, ns};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::context::MatchingContext;
use selectors::matching::{matches_selector_list, ElementSelectorFlags};
use selectors::parser::SelectorImpl;
use selectors::{OpaqueElement, SelectorList};

use crate::css::CssLocalName;
use crate::dom_tree::{Node, NodeData, NodeRef};
use crate::matcher::{InnerSelector, NonTSPseudoClass};

impl<'a> selectors::Element for Node<'a> {
    type Impl = InnerSelector;

    // Converts self into an opaque representation.
    #[inline]
    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(&self.id)
    }
    
    #[inline]
    fn parent_element(&self) -> Option<Self> {
        self.parent()
    }

    // Whether the parent node of this element is a shadow root.
    #[inline]
    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    // The host of the containing shadow root, if any.
    #[inline]
    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    // Whether we're matching on a pseudo-element.
    #[inline]
    fn is_pseudo_element(&self) -> bool {
        false
    }

    // Skips non-element nodes.
    #[inline]
    fn prev_sibling_element(&self) -> Option<Self> {
        self.prev_element_sibling()
    }

    // Skips non-element nodes.
    #[inline]
    fn next_sibling_element(&self) -> Option<Self> {
        self.next_element_sibling()
    }

    #[inline]
    fn is_html_element_in_html_document(&self) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return e.name.ns == ns!(html);
            }
            false
        })
    }

    #[inline]
    fn has_local_name(&self, local_name: &<Self::Impl as SelectorImpl>::BorrowedLocalName) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return &e.name.local == local_name.deref();
            }
            false
        })
    }

    // Empty string for no namespace.
    #[inline]
    fn has_namespace(&self, ns: &<Self::Impl as SelectorImpl>::BorrowedNamespaceUrl) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return &e.name.ns == ns;
            }
            false
        })
    }

    // Whether this element and the `other` element have the same local name and namespace.

    fn is_same_type(&self, other: &Self) -> bool {
        //TODO: maybe we should unpack compare_node directly here
        self.tree
            .compare_node(&self.id, &other.id, |a, b| {
                if let (NodeData::Element(ref e1), NodeData::Element(ref e2)) = (&a.data, &b.data) {
                    e1.name == e2.name
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&<Self::Impl as SelectorImpl>::NamespaceUrl>,
        local_name: &<Self::Impl as SelectorImpl>::LocalName,
        operation: &AttrSelectorOperation<&<Self::Impl as SelectorImpl>::AttrValue>,
    ) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return e.attrs.iter().any(|attr| match *ns {
                    NamespaceConstraint::Specific(url) if *url != attr.name.ns => false,
                    _ => *local_name.as_ref() == attr.name.local && operation.eval_str(&attr.value),
                });
            }
            false
        })
    }

    fn match_non_ts_pseudo_class(
        &self,
        pseudo: &<Self::Impl as SelectorImpl>::NonTSPseudoClass,
        context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        use self::NonTSPseudoClass::*;
        match pseudo {
            Active | Focus | Hover | Enabled | Disabled | Checked | Indeterminate | Visited => {
                false
            }
            AnyLink | Link => match self.node_name() {
                Some(node_name) => {
                    matches!(node_name.deref(), "a" | "area" | "link")
                        && self.attr("href").is_some()
                }
                None => false,
            },
            Has(list) => {
                //it checks only in descendants
                has_descendant_match(self, list, context)
            }
            HasText(s) => self.has_text(s.as_str()),
            Contains(s) => self.text().contains(s.as_str()),
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &<Self::Impl as SelectorImpl>::PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    // Whether this element is a `link`.
    fn is_link(&self) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return matches!(
                    e.name.local,
                    local_name!("a") | local_name!("area") | local_name!("link")
                ) && e
                    .attrs
                    .iter()
                    .any(|attr| attr.name.local == local_name!("href"));
            }
            false
        })
    }

    // Whether the element is an HTML element.
    fn is_html_slot_element(&self) -> bool {
        true
    }

    fn has_id(
        &self,
        name: &<Self::Impl as SelectorImpl>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return e.attrs.iter().any(|attr| {
                    attr.name.local.deref() == "id"
                        && case_sensitivity.eq(name.as_bytes(), attr.value.as_bytes())
                });
            }
            false
        })
    }

    fn has_class(
        &self,
        name: &<Self::Impl as SelectorImpl>::LocalName,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        self.query_or(false, |node| {
            if let NodeData::Element(ref e) = node.data {
                return e
                    .attrs
                    .iter()
                    .find(|a| a.name.local == local_name!("class"))
                    .map_or(false, |a| {
                        a.value
                            .deref()
                            .split_whitespace()
                            .any(|c| case_sensitivity.eq(name.as_bytes(), c.as_bytes()))
                    });
            }

            false
        })
    }

    // Returns the mapping from the `exportparts` attribute in the regular direction, that is, outer-tree->inner-tree.
    fn imported_part(&self, _name: &CssLocalName) -> Option<CssLocalName> {
        None
    }

    fn is_part(&self, _name: &CssLocalName) -> bool {
        false
    }

    // Whether this element matches `:empty`.
    fn is_empty(&self) -> bool {
        !self
            .children()
            .iter()
            .any(|child| child.is_element() || child.is_text())
    }

    // Whether this element matches `:root`, i.e. whether it is the root element of a document.
    fn is_root(&self) -> bool {
        self.is_document()
    }

    fn first_element_child(&self) -> Option<Self> {
        self.children()
            .iter()
            .find(|&child| child.is_element())
            .cloned()
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}
}

fn has_descendant_match(
    n: &NodeRef<NodeData>,
    selectors_list: &SelectorList<InnerSelector>,
    ctx: &mut MatchingContext<InnerSelector>,
) -> bool {
    let mut node = n.first_child();
    while let Some(ref n) = node {
        if matches_selector_list(selectors_list, n, ctx)
            || (n.is_element() && has_descendant_match(n, selectors_list, ctx))
        {
            return true;
        }
        node = n.next_sibling();
    }
    false
}
