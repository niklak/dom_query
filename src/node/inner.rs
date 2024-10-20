use std::fmt::{self, Debug};

use super::node_data::{Element, NodeData};
use crate::NodeId;

/// The inner node is a [`crate::Tree`] node.
pub struct InnerNode<T> {
    pub id: Option<NodeId>,
    pub parent: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub data: T,
}

impl<T> InnerNode<T> {
    /// Creates a new inner node.
    pub(crate) fn new(id: NodeId, data: T) -> Self {
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

impl<T: Debug> Debug for InnerNode<T> {
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

impl InnerNode<NodeData> {
    pub fn is_document(&self) -> bool {
        matches!(self.data, NodeData::Document)
    }

    pub fn is_element(&self) -> bool {
        matches!(self.data, NodeData::Element(_))
    }

    pub fn is_text(&self) -> bool {
        matches!(self.data, NodeData::Text { .. })
    }

    pub fn is_comment(&self) -> bool {
        matches!(self.data, NodeData::Comment { .. })
    }

    pub fn is_fragment(&self) -> bool {
        matches!(self.data, NodeData::Fragment)
    }

    pub fn is_doctype(&self) -> bool {
        matches!(self.data, NodeData::Doctype { .. })
    }

    pub fn as_element(&self) -> Option<&Element> {
        match self.data {
            NodeData::Element(ref e) => Some(e),
            _ => None,
        }
    }

    pub fn as_element_mut(&mut self) -> Option<&mut Element> {
        match self.data {
            NodeData::Element(ref mut e) => Some(e),
            _ => None,
        }
    }
}

impl<T: Clone> Clone for InnerNode<T> {
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
