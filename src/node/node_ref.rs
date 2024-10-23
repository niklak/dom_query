use std::fmt::Debug;

use html5ever::serialize;
use html5ever::serialize::SerializeOpts;
use html5ever::serialize::TraversalScope;

use html5ever::Attribute;

use tendril::StrTendril;

use crate::Document;
use crate::Tree;

use super::children_of;
use super::inner::InnerNode;
use super::node_data::NodeData;
use super::serializing::SerializableNodeRef;
use super::NodeId;

/// Alias for `NodeRef`.
pub type Node<'a> = NodeRef<'a, NodeData>;

#[derive(Clone, Debug)]
pub struct NodeRef<'a, T> {
    pub id: NodeId,
    pub tree: &'a Tree<T>,
}

impl<'a, T: Debug> NodeRef<'a, T> {
    /// Creates a new node reference.
    pub fn new(id: NodeId, tree: &'a Tree<T>) -> Self {
        Self { id, tree }
    }

    /// Selects the node from the tree and applies a function to it.
    #[inline]
    pub fn query<F, B>(&self, f: F) -> Option<B>
    where
        F: FnOnce(&InnerNode<T>) -> B,
    {
        self.tree.query_node(&self.id, f)
    }

    /// Selects the node from the tree and applies a function to it.
    /// Accepts a default value to return for a case if the node doesn't exist.
    #[inline]
    pub fn query_or<F, B>(&self, default: B, f: F) -> B
    where
        F: FnOnce(&InnerNode<T>) -> B,
    {
        self.tree.query_node_or(&self.id, default, f)
    }

    /// Selects the node from the tree and applies a function to it.
    #[inline]
    pub fn update<F, B>(&self, f: F) -> Option<B>
    where
        F: FnOnce(&mut InnerNode<T>) -> B,
    {
        self.tree.update_node(&self.id, f)
    }

    /// Returns the parent node of the selected node.
    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.tree.parent_of(&self.id)
    }

    /// Returns the children nodes of the selected node.
    #[inline]
    pub fn children(&self) -> Vec<Self> {
        self.tree.children_of(&self.id)
    }

    /// Returns ancestor nodes of the selected node.
    ///
    /// # Arguments
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    #[inline]
    pub fn ancestors(&self, max_depth: Option<usize>) -> Vec<Self> {
        self.tree.ancestors_of(&self.id, max_depth)
    }

    /// Returns the first child node of the selected node.
    #[inline]
    pub fn first_child(&self) -> Option<Self> {
        self.tree.first_child_of(&self.id)
    }

    /// Returns the last child node of the selected node.
    #[inline]
    pub fn last_child(&self) -> Option<Self> {
        self.tree.last_child_of(&self.id)
    }

    /// Returns the next sibling node of the selected node.
    #[inline]
    pub fn next_sibling(&self) -> Option<Self> {
        self.tree.next_sibling_of(&self.id)
    }

    /// Removes the selected node from its parent node, but keeps it in the tree.
    #[inline]
    pub fn remove_from_parent(&self) {
        self.tree.remove_from_parent(&self.id)
    }

    /// Removes all children nodes of the selected node.
    #[inline]
    pub fn remove_children(&self) {
        self.tree.remove_children_of(&self.id)
    }

    /// Appends another node by id to the parent node of the selected node.
    /// Another node takes place of the selected node.
    #[inline]
    pub fn append_prev_sibling(&self, id: &NodeId) {
        self.tree.append_prev_sibling_of(&self.id, id)
    }

    /// Appends another node by id to the selected node.
    #[inline]
    pub fn append_child(&self, id: &NodeId) {
        self.tree.append_child_of(&self.id, id)
    }

    /// Appends another tree to the selected node from another tree.
    #[inline]
    pub fn append_children_from_another_tree(&self, tree: Tree<T>) {
        self.tree.append_children_from_another_tree(&self.id, tree)
    }

    #[inline]
    pub fn append_prev_siblings_from_another_tree(&self, tree: Tree<T>) {
        self.tree
            .append_prev_siblings_from_another_tree(&self.id, tree)
    }
}

