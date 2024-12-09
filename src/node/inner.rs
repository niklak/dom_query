use std::fmt::Debug;

use super::node_data::{Element, NodeData};
use crate::NodeId;

/// The inner node is a [`crate::Tree`] node.
#[derive(Debug)]
pub struct TreeNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub data: NodeData,
}

impl TreeNode {
    /// Creates a new inner node.
    pub(crate) fn new(id: NodeId, data: NodeData) -> Self {
        TreeNode {
            id,
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            data,
        }
    }
}

impl TreeNode {
    /// fixes node ids
    pub(crate) fn adjust(&mut self, offset: usize) {
        self.id = NodeId::new(self.id.value + offset);
        self.parent = self.parent.map(|id| NodeId::new(id.value + offset));
        self.prev_sibling = self.prev_sibling.map(|id| NodeId::new(id.value + offset));
        self.next_sibling = self.next_sibling.map(|id| NodeId::new(id.value + offset));
        self.first_child = self.first_child.map(|id| NodeId::new(id.value + offset));
        self.last_child = self.last_child.map(|id| NodeId::new(id.value + offset));
    }
}

impl TreeNode {
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

    /// Checks if node may have children nodes.
    pub fn may_have_children(&self) -> bool {
        matches!(
            self.data,
            NodeData::Document | NodeData::Fragment | NodeData::Element(_)
        )
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

impl Clone for TreeNode {
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
