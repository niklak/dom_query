use std::cell::Ref;

use super::inner::TreeNode;
use super::NodeId;

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
        let first_child = nodes
            .get(node_id.value)
            .map(|node| {
                if rev {
                    node.last_child
                } else {
                    node.first_child
                }
            })
            .unwrap_or(None);

        ChildNodes {
            nodes,
            next_child_id: first_child,
            rev,
        }
    }
}

impl<'a> Iterator for ChildNodes<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current_id = self.next_child_id?;

        self.nodes.get(current_id.value).map(|node| {
            self.next_child_id = if self.rev {
                node.prev_sibling
            } else {
                node.next_sibling
            };
            current_id
        })
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

        self.nodes.get(current_id.value).map(|node| {
            self.next_parent_id = node.parent;
            self.current_depth += 1;
            current_id
        })
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
        let next_child_id = nodes.get(node_id.value).and_then(|node| node.first_child);

        DescendantNodes {
            nodes,
            next_child_id,  
        }
    }

    fn get_child_or_sibling(&self, node: &TreeNode) -> Option<NodeId> {
        if node.first_child.is_some() {
            node.first_child
        } else if node.next_sibling.is_some() {
            node.next_sibling
        } else {
            let mut parent = node.parent;
            while let Some(parent_node) = parent.and_then(|id| self.nodes.get(id.value)) {
                if parent_node.next_sibling.is_some() {
                    return parent_node.next_sibling;
                } else {
                    parent = parent_node.parent
                }
            }
    
            None
        }
    }
}

impl<'a> Iterator for DescendantNodes<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        
        let current_id = self.next_child_id?;

        if let Some(node) = self.nodes.get(current_id.value) {
            self.next_child_id = self.get_child_or_sibling(node);
            Some(current_id)
        } else {
            None
        }
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
pub fn descendant_nodes<'a>(
    nodes: Ref<'a, Vec<TreeNode>>,
    id: &NodeId,
) -> DescendantNodes<'a> {
    DescendantNodes::new(nodes, id)
}