impl<'a> Node<'a> {
    /// Parses given fragment html and appends its contents to the selected node.
    pub fn append_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        let fragment = Document::fragment(html);
        self.append_children_from_another_tree(fragment.tree);
    }

    /// Parses given fragment html and sets its contents to the selected node.
    pub fn set_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        self.remove_children();
        self.append_html(html);
    }
}

impl<'a> Node<'a> {
    /// Returns the next sibling, that is an [`crate::node::node_data::Element`] of the selected node.
    pub fn next_element_sibling(&self) -> Option<Node<'a>> {
        let nodes = self.tree.nodes.borrow();
        let mut node = nodes.get(self.id.value)?;

        let r = loop {
            if let Some(id) = node.next_sibling {
                node = nodes.get(id.value)?;
                if node.is_element() {
                    break Some(NodeRef::new(id, self.tree));
                }
            } else {
                break None;
            }
        };
        r
    }

    /// Returns the previous sibling, that is an [`crate::node::node_data::Element`] of the selected node.
    pub fn prev_element_sibling(&self) -> Option<Node<'a>> {
        let nodes = self.tree.nodes.borrow();
        let mut node = nodes.get(self.id.value)?;

        let r = loop {
            if let Some(id) = node.prev_sibling {
                node = nodes.get(id.value)?;
                if node.is_element() {
                    break Some(NodeRef::new(id, self.tree));
                }
            } else {
                break None;
            }
        };
        r
    }

    /// Returns the first child, that is an [`crate::node::node_data::Element`] of the selected node.
    pub fn first_element_child(&self) -> Option<Node<'a>> {
        let nodes = self.tree.nodes.borrow();
        if let Some(node) = nodes.get(self.id.value) {
            let mut next_child_id = node.first_child;

            while let Some(node_id) = next_child_id {
                if node.is_element() {
                    return Some(NodeRef {
                        id: node_id,
                        tree: self.tree,
                    });
                }
                next_child_id = node.next_sibling;
            }
        }
        None
    }
}

impl<'a> Node<'a> {
    /// Returns the name of the selected node if it is an [`crate::node::node_data::Element`] otherwise `None`.
    pub fn node_name(&self) -> Option<StrTendril> {
        let nodes = self.tree.nodes.borrow();
        nodes
            .get(self.id.value)
            .and_then(|node| node.as_element().map(|e| e.node_name()))
    }

    /// Checks if node has a specified class
    pub fn has_class(&self, class: &str) -> bool {
        self.query_or(false, |node| {
            node.as_element().map_or(false, |e| e.has_class(class))
        })
    }

    /// Adds a class to the node
    pub fn add_class(&self, class: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.add_class(class);
            }
        });
    }

    /// Removes a class from the node
    pub fn remove_class(&self, class: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_class(class);
            }
        });
    }

    /// Returns the value of the specified attribute
    pub fn attr(&self, name: &str) -> Option<StrTendril> {
        self.query_or(None, |node| node.as_element().and_then(|e| e.attr(name)))
    }

    /// Returns all attributes
    pub fn attrs(&self) -> Vec<Attribute> {
        self.query_or(vec![], |node| {
            node.as_element().map_or(vec![], |e| e.attrs.to_vec())
        })
    }

    /// Sets the value of the specified attribute to the node.
    pub fn set_attr(&self, name: &str, val: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.set_attr(name, val);
            }
        });
    }

    /// Removes the specified attribute from the element.
    pub fn remove_attr(&self, name: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_attr(name);
            }
        });
    }

    /// Removes the specified attributes from the element.
    pub fn remove_attrs(&self, names: &[&str]) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_attrs(names);
            }
        });
    }

    /// Removes all attributes from the element.
    pub fn remove_all_attrs(&self) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_all_attrs();
            }
        });
    }

    /// Checks if node has a specified attribute
    pub fn has_attr(&self, name: &str) -> bool {
        self.query_or(false, |node| {
            node.as_element().map_or(false, |e| e.has_attr(name))
        })
    }

    /// Renames the node if node is an [`crate::node::node_data::Element`].
    pub fn rename(&self, name: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.rename(name);
            }
        });
    }
}

