mod inner;
mod node_data;
mod node_ref;
mod selector;
mod serializing;

use std::cell::Ref;
use std::fmt::Debug;

pub use inner::InnerNode;
pub use node_data::{Element, NodeData};
pub use node_ref::{Node, NodeRef};
pub use serializing::SerializableNodeRef;

/// Represents a Node ID.
#[derive(Copy, Debug, Clone, Eq, PartialEq, Hash)]
pub struct NodeId {
    pub(crate) value: usize,
}

impl NodeId {
    pub(crate) fn new(value: usize) -> Self {
        NodeId { value }
    }
}

pub(crate) fn children_of<T>(nodes: &Ref<Vec<InnerNode<T>>>, id: &NodeId) -> Vec<NodeId> {
    let mut children = vec![];

    if let Some(node) = nodes.get(id.value) {
        let mut next_child_id = node.first_child;

        while let Some(node_id) = next_child_id {
            if let Some(node) = nodes.get(node_id.value) {
                next_child_id = node.next_sibling;
                children.push(node_id);
            }
        }
    }
    children
}
