use std::cell::Ref;

use tendril::StrTendril;

use crate::node::child_nodes;
use crate::node::{NodeData, NodeId, TreeNode};
pub struct TreeNodeOps {}

// property
impl TreeNodeOps {
    /// Collects all text content of a node and its descendants.
    ///
    /// - `nodes`: a reference to a vector of `TreeNode`s.
    /// - `id`: `NodeId` of the element to get the text content from.
    ///
    /// This function will traverse the tree and collect all text content
    /// from the node and its descendants. It will ignore any nodes that
    /// are not `Element`s or `Text`s.
    ///
    /// The function returns a `StrTendril` containing all collected text content.
    pub fn text_of(nodes: Ref<Vec<TreeNode>>, id: NodeId) -> StrTendril {
        let mut ops = vec![id];
        let mut text = StrTendril::new();

        while let Some(id) = ops.pop() {
            if let Some(node) = nodes.get(id.value) {
                match node.data {
                    NodeData::Document | NodeData::Fragment | NodeData::Element(_) => {
                        ops.extend(child_nodes(Ref::clone(&nodes), &id, true));
                    }
                    NodeData::Text { ref contents } => text.push_tendril(contents),

                    _ => continue,
                }
            }
        }
        text
    }

    /// Gets the last sibling node of a node by id.
    ///
    /// This function walks through sibling nodes from the given node until there are no more sibling nodes.
    /// It returns the last sibling node it found.
    pub fn last_sibling_of(nodes: &[TreeNode], id: &NodeId) -> Option<NodeId> {
        let node = nodes.get(id.value)?;

        let mut next_node = node.next_sibling.and_then(|id| nodes.get(id.value));
        let mut last_node = None;
        while let Some(node) = next_node {
            let n = node.next_sibling.and_then(|id| nodes.get(id.value));
            last_node = Some(node.id);
            next_node = n;
        }
        last_node
    }
}

// manipulation
impl TreeNodeOps {
    /// Creates a new element from data  and appends it to a node by id
    pub fn append_child_data_of(nodes: &mut Vec<TreeNode>, id: &NodeId, data: NodeData) {
        let last_child_id = nodes.get(id.value).and_then(|node| node.last_child);

        let new_child_id = NodeId::new(nodes.len());
        let mut child = TreeNode::new(new_child_id, data);
        let new_child_id_opt = Some(new_child_id);
        child.prev_sibling = last_child_id;
        child.parent = Some(*id);
        nodes.push(child);

        if let Some(id) = last_child_id {
            if let Some(node) = nodes.get_mut(id.value) {
                node.next_sibling = new_child_id_opt
            };
        }

        if let Some(parent) = nodes.get_mut(id.value) {
            if parent.first_child.is_none() {
                parent.first_child = new_child_id_opt
            }
            parent.last_child = new_child_id_opt;
        }
    }

    /// Appends a child node by `new_child_id` to a node by `id`. `new_child_id` must exist in the tree.
    pub fn append_child_of(nodes: &mut [TreeNode], id: &NodeId, new_child_id: &NodeId) {
        let Some(parent) = nodes.get_mut(id.value) else {
            // TODO: panic or not?
            return;
        };

        let last_child_id = parent.last_child;

        if last_child_id.is_none() {
            parent.first_child = Some(*new_child_id);
        }

        parent.last_child = Some(*new_child_id);

        if let Some(id) = last_child_id {
            if let Some(last_child) = nodes.get_mut(id.value) {
                last_child.next_sibling = Some(*new_child_id);
            }
        }

        {
            if let Some(child) = nodes.get_mut(new_child_id.value) {
                child.prev_sibling = last_child_id;
                child.parent = Some(*id);
            }
        }
    }

    /// Prepend a child node by `new_child_id` to a node by `id`. `new_child_id` must exist in the tree.
    pub fn prepend_child_of(nodes: &mut [TreeNode], id: &NodeId, new_child_id: &NodeId) {
        let Some(parent) = nodes.get_mut(id.value) else {
            // TODO: panic or not?
            return;
        };
        let first_child_id = parent.first_child;

        if first_child_id.is_none() {
            parent.last_child = Some(*new_child_id);
        }

        parent.first_child = Some(*new_child_id);

        if let Some(id) = first_child_id {
            if let Some(first_child) = nodes.get_mut(id.value) {
                first_child.prev_sibling = Some(*new_child_id);
            }
        }

        {
            if let Some(child) = nodes.get_mut(new_child_id.value) {
                child.next_sibling = first_child_id;
                child.parent = Some(*id);
                child.prev_sibling = None;
            }
        }
    }

