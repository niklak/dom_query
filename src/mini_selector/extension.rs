use std::cell::Ref;

use super::parser::parse_selector_list;
use super::selector::Combinator;
use crate::node::child_nodes;
use crate::node::{NodeId, TreeNode};
use crate::NodeRef;

pub fn find_descendant_ids<'a>(
    nodes: &Ref<Vec<TreeNode>>,
    id: NodeId,
    path: &'a str,
) -> Result<Vec<NodeId>, nom::Err<nom::error::Error<&'a str>>> {
    let mut tops = vec![id];
    let mut res = vec![];

    let (_, selectors) = parse_selector_list(path)?;
    'work_loop: for (idx, sel) in selectors.iter().enumerate() {
        let is_last = selectors.len() - 1 == idx;

        while let Some(id) = tops.pop() {
            let mut ops: Vec<NodeId> = child_nodes(Ref::clone(nodes), &id, is_last)
                .filter(|id| nodes[id.value].is_element())
                .collect();
            let mut candidates = vec![];

            while let Some(node_id) = ops.pop() {
                // Since these nodes are descendants of the primary node and
                // were previously extracted from the `Tree` with only elements remaining,
                // `else` case should be unreachable.
                let tree_node = &nodes[node_id.value];

                if sel.match_tree_node(tree_node) {
                    candidates.push(node_id);
                    if !is_last {
                        continue;
                    }
                }

                if matches!(sel.combinator, Combinator::Child) {
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
    Ok(res)
}

impl NodeRef<'_> {
    /// Finds all descendant elements of this node that match the given CSS selector.
    ///
    /// The method returns a vector of descendant `NodeRef` elements that match the selector.
    /// The elements are returned in the order they appear in the document tree.
    ///
    /// Current support is limited: it supports only the `child` (`>`) and `descendant` (` `) combinators.
    /// It does not support the `selector list` combinator (`,`) or any pseudo-classes.
    /// Each selector in the chain may contain at most one attribute selector.
    ///
    /// # Experimental
    /// This method is experimental and may change in the future.
    ///
    /// # Arguments
    ///
    /// * `css_path` - The CSS selector used to match descendant elements.
    ///
    /// # Returns
    ///
    /// A vector of descendant `NodeRef` elements matching the selector.
    pub fn find_descendants(&self, css_path: &str) -> Vec<NodeRef> {
        self.try_find_descendants(css_path)
            .unwrap_or_else(|_| vec![])
    }

    /// Finds all descendant elements of this node that match the given CSS selector.
    ///
    /// The method returns a vector of descendant `NodeRef` elements that match the selector.
    /// The elements are returned in the order they appear in the document tree.
    ///
    /// Current support is limited: it supports only the `child` (`>`) and `descendant` (` `) combinators.
    /// It does not support the `selector list` combinator (`,`) or any pseudo-classes.
    /// Each selector in the chain may contain at most one attribute selector.
    ///
    /// # Experimental
    /// This method is experimental and may change in the future.
    ///
    /// # Arguments
    ///
    /// * `css_path` - The CSS selector used to match descendant elements.
    ///
    /// # Returns
    ///
    /// A vector of descendant `NodeRef` elements matching the selector.
    ///
    /// # Errors
    ///
    /// Returns an error if the CSS selector is invalid.

    pub fn try_find_descendants<'a>(
        &self,
        css_path: &'a str,
    ) -> Result<Vec<NodeRef>, nom::Err<nom::error::Error<&'a str>>> {
        let nodes = self.tree.nodes.borrow();
        let found_ids = find_descendant_ids(&nodes, self.id, css_path)?;
        let res = found_ids
            .iter()
            .map(|node_id| NodeRef::new(*node_id, self.tree))
            .collect();
        Ok(res)
    }
}
