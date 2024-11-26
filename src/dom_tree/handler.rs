use std::cell::Ref;

use tendril::StrTendril;

use crate::node::child_nodes;
use crate::node::{NodeData, NodeId, TreeNode};
pub struct TreeNodeHandler {}

impl TreeNodeHandler {
    pub fn text_of(id: NodeId, nodes: Ref<Vec<TreeNode>>) -> StrTendril {
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
}