    /// Append a sibling node in the tree before the given node.
    pub fn insert_before_of(nodes: &mut [TreeNode], id: &NodeId, new_sibling_id: &NodeId) {
        Self::remove_from_parent(nodes, new_sibling_id);
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let parent_id = node.parent;
        let prev_sibling_id = node.prev_sibling;

        node.prev_sibling = Some(*new_sibling_id);

        if let Some(new_sibling) = nodes.get_mut(new_sibling_id.value) {
            new_sibling.parent = parent_id;
            new_sibling.prev_sibling = prev_sibling_id;
            new_sibling.next_sibling = Some(*id);
        };

        if let Some(parent) = parent_id.and_then(|id| nodes.get_mut(id.value)) {
            if parent.first_child == Some(*id) {
                parent.first_child = Some(*new_sibling_id);
            }
        }

        if let Some(prev_sibling) = prev_sibling_id.and_then(|id| nodes.get_mut(id.value)) {
            prev_sibling.next_sibling = Some(*new_sibling_id);
        }
    }

    /// Append a sibling node in the tree after the given node.
    pub fn insert_after_of(nodes: &mut [TreeNode], id: &NodeId, new_sibling_id: &NodeId) {
        Self::remove_from_parent(nodes, new_sibling_id);
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let parent_id = node.parent;
        let next_sibling_id = node.next_sibling;

        node.next_sibling = Some(*new_sibling_id);

        if let Some(new_sibling) = nodes.get_mut(new_sibling_id.value) {
            new_sibling.parent = parent_id;
            new_sibling.prev_sibling = Some(*id);
            new_sibling.next_sibling = next_sibling_id;
        };

        if let Some(parent) = parent_id.and_then(|id| nodes.get_mut(id.value)) {
            if parent.last_child == Some(*id) {
                parent.last_child = Some(*new_sibling_id);
            }
        }

        if let Some(next_sibling) = next_sibling_id.and_then(|id| nodes.get_mut(id.value)) {
            next_sibling.prev_sibling = Some(*new_sibling_id);
        }
    }

    /// Remove a node from the its parent by id. The node remains in the tree.
    /// It is possible to assign it to another node in the tree after this operation.
    pub fn remove_from_parent(nodes: &mut [TreeNode], id: &NodeId) {
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };
        let parent_id = node.parent;
        let prev_sibling_id = node.prev_sibling;
        let next_sibling_id = node.next_sibling;

        if parent_id.is_none() && prev_sibling_id.is_none() && next_sibling_id.is_none() {
            return;
        }

        node.parent = None;
        node.next_sibling = None;
        node.prev_sibling = None;

        if let Some(parent) = parent_id.and_then(|id| nodes.get_mut(id.value)) {
            if parent.first_child == Some(*id) {
                parent.first_child = next_sibling_id;
            }

            if parent.last_child == Some(*id) {
                parent.last_child = prev_sibling_id;
            }
        }

        if let Some(prev_sibling) = prev_sibling_id.and_then(|id| nodes.get_mut(id.value)) {
            prev_sibling.next_sibling = next_sibling_id;
        }

        if let Some(next_sibling) = next_sibling_id.and_then(|id| nodes.get_mut(id.value)) {
            next_sibling.prev_sibling = prev_sibling_id;
        }
    }

    /// Changes the parent of children nodes of a node.
    pub fn reparent_children_of(
        nodes: &mut [TreeNode],
        id: &NodeId,
        new_parent_id: Option<NodeId>,
    ) {
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let first_child_id = node.first_child;
        let last_child_id = node.last_child;
        node.first_child = None;
        node.last_child = None;

        if let Some(new_parent_id) = new_parent_id {
            if let Some(new_parent) = nodes.get_mut(new_parent_id.value) {
                new_parent.first_child = first_child_id;
                new_parent.last_child = last_child_id;
            }
        }
        let mut next_child_id = first_child_id;
        while let Some(child_id) = next_child_id {
            if let Some(child) = nodes.get_mut(child_id.value) {
                child.parent = new_parent_id;
                next_child_id = child.next_sibling;
            }
        }
    }
}
