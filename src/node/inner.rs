use std::fmt::{self, Debug};

use super::node_data::{Element, NodeData};
use crate::NodeId;

/// The inner node is a [`crate::Tree`] node.
pub struct InnerNode {
    pub id: Option<NodeId>,
    pub parent: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub data: NodeData,
}

impl InnerNode {
    /// Creates a new inner node.
    pub(crate) fn new(id: NodeId, data: NodeData) -> Self {
        InnerNode {
            id: Some(id),
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            data,
        }
    }
}

impl Debug for InnerNode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Node")
            .field("id", &self.id)
            .field("parent", &self.parent)
            .field("prev_sibling", &self.prev_sibling)
            .field("next_sibling", &self.next_sibling)
            .field("first_child", &self.first_child)
            .field("last_child", &self.last_child)
            .field("data", &self.data)
            .finish()
    }
}

impl InnerNode {
    /// Checks if the node is a document node.
    pub fn is_document(&self) -> bool {
        matches!(self.data, NodeData::Document)
    }

    /// Checks if the node is an element node.
    pub fn is_element(&self) -> bool {
        matches!(self.data, NodeData::Element(_))
    }

    /// Checks if the node is a text node.
    pub fn is_text(&self) -> bool {
        matches!(self.data, NodeData::Text { .. })
    }

    /// Checks if the node is a comment node.
    pub fn is_comment(&self) -> bool {
        matches!(self.data, NodeData::Comment { .. })
    }

    /// Checks if the node is a fragment node.
    pub fn is_fragment(&self) -> bool {
        matches!(self.data, NodeData::Fragment)
    }

    /// Checks if the node is a doctype node.
    pub fn is_doctype(&self) -> bool {
        matches!(self.data, NodeData::Doctype { .. })
    }
    /// Returns a reference to the node as an element. If the node is not an element, `None` is returned.
    ///
    /// # Returns
    /// `Option<&Element>`
    pub fn as_element(&self) -> Option<&Element> {
        match self.data {
            NodeData::Element(ref e) => Some(e),
            _ => None,
        }
    }

    /// Returns a mutable reference to the node as an element. If the node is not an element, `None` is returned.
    pub fn as_element_mut(&mut self) -> Option<&mut Element> {
        match self.data {
            NodeData::Element(ref mut e) => Some(e),
            _ => None,
        }
    }
}

impl Clone for InnerNode {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            parent: self.parent,
            prev_sibling: self.prev_sibling,
            next_sibling: self.next_sibling,
            first_child: self.first_child,
            last_child: self.last_child,
            data: self.data.clone(),
        }
    }
}