impl<'a> Node<'a> {
    /// Returns true if this node is a document.
    pub fn is_document(&self) -> bool {
        self.query_or(false, |node| node.is_document())
    }

    /// Returns true if this node is a fragment.
    pub fn is_fragment(&self) -> bool {
        self.query_or(false, |node| node.is_fragment())
    }

    /// Returns true if this node is an element.
    pub fn is_element(&self) -> bool {
        self.query_or(false, |node| node.is_element())
    }

    /// Returns true if this node is a text node.
    pub fn is_text(&self) -> bool {
        self.query_or(false, |node| node.is_text())
    }
    /// Returns true if this node is a comment.
    pub fn is_comment(&self) -> bool {
        self.query_or(false, |node| node.is_comment())
    }
    /// Returns true if this node is a DOCTYPE.
    pub fn is_doctype(&self) -> bool {
        self.query_or(false, |node| node.is_doctype())
    }
}

impl<'a> Node<'a> {
    /// Returns the HTML representation of the DOM tree.
    /// Panics if serialization fails.
    pub fn html(&self) -> StrTendril {
        self.serialize_html(TraversalScope::IncludeNode).unwrap()
    }

    /// Returns the HTML representation of the DOM tree without the outermost node.
    /// Panics if serialization fails.
    pub fn inner_html(&self) -> StrTendril {
        self.serialize_html(TraversalScope::ChildrenOnly(None))
            .unwrap()
    }

    // Returns the HTML representation of the DOM tree, if it succeeds or `None`.
    pub fn try_html(&self) -> Option<StrTendril> {
        self.serialize_html(TraversalScope::IncludeNode)
    }

    // Returns the HTML representation of the DOM tree without the outermost node, if it succeeds or `None`.
    pub fn try_inner_html(&self) -> Option<StrTendril> {
        self.serialize_html(TraversalScope::ChildrenOnly(None))
    }

    fn serialize_html(&self, traversal_scope: TraversalScope) -> Option<StrTendril> {
        let inner: SerializableNodeRef = self.clone().into();
        let mut result = vec![];
        serialize(
            &mut result,
            &inner,
            SerializeOpts {
                scripting_enabled: false,
                create_missing_parent: false,
                traversal_scope,
            },
        )
        .ok()?;
        StrTendril::try_from_byte_slice(&result).ok()
    }

    /// Returns the text of the node and its descendants.
    pub fn text(&self) -> StrTendril {
        let mut ops = vec![self.id];
        let mut text = StrTendril::new();
        let nodes = self.tree.nodes.borrow();
        while !ops.is_empty() {
            let id = ops.remove(0);
            if let Some(node) = nodes.get(id.value) {
                match node.data {
                    NodeData::Element(_) => {
                        for child in children_of(&nodes, &id).into_iter().rev() {
                            ops.insert(0, child);
                        }
                    }

                    NodeData::Text { ref contents } => text.push_tendril(contents),

                    _ => continue,
                }
            }
        }
        text
    }

    /// Checks if the node contains the specified text
    pub fn has_text(&self, needle: &str) -> bool {
        let mut ops = vec![self.id];
        let nodes = self.tree.nodes.borrow();
        while !ops.is_empty() {
            let id = ops.remove(0);
            if let Some(node) = nodes.get(id.value) {
                match node.data {
                    NodeData::Element(_) => {
                        for child in children_of(&nodes, &id).into_iter().rev() {
                            ops.insert(0, child);
                        }
                    }

                    NodeData::Text { ref contents } => {
                        if contents.contains(needle) {
                            return true;
                        }
                    }

                    _ => continue,
                }
            }
        }
        false
    }
}
