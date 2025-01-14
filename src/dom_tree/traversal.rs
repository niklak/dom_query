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
            .filter_map(|node_id| nodes.get(node_id.value))
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
                .map_or(false, |el| el.node_name().as_ref() == name)
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
        let mut tops = vec![id];
        let mut res = vec![];
        'work_loop: for (idx, name) in path.iter().enumerate() {
            let is_last = path.len() - 1 == idx;

            while let Some(id) = tops.pop() {
                let mut ops: Vec<NodeId> = child_nodes(Ref::clone(nodes), &id, is_last)
                    .filter(|id| nodes[id.value].is_element())
                    .collect();
                let mut candidates = vec![];

                while let Some(node_id) = ops.pop() {
                    let Some(node_name) = nodes
                        .get(node_id.value)
                        .and_then(|n| n.as_element().map(|el| el.node_name()))
                    else {
                        continue;
                    };

                    if node_name.as_ref() == *name {
                        candidates.push(node_id);
                        continue;
                    }
                    ops.extend(
                        child_nodes(Ref::clone(nodes), &node_id, is_last)
                            .filter(|id| nodes[id.value].is_element()),
                    );
                }
                if is_last {
                    res.extend(candidates);
                } else {
                    tops.extend(candidates);

                    continue 'work_loop;
                }
            }
        }
        res
    }
}
