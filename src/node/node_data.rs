use std::fmt::Debug;
use std::ops::Deref;

use html5ever::LocalName;
use html5ever::{local_name, namespace_url, ns, Attribute, QualName};
use selectors::attr::CaseSensitivity;
use tendril::StrTendril;

use crate::entities::HashSetFx;

use super::NodeId;

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
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },

    /// A text node.
    Text { contents: StrTendril },

    /// A comment.
    Comment { contents: StrTendril },

    /// An element with attributes.
    Element(Element),

    /// A Processing instruction.
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

/// An element with attributes.
#[derive(Debug, Clone)]
pub struct Element {
    pub name: QualName,
    pub attrs: Vec<Attribute>,

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
            attrs,
            template_contents,
            mathml_annotation_xml_integration_point,
        }
    }

    /// The name of the node.
    pub fn node_name(&self) -> StrTendril {
        StrTendril::from(self.name.local.as_ref())
    }

    /// Whether the element has the given class.
    pub fn has_class(&self, class: &str) -> bool {
        self.attrs
            .iter()
            .find(|attr| &attr.name.local == "class")
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

        let class_set: HashSetFx<&str> = classes
            .split(' ')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let attr = self
            .attrs
            .iter_mut()
            .find(|attr| &attr.name.local == "class");

        match attr {
            Some(attr) => {
                let value: &mut StrTendril = &mut attr.value;
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
                self.attrs.push(Attribute { name, value });
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
            .find(|attr| &attr.name.local == "class")
        {
            let mut set: HashSetFx<&str> = attr
                .value
                .split(' ')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let removes = class.split(' ').map(|s| s.trim()).filter(|s| !s.is_empty());

            for remove in removes {
                set.remove(remove);
            }

            attr.value = StrTendril::from(set.into_iter().collect::<Vec<&str>>().join(" "));
        }
    }

    /// Gets the specified attribute's value.
    pub fn attr(&self, name: &str) -> Option<StrTendril> {
        self.attrs
            .iter()
            .find(|attr| &attr.name.local == name)
            .map(|attr| attr.value.clone())
    }

    /// Sets the specified attribute's value.
    pub fn set_attr(&mut self, name: &str, val: &str) {
        let attr = self.attrs.iter_mut().find(|a| &a.name.local == name);
        match attr {
            Some(attr) => attr.value = StrTendril::from(val),
            None => {
                let value = StrTendril::from(val);
                // The namespace on the attribute name is almost always ns!().
                let name = QualName::new(None, ns!(), LocalName::from(name));
                self.attrs.push(Attribute { name, value })
            }
        }
    }

    /// Removes the specified attribute from the element.
    pub fn remove_attr(&mut self, name: &str) {
        self.attrs.retain(|attr| &attr.name.local != name);
    }

    /// Removes the specified attributes from the element.
    pub fn remove_attrs(&mut self, names: &[&str]) {
        self.attrs.retain(|attr| {
            let name_local: &str = &attr.name.local;
            !names.contains(&name_local)
        });
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
        let existing_names = self
            .attrs
            .iter()
            .map(|e| e.name.clone())
            .collect::<HashSetFx<_>>();

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
}
