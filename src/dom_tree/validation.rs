// Validation methods
use bit_set::BitSet;

use super::tree::Tree;

impl Tree {
    /// Validates the structural integrity of the tree.
    ///
    /// This function checks for:
    ///
    /// 1. **Root Uniqueness**:
    ///    - Ensures exactly one root node exists (`parent == None`) and it is `NodeId(0)`.
    ///
    /// 2. **Parent-Child Link Consistency**:
    ///    - `first_child.parent == Self`
    ///    - `first_child.prev_sibling == None`
    ///    - `last_child.parent == Self`
    ///    - `last_child.next_sibling == None`
    ///    - All children in the chain from `first_child` to `last_child` have correct `parent`, `prev_sibling`, and `next_sibling` references.
    ///    - A node naming a parent is linked in that parent's child list.
    ///
    /// 3. **Sibling Link Consistency**:
    ///    - `prev_sibling.next_sibling == Self`
    ///    - `next_sibling.prev_sibling == Self`
    ///
    /// 4. **Valid Node References**:
    ///    - All `NodeId` references must point to valid nodes within the tree.
    ///
    ///
    /// 5. **Cycle Detection**:
    ///    - No cycles in parent chains (traverse `parent` links).
    ///    - No cycles in sibling chains (traverse `next_sibling` links).
    ///
    /// Returns:
    /// - `Ok(())` if the tree structure is valid.
    ///
    /// Orphaned nodes (non-root nodes with `parent == None`) are allowed and not considered invalid.
    ///
    /// # Errors
    /// - `Err(String)` with a descriptive message if any inconsistency or cycle is detected.
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self) -> Result<(), String> {
        let nodes = self.nodes.borrow();

        // Root uniqueness check
        let root_node = nodes
            .first()
            .ok_or_else(|| "Root node (NodeId(0)) missing".to_string())?;
        if root_node.parent.is_some() {
            return Err("Root node (NodeId(0)) must have no parent".to_string());
        }

        // Node parent-child / sibling link consistency checks
        let mut linked = BitSet::with_capacity(nodes.len());
        for node in nodes.iter() {
            let id = node.id;

            // Validate self-cycles
            if node.parent == Some(id) {
                return Err(format!("Node {id:?} references itself as parent"));
            }
            if node.prev_sibling == Some(id) {
                return Err(format!("Node {id:?} references itself as prev_sibling"));
            }
            if node.next_sibling == Some(id) {
                return Err(format!("Node {id:?} references itself as next_sibling"));
            }

            // Validate first_child linkage
            if let Some(first_child_id) = node.first_child {
                let first_child = nodes.get(first_child_id.value()).ok_or_else(|| {
                    format!("Invalid first_child {first_child_id:?} reference at node {id:?}")
                })?;

                if first_child.prev_sibling.is_some() {
                    return Err(format!(
                        "first_child {first_child_id:?} of node {id:?} has non-None prev_sibling"
                    ));
                }

                // Validate child chain
                let mut current_id = Some(first_child_id);
                let mut last_seen = None;
                while let Some(cid) = current_id {
                    let current = nodes
                        .get(cid.value())
                        .ok_or_else(|| format!("Invalid child reference at node {cid:?}"))?;

                    if current.parent != Some(id) {
                        return Err(format!(
                            "Child {cid:?} has incorrect parent, expected {id:?}"
                        ));
                    }

                    linked.insert(cid.value());

                    if let Some(next) = current.next_sibling {
                        let next_node = nodes.get(next.value()).ok_or_else(|| {
                            format!("Invalid next_sibling reference at node {next:?}")
                        })?;
                        if next_node.prev_sibling != Some(cid) {
                            return Err(format!(
                                "next_sibling {next:?} prev_sibling mismatch with {cid:?}"
                            ));
                        }
                        current_id = Some(next);
                    } else {
                        last_seen = Some(cid);
                        current_id = None;
                    }
                }

                if node.last_child != last_seen {
                    return Err(format!(
                        "last_child mismatch at node {:?}, expected {:?}, found {:?}",
                        id, node.last_child, last_seen
                    ));
                }
            }

            // Validate last_child linkage
            if let Some(last_child_id) = node.last_child {
                let last_child = nodes
                    .get(last_child_id.value())
                    .ok_or_else(|| format!("Invalid last_child reference at node {id:?}"))?;

                if last_child.parent != Some(id) {
                    return Err(format!("last_child's parent does not match node {id:?}"));
                }

                if last_child.next_sibling.is_some() {
                    return Err(format!(
                        "last_child {last_child_id:?} of node {id:?} has non-None next_sibling"
                    ));
                }
            }

            // Validate sibling links
            if let Some(prev_sibling_id) = node.prev_sibling {
                let prev_sibling = nodes
                    .get(prev_sibling_id.value())
                    .ok_or_else(|| format!("Invalid prev_sibling reference at node {id:?}"))?;

                if prev_sibling.next_sibling != Some(id) {
                    return Err(format!(
                        "prev_sibling {prev_sibling_id:?} does not link back to {id:?}",
                    ));
                }
            }

            if let Some(next_sibling_id) = node.next_sibling {
                let next_sibling = nodes
                    .get(next_sibling_id.value())
                    .ok_or_else(|| format!("Invalid next_sibling reference at node {id:?}"))?;

                if next_sibling.prev_sibling != Some(id) {
                    return Err(format!(
                        "next_sibling {next_sibling_id:?} does not link back to {id:?}"
                    ));
                }
            }
        }

        // Cycle detection: Parent chains
        for node in nodes.iter() {
            let mut visited = BitSet::new();
            let mut current = Some(node.id);
            while let Some(cid) = current {
                if !visited.insert(cid.value()) {
                    return Err(format!("Cycle detected in parent chain at node {cid:?}"));
                }
                current = nodes.get(cid.value()).and_then(|n| n.parent);
            }
        }

        // Reverse check: a node naming a parent must be linked in that parent's child list.
        for node in nodes.iter() {
            let Some(parent_id) = node.parent else {
                continue;
            };
            nodes.get(parent_id.value()).ok_or_else(|| {
                format!(
                    "Invalid parent {parent_id:?} reference at node {:?}",
                    node.id
                )
            })?;

            if !linked.contains(node.id.value()) {
                return Err(format!(
                    "Node {:?} is not linked as a child of {parent_id:?}",
                    node.id
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::node::TreeNode;
    use crate::{Document, NodeId};

    const ROOT: Option<NodeId> = Some(NodeId::new(0));
    const A: Option<NodeId> = Some(NodeId::new(1));
    const B: Option<NodeId> = Some(NodeId::new(2));
    const C: Option<NodeId> = Some(NodeId::new(3));
    const D: Option<NodeId> = Some(NodeId::new(4));
    const E: Option<NodeId> = Some(NodeId::new(5));
    const MISSING: Option<NodeId> = Some(NodeId::new(9999));

    /// Validates `root(0) > a(1) > [b(2), c(3)]` with detached `d(4)` and `e(5)` after `corrupt`.
    #[track_caller]
    fn assert_invalid(expected: &str, corrupt: impl FnOnce(&mut Vec<TreeNode>)) {
        let doc = Document::default();
        let a = doc.tree.new_element("a");
        doc.root().append_child(&a);
        a.append_child(&doc.tree.new_element("b"));
        a.append_child(&doc.tree.new_element("c"));
        doc.tree.new_element("d");
        doc.tree.new_element("e");
        doc.tree.validate().unwrap();
        corrupt(&mut doc.tree.nodes.borrow_mut());
        let err = doc.tree.validate().unwrap_err();
        assert!(err.contains(expected), "{err}");
    }

    #[test]
    fn test_validate_root() {
        assert_invalid("Root node (NodeId(0)) missing", Vec::clear);
        assert_invalid("must have no parent", |n| n[0].parent = A);
    }

    #[test]
    fn test_validate_self_links() {
        assert_invalid("itself as parent", |n| n[4].parent = D);
        assert_invalid("itself as prev_sibling", |n| n[4].prev_sibling = D);
        assert_invalid("itself as next_sibling", |n| n[4].next_sibling = D);
    }

    #[test]
    fn test_validate_child_chain() {
        assert_invalid("Invalid first_child", |n| n[4].first_child = MISSING);
        assert_invalid("has non-None prev_sibling", |n| n[2].prev_sibling = D);
        assert_invalid("has incorrect parent", |n| n[3].parent = ROOT);
        assert_invalid(
            "Invalid next_sibling reference at node NodeId { value: 9999 }",
            |n| n[3].next_sibling = MISSING,
        );
        assert_invalid("prev_sibling mismatch", |n| n[2].next_sibling = D);
        assert_invalid("last_child mismatch", |n| n[1].last_child = B);
    }

    #[test]
    fn test_validate_last_child() {
        assert_invalid("Invalid last_child", |n| n[4].last_child = MISSING);
        assert_invalid("last_child's parent", |n| n[4].last_child = B);
        assert_invalid("has non-None next_sibling", |n| {
            n[4].last_child = E;
            n[5].parent = D;
            n[5].next_sibling = A;
        });
    }

    #[test]
    fn test_validate_sibling_links() {
        assert_invalid(
            "Invalid prev_sibling reference at node NodeId { value: 4 }",
            |n| n[4].prev_sibling = MISSING,
        );
        assert_invalid("prev_sibling NodeId { value: 2 } does not link back", |n| {
            n[4].prev_sibling = B;
        });
        assert_invalid(
            "Invalid next_sibling reference at node NodeId { value: 4 }",
            |n| n[4].next_sibling = MISSING,
        );
        assert_invalid("next_sibling NodeId { value: 3 } does not link back", |n| {
            n[4].next_sibling = C;
        });
    }

    #[test]
    fn test_validate_parent_links() {
        assert_invalid("Cycle detected in parent chain", |n| {
            n[4].parent = E;
            n[5].parent = D;
        });
        assert_invalid("Invalid parent", |n| n[4].parent = MISSING);
        assert_invalid("is not linked as a child", |n| n[4].parent = A);
    }
}
