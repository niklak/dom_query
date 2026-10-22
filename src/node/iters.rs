use std::cell::Ref;

use super::NodeId;
use super::inner::TreeNode;

/// An iterator over the children of a node.
pub struct ChildNodes<'a> {
    nodes: Ref<'a, Vec<TreeNode>>,
    next_child_id: Option<NodeId>,
    rev: bool,
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
    pub fn new(nodes: Ref<'a, Vec<TreeNode>>, node_id: &NodeId, rev: bool) -> Self {
        let first_child = nodes.get(node_id.value()).and_then(|node| {
            if rev {
                node.last_child
            } else {
                node.first_child
            }
        });

        ChildNodes {
            nodes,
            next_child_id: first_child,
            rev,
        }
    }
}

impl Iterator for ChildNodes<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current_id = self.next_child_id?;
        let current_node = self.nodes.get(current_id.value())?;

        if self.rev {
            self.next_child_id = current_node.prev_sibling;
        } else {
            self.next_child_id = current_node.next_sibling;
        }
        Some(current_id)
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
pub fn child_nodes<'a>(nodes: Ref<'a, Vec<TreeNode>>, id: &NodeId, rev: bool) -> ChildNodes<'a> {
    ChildNodes::new(nodes, id, rev)
}

/// An iterator over the ancestors of a node.
pub struct AncestorNodes<'a> {
    nodes: Ref<'a, Vec<TreeNode>>,
    next_parent_id: Option<NodeId>,
    // u32::MAX means unlimited; a tree cannot have more than u32::MAX nodes.
    remaining: u32,
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
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(nodes: Ref<'a, Vec<TreeNode>>, node_id: &NodeId, max_depth: Option<usize>) -> Self {
        let next_parent_id = nodes.get(node_id.value()).and_then(|node| node.parent);
        let remaining = match max_depth {
            Some(depth) if depth > 0 => depth.min((u32::MAX - 1) as usize) as u32,
            _ => u32::MAX,
        };

        AncestorNodes {
            nodes,
            next_parent_id,
            remaining,
        }
    }
}

impl Iterator for AncestorNodes<'_> {
    type Item = NodeId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let current_id = self.next_parent_id?;
        let current_node = self.nodes.get(current_id.value())?;
        self.next_parent_id = current_node.parent;
        Some(current_id)
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

/// An iterator over the descendants of a node.
pub struct DescendantNodes<'a> {
    nodes: Ref<'a, Vec<TreeNode>>,
    next_child_id: Option<NodeId>,
    start_id: NodeId,
}

impl<'a> DescendantNodes<'a> {
    /// Creates a new `DescendantNodes` iterator.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `node_id` - The id of the starting node.
    ///
    /// # Returns
    ///
    /// `DescendantNodes<'a, T>`
    pub fn new(nodes: Ref<'a, Vec<TreeNode>>, node_id: &NodeId) -> Self {
        let next_child_id = nodes.get(node_id.value()).and_then(|node| node.first_child);

        DescendantNodes {
            nodes,
            next_child_id,
            start_id: *node_id,
        }
    }

    fn get_child_or_sibling(&self, node_id: NodeId) -> Option<NodeId> {
        let node = self.nodes.get(node_id.value())?;
        if node.first_child.is_some() {
            node.first_child
        } else if node.next_sibling.is_some() {
            node.next_sibling
        } else {
            let mut parent = node.parent;
            while let Some(parent_node) = parent.and_then(|id| self.nodes.get(id.value())) {
                if parent_node.id == self.start_id {
                    return None;
                }
                if parent_node.next_sibling.is_some() {
                    return parent_node.next_sibling;
                }
                parent = parent_node.parent;
            }

            None
        }
    }

    pub(crate) fn node(&self, id: NodeId) -> Option<&TreeNode> {
        self.nodes.get(id.value())
    }
}

impl Iterator for DescendantNodes<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current_id = self.next_child_id?;
        self.next_child_id = self.get_child_or_sibling(current_id);
        Some(current_id)
    }
}

/// Returns an iterator over the descendants of a node
///
/// # Arguments
///
/// * `nodes` - The nodes of the tree.
/// * `node_id` - The id of the starting node.
///
/// # Returns
///
/// `DescendantNodes<'a, T>`
pub fn descendant_nodes<'a>(nodes: Ref<'a, Vec<TreeNode>>, id: &NodeId) -> DescendantNodes<'a> {
    DescendantNodes::new(nodes, id)
}
