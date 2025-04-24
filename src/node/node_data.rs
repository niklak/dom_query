use std::fmt::Debug;
use std::ops::Deref;

#[allow(unused_imports)]
use html5ever::namespace_url;
use html5ever::LocalName;
use html5ever::{local_name, ns, Attribute, QualName};
use selectors::attr::CaseSensitivity;
use tendril::StrTendril;

use super::NodeId;
use crate::entities::{into_tendril, wrap_attrs, wrap_tendril, Attr, InnerHashSet, StrWrap};

fn contains_class(classes: &str, target_class: &str) -> bool {
    classes.split_whitespace().any(|c| c == target_class)
}

/// The different kinds of nodes in the DOM.
#[derive(Debug, Clone)]
pub enum NodeData {
    /// The `Tree` itself - the root node of a HTML tree.
    Document,

    /// A root of the html fragment
    Fragment,

    /// A `DOCTYPE` with name, public id, and system id. See
    /// [tree type declaration on wikipedia][dtd wiki].
    ///
    /// [dtd wiki]: https://en.wikipedia.org/wiki/Tree_type_declaration
    Doctype {
        name: StrWrap,
        public_id: StrWrap,
        system_id: StrWrap,
    },

    /// A text node.
    Text { contents: StrWrap },

    /// A comment.
    Comment { contents: StrWrap },

    /// An element with attributes.
    Element(Element),

    /// A Processing instruction.
    ProcessingInstruction { target: StrWrap, contents: StrWrap },
}

/// An element with attributes.
#[derive(Debug, Clone)]
pub struct Element {
    pub name: QualName,
    pub attrs: Vec<Attr>,

    /// For HTML \<template\> elements, the [template contents].
    ///
    /// [template contents]: https://html.spec.whatwg.org/multipage/#template-contents
    pub template_contents: Option<NodeId>,

    /// Whether the node is a [HTML integration point].
    ///
    /// [HTML integration point]: https://html.spec.whatwg.org/multipage/#html-integration-point
    #[allow(dead_code)]
    mathml_annotation_xml_integration_point: bool,
}

impl Element {
    /// Create a new element.
    pub fn new(
        name: QualName,
        attrs: Vec<Attribute>,
        template_contents: Option<NodeId>,
        mathml_annotation_xml_integration_point: bool,
    ) -> Element {
        Element {
            name,
            attrs: wrap_attrs(attrs),
            template_contents,
            mathml_annotation_xml_integration_point,
        }
    }

    /// The name of the node.
    pub fn node_name(&self) -> StrTendril {
        StrTendril::from(self.name.local.as_ref())
    }

    /// Get the class attribute of the node.
    pub fn class(&self) -> Option<StrTendril> {
        self.attrs
            .iter()
            .find(|a| a.name.local == local_name!("class"))
            .map(|a| into_tendril(a.value.clone()))
    }

    /// Get the id attribute of the node.
    pub fn id(&self) -> Option<StrTendril> {
        self.attrs
            .iter()
            .find(|a| a.name.local == local_name!("id"))
            .map(|a| into_tendril(a.value.clone()))
    }

    /// Whether the element has the given class.
    pub fn has_class(&self, class: &str) -> bool {
        self.attrs
            .iter()
            .find(|a| a.name.local == local_name!("class"))
            .map_or(false, |attr| contains_class(&attr.value, class))
    }

    /// Whether the element has the given class.
    pub fn has_class_bytes(&self, name: &[u8], case_sensitivity: CaseSensitivity) -> bool {
        self.attrs
            .iter()
            .find(|a| a.name.local == local_name!("class"))
            .map_or(false, |a| {
                a.value
                    .deref()
                    .split_whitespace()
                    .any(|c| case_sensitivity.eq(name, c.as_bytes()))
            })
    }

