use std::cell::Ref;

use super::inner::TreeNode;
use super::NodeId;

/// An iterator over the children of a node.
pub struct ChildNodes<'a> {
    nodes: Ref<'a, Vec<TreeNode>>,
    next_child_id: Option<NodeId>,
}

impl<'a> ChildNodes<'a> {
    /// Creates a new `ChildNodes` iterator.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `node_id` - The id of the parent node.
    ///
    /// # Returns
    ///
    /// `ChildNodes<'a>`
    pub fn new(nodes: Ref<'a, Vec<TreeNode>>, node_id: &NodeId) -> Self {
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

impl<'a> Iterator for ChildNodes<'a> {
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
pub fn child_nodes<'a>(nodes: Ref<'a, Vec<TreeNode>>, id: &NodeId) -> ChildNodes<'a> {
    ChildNodes::new(nodes, id)
}

/// An iterator over the ancestors of a node.
pub struct AncestorNodes<'a> {
    nodes: Ref<'a, Vec<TreeNode>>,
    next_parent_id: Option<NodeId>,
    max_depth: Option<usize>,
    current_depth: usize,
}

impl<'a> AncestorNodes<'a> {
    /// Creates a new `AncestorsIter` iterator.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `node_id` - The id of the starting node.
    /// * `max_depth` - Maximum depth to traverse up the tree. None or Some(0) means no limit.
    ///
    /// # Returns
    ///
    /// `AncestorsIter<'a, T>`
    pub fn new(nodes: Ref<'a, Vec<TreeNode>>, node_id: &NodeId, max_depth: Option<usize>) -> Self {
        let next_parent_id = nodes.get(node_id.value).and_then(|node| node.parent);

        AncestorNodes {
            nodes,
            next_parent_id,
            max_depth,
            current_depth: 0,
        }
    }
}

impl<'a> Iterator for AncestorNodes<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if max_depth > 0 && self.current_depth >= max_depth {
                return None;
            }
        }

        let current_id = self.next_parent_id?;

        if let Some(node) = self.nodes.get(current_id.value) {
            self.next_parent_id = node.parent;
            self.current_depth += 1;
            Some(current_id)
        } else {
            None
        }
    }
}

/// Returns an iterator over the ancestors of a node
///
/// # Arguments
///
/// * `nodes` - The nodes of the tree.
/// * `node_id` - The id of the starting node.
/// * `max_depth` - Maximum depth to traverse up the tree. None or Some(0) means no limit.
///
/// # Returns
///
/// `AncestorsIter<'a, T>`
pub fn ancestor_nodes<'a>(
    nodes: Ref<'a, Vec<TreeNode>>,
    id: &NodeId,
    max_depth: Option<usize>,
) -> AncestorNodes<'a> {
    AncestorNodes::new(nodes, id, max_depth)
}
