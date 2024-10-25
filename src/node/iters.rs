use std::cell::Ref;

use super::inner::InnerNode;
use super::NodeId;


/// An iterator over the children of a node.
pub struct ChildNodes<'a, T> {
    nodes: Ref<'a, Vec<InnerNode<T>>>,
    next_child_id: Option<NodeId>,
}

impl<'a, T> ChildNodes<'a, T> {
    /// Creates a new `ChildNodes` iterator.
    /// 
    /// # Arguments
    /// 
    /// * `nodes` - The nodes of the tree.
    /// * `node_id` - The id of the parent node.
    /// 
    /// # Returns
    /// 
    /// `ChildNodes<'a, T>`
    pub fn new(nodes: Ref<'a, Vec<InnerNode<T>>>, node_id: &NodeId) -> Self {
        let first_child = nodes
            .get(node_id.value)
            .map(|node| node.first_child)
            .unwrap_or(None);

        ChildNodes {
            nodes,
            next_child_id: first_child,
        }
    }
}

impl<'a, T> Iterator for ChildNodes<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current_id = self.next_child_id?;

        if let Some(node) = self.nodes.get(current_id.value) {
            self.next_child_id = node.next_sibling;
            Some(current_id)
        } else {
            None
        }
    }
}

/// Returns an iterator over the children of a node
/// 
/// # Arguments
/// 
/// * `nodes` - The nodes of the tree.
/// * `node_id` - The id of the parent node.
/// 
/// # Returns
/// 
/// `ChildNodes<'a, T>`
pub fn child_nodes<'a, T>(nodes: Ref<'a, Vec<InnerNode<T>>>, id: &NodeId) -> ChildNodes<'a, T> {
    ChildNodes::new(nodes, id)
}