    /// Add a class to the element.
    pub fn add_class(&mut self, classes: &str) {
        if classes.trim().is_empty() {
            return;
        }

        let class_set: InnerHashSet<&str> = classes
            .split(' ')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let attr = self
            .attrs
            .iter_mut()
            .find(|attr| attr.name.local == local_name!("class"));

        match attr {
            Some(attr) => {
                let value = &mut attr.value;
                for item in class_set {
                    if !contains_class(value, item) {
                        value.push_slice(" ");
                        value.push_slice(item);
                    }
                }
            }
            None => {
                let classes: Vec<&str> = class_set.into_iter().collect();
                let value = StrTendril::from(classes.join(" "));
                // The namespace on the attribute name is almost always ns!().
                let name = QualName::new(None, ns!(), local_name!("class"));
                self.attrs.push(Attr {
                    name,
                    value: wrap_tendril(value),
                });
            }
        }
    }

    /// Remove a class from the element.
    pub fn remove_class(&mut self, class: &str) {
        if class.trim().is_empty() {
            return;
        }

        if let Some(attr) = self
            .attrs
            .iter_mut()
            .find(|attr| attr.name.local == local_name!("class"))
        {
            let mut set: InnerHashSet<&str> = attr
                .value
                .split(' ')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let removes = class.split(' ').map(|s| s.trim()).filter(|s| !s.is_empty());

            for remove in removes {
                set.remove(remove);
            }

            attr.value = wrap_tendril(StrTendril::from(
                set.into_iter().collect::<Vec<&str>>().join(" "),
            ));
        }
    }

    /// Gets the specified attribute's value.
    pub fn attr(&self, name: &str) -> Option<StrTendril> {
        self.attrs
            .iter()
            .find(|attr| &attr.name.local == name)
            .map(|attr| into_tendril(attr.value.clone()))
    }

    /// Sets the specified attribute's value.
    pub fn set_attr(&mut self, name: &str, val: &str) {
        let attr = self.attrs.iter_mut().find(|a| &a.name.local == name);
        match attr {
            Some(attr) => attr.value = wrap_tendril(StrTendril::from(val)),
            None => {
                let value = StrTendril::from(val);
                // The namespace on the attribute name is almost always ns!().
                let name = QualName::new(None, ns!(), LocalName::from(name));
                self.attrs.push(Attr {
                    name,
                    value: wrap_tendril(value),
                })
            }
        }
    }

    /// Removes the specified attribute from the element.
    pub fn remove_attr(&mut self, name: &str) {
        self.attrs.retain(|attr| &attr.name.local != name);
    }

    /// Removes the specified attributes from the element.
    ///
    /// # Arguments
    /// - `names`: A slice of attribute names to remove. Empty slice removes no attributes.
    pub fn remove_attrs(&mut self, names: &[&str]) {
        self.attrs
            .retain(|attr| !names.contains(&attr.name.local.as_ref()));
    }

    /// Retains only the attributes with the specified names.
    ///
    /// # Arguments
    /// - `names`: A slice of attribute names to retain. Empty slice retains no attributes.
    pub fn retain_attrs(&mut self, names: &[&str]) {
        self.attrs
            .retain(|a| names.contains(&a.name.local.as_ref()));
    }

    /// Removes all attributes from the element.
    pub fn remove_all_attrs(&mut self) {
        self.attrs.clear();
    }

    /// Checks if the element has an attribute with the name.
    pub fn has_attr(&self, name: &str) -> bool {
        self.attrs.iter().any(|attr| &attr.name.local == name)
    }

    /// Add attributes if they are not already present.
    pub(crate) fn add_attrs_if_missing(&mut self, attrs: Vec<Attribute>) {
        let attrs = wrap_attrs(attrs);
        let existing_names = self
            .attrs
            .iter()
            .map(|e| e.name.clone())
            .collect::<Vec<_>>();

        self.attrs.extend(
            attrs
                .into_iter()
                .filter(|attr| !existing_names.contains(&attr.name)),
        );
    }

    /// Renames the element.
    pub fn rename(&mut self, name: &str) {
        let new_name = QualName::new(None, ns!(), LocalName::from(name));
        self.name = new_name;
    }

    /// If element is a link.
    pub fn is_link(&self) -> bool {
        matches!(
            self.name.local,
            local_name!("a") | local_name!("area") | local_name!("link")
        ) && self
            .attrs
            .iter()
            .any(|a| a.name.local == local_name!("href"))
    }
}
