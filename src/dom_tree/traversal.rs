use std::cell::Ref;

use crate::node::child_nodes;
use crate::node::{NodeId, TreeNode};

pub struct Traversal {}

impl Traversal {
    /// Finds the first child element of a node that satisfies the given predicate.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `id` - The id of the parent node.
    /// * `f` - The predicate to apply to each child element.
    ///
    /// # Returns
    ///
    /// The id of the first element that satisfies the predicate, if any.
    pub fn find_child_element<F>(nodes: Ref<Vec<TreeNode>>, id: NodeId, f: F) -> Option<NodeId>
    where
        F: Fn(&TreeNode) -> bool,
    {
        child_nodes(Ref::clone(&nodes), &id, false)
            .filter_map(|node_id| nodes.get(node_id.value()))
            .filter(|tree_node| tree_node.is_element())
            .find(|tree_node| f(tree_node))
            .map(|tree_node| tree_node.id)
    }

    /// Finds the first child element of a node that has the given name.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `id` - The id of the parent node.
    /// * `name` - The name of the element to search for.
    ///
    /// # Returns
    ///
    /// The id of the first element that has the given name, if any.
    pub fn find_child_element_by_name(
        nodes: Ref<Vec<TreeNode>>,
        id: NodeId,
        name: &str,
    ) -> Option<NodeId> {
        Self::find_child_element(nodes, id, |tree_node| {
            tree_node
                .as_element()
                .is_some_and(|el| el.node_name().as_ref() == name)
        })
    }

    /// Finds the first descendant element of a node that match given path.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `id` - The id of the starting node.
    /// * `names` - The sequence of element names to search for. Currently, only element names are supported.
    ///
    /// # Returns
    ///
    /// The id of the first descendant element that has the given names, if any.
    pub fn find_descendant_element(
        nodes: Ref<Vec<TreeNode>>,
        id: NodeId,
        path: &[&str],
    ) -> Option<NodeId> {
        path.iter().try_fold(id, |current_id, name| {
            Self::find_child_element_by_name(Ref::clone(&nodes), current_id, name)
        })
    }

    /// Finds all descendant elements of a node that match given path.
    ///
    /// # Arguments
    ///
    /// * `nodes` - The nodes of the tree.
    /// * `id` - The id of the starting node.
    /// * `path` - The sequence of element names to search for. Currently, only element names are supported.
    ///
    /// # Returns
    ///
    /// A list of ids of all descendant elements that have the given names.
    ///
    /// # Experimental
    ///
    /// This method is experimental and may change in the future. The `path` argument will be revised.
    pub fn find_descendant_elements(
        nodes: &Ref<Vec<TreeNode>>,
        id: NodeId,
        path: &[&str],
    ) -> Vec<NodeId> {
        let mut stack = vec![id];
        let mut iter_stack = Vec::new();
        for (idx, name) in path.iter().enumerate() {
            let is_last = path.len() - 1 == idx;
            let mut new_stack = vec![];

            for node_id in &stack {
                collect_matching_descendants(
                    nodes,
                    node_id,
                    name,
                    is_last,
                    &mut new_stack,
                    &mut iter_stack,
                );
            }
            stack = new_stack;
        }
        stack
    }
}

fn collect_matching_descendants(
    nodes: &Ref<'_, Vec<TreeNode>>,
    current_node_id: &NodeId,
    matching_name: &str,
    is_last: bool,
    results: &mut Vec<NodeId>,
    pending: &mut Vec<NodeId>,
) {
    // Walk with `first_child`/`next_sibling` and a stack of pending siblings instead of recursion,
    // so deep trees do not overflow the stack. The stack is reused across calls.
    pending.clear();
    let mut next = nodes[current_node_id.value()].first_child;
    loop {
        let Some(child_id) = next else {
            match pending.pop() {
                Some(sibling) => next = Some(sibling),
                None => break,
            }
            continue;
        };
        let tree_node = &nodes[child_id.value()];
        next = tree_node.next_sibling;

        let Some(node_name) = tree_node.as_element().map(|el| el.node_name()) else {
            continue;
        };
        let matched = node_name.as_ref() == matching_name;

        if matched {
            results.push(child_id);
        }

        // Continue the search only if:
        // 1. The node does NOT match the selector.
        // 2. OR this is the last selector in the path.
        if !matched || is_last {
            if let Some(sibling) = next {
                pending.push(sibling);
            }
            next = tree_node.first_child;
        }
    }
}
