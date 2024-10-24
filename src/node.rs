mod inner;
mod node_data;
mod node_ref;
mod selector;
mod serializing;
mod iters;

use std::cell::Ref;
use std::fmt::Debug;

pub use inner::InnerNode;
pub use node_data::{Element, NodeData};
pub use node_ref::{Node, NodeRef};
pub use serializing::SerializableNodeRef;
pub use iters::ChildrenIterator;

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
    ChildrenIterator::new(nodes, id).collect()
}

pub(crate) fn ancestors_of<T>(
    nodes: &Ref<Vec<InnerNode<T>>>,
    id: &NodeId,
    max_depth: Option<usize>,
) -> Vec<NodeId> {
    let max_depth = max_depth.unwrap_or(0);
    let mut depth: usize = 0;

    let mut ancestors = vec![];

    if let Some(node) = nodes.get(id.value) {
        let mut parent = node.parent;
        while let Some(parent_id) = parent {
            if max_depth > 0 && depth == max_depth {
                break;
            }

            ancestors.push(parent_id);
            depth += 1;
            if let Some(node) = nodes.get(parent_id.value) {
                parent = node.parent;
            }
        }
    }
    ancestors
}
