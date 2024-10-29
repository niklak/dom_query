mod inner;
mod iters;
mod node_data;
mod node_ref;
mod selector;
mod serializing;

use std::fmt::Debug;

pub use inner::TreeNode;
pub use iters::{ancestor_nodes, child_nodes, AncestorNodes, ChildNodes};
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
