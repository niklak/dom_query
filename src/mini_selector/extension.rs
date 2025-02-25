use std::cell::Ref;

use super::parser::parse_selector_list;
use super::selector::{Combinator, MiniSelector};
use crate::node::child_nodes;
use crate::node::{NodeId, TreeNode};
use crate::NodeRef;

pub fn find_descendant_ids<'a>(
    nodes: &Ref<Vec<TreeNode>>,
    id: NodeId,
    path: &'a str,
) -> Result<Vec<NodeId>, nom::Err<nom::error::Error<&'a str>>> {
    // Start with the provided node ID as the initial working set
    let mut tops = vec![id];
    // Final collection of matching node IDs
    let mut res = vec![];

    // Parse the CSS selector list and process each selector sequentially
    let (_, selectors) = parse_selector_list(path)?;
    'work_loop: for (idx, sel) in selectors.iter().enumerate() {
        let is_last = selectors.len() - 1 == idx;

        // Process all current top-level nodes before moving to the next selector
        while let Some(id) = tops.pop() {
            // Collect immediate children that are elements (for potential matching)
            let mut ops: Vec<NodeId> = child_nodes(Ref::clone(nodes), &id, is_last)
                .filter(|id| nodes[id.value].is_element())
                .collect();
            // Collection of nodes that match the current selector
            let mut candidates = vec![];

            // Depth-first traversal of the element tree from the current node
            while let Some(node_id) = ops.pop() {
                // Since these nodes are descendants of the primary node and
                // were previously extracted from the `Tree` with only elements remaining,
                // `else` case should be unreachable.
                let tree_node = &nodes[node_id.value];

                // If the node matches the current selector, add it to candidates
                if sel.match_tree_node(tree_node) {
                    candidates.push(node_id);
                    if !is_last {
                        continue;
                    }
                }

                // For child combinator ('>'), only immediate children are considered
                if matches!(sel.combinator, Combinator::Child) {
                    continue;
                }

                // For descendant combinator (space), add all children to the traversal stack
                ops.extend(
                    child_nodes(Ref::clone(nodes), &node_id, is_last)
                        .filter(|id| nodes[id.value].is_element()),
                );
            }
            // If processing the last selector, add matches to final results
            // Otherwise, use matches as starting points for the next selector
            if is_last {
                res.extend(candidates);
            } else {
                tops.extend(candidates);

                // Continue with the next selector since we've updated the tops
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
    /// This method uses [`MiniSelector`] for matching elements—please note its limitations.
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
    /// This method uses [`MiniSelector`] for matching elements—please note its limitations.
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

    /// Checks if this node matches the given CSS selector.
    ///
    /// This method uses [`MiniSelector`] for matching elements—please note its limitations.
    /// It is faster than [`NodeRef::is`] method but has limitations.
    ///
    /// # Arguments
    ///
    /// * `css_sel` - The CSS selector used to match this node.
    ///
    /// # Returns
    ///
    /// `true` if this node matches the given CSS selector, `false` otherwise.
    pub fn snap_is(&self, css_sel: &str) -> bool {
        MiniSelector::new(css_sel).map_or(false, |(_, sel)| self.snap_match(&sel))
    }

    /// Checks if this node matches the given CSS selector.
    ///
    /// This method uses the given `MiniSelector` for matching elements.
    /// It is faster than [`NodeRef::is_match`] method but has limitations.
    ///
    /// # Arguments
    ///
    /// * `matcher` - The `MiniSelector` used to match this node.
    ///
    /// # Returns
    ///
    /// `true` if this node matches the given CSS selector, `false` otherwise.
    pub fn snap_match(&self, matcher: &MiniSelector) -> bool {
        matcher.match_node(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, NodeId};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg(target_arch = "wasm32")]
    #[global_allocator]
    pub static ALLOC: &alloc_cat::AllocCat = &alloc_cat::ALLOCATOR;

    #[test]
    fn test_names() {
        let sel = r#"body td a"#;
        let parsed = parse_selector_list(sel).unwrap();
        assert_eq!(parsed.1.len(), 3);
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_node_find_descendants() {
        let html_contents = include_str!("../../test-pages/hacker_news.html");
        let doc = Document::from(html_contents);
        let a_sel = doc.select(r#"body td.title a[href^="https://"]"#);
        let expected_ids: Vec<NodeId> = a_sel.nodes().iter().map(|n| n.id).collect();

        let root = doc.root();
        let got_ids: Vec<NodeId> = root
            .find_descendants(r#"body td.title a[href^="https://"]"#)
            .iter()
            .map(|n| n.id)
            .collect();

        assert_eq!(got_ids, expected_ids);

        let a_sel = doc.select("a");
        let expected_ids: Vec<NodeId> = a_sel.nodes().iter().map(|n| n.id).collect();
        let got_ids_a: Vec<NodeId> = root.find_descendants("a").iter().map(|n| n.id).collect();
        assert_eq!(got_ids_a, expected_ids);

        let len_fin_ne = root.find_descendants("body td p").len();
        assert_eq!(len_fin_ne, 0);
        let len_sel_ne = doc.select("body td p").length();
        assert_eq!(len_sel_ne, 0)
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_node_find_descendant_combinators() {
        let html_contents = include_str!("../../test-pages/hacker_news.html");
        let doc = Document::from(html_contents);
        let selectors = ["body td.title a", "body td.title > a"];

        for sel in selectors {
            let a_sel = doc.select(sel);
            let expected_ids: Vec<NodeId> = a_sel.nodes().iter().map(|n| n.id).collect();
            let root = doc.root();
            let got_ids: Vec<NodeId> = root.find_descendants(sel).iter().map(|n| n.id).collect();
            assert_eq!(got_ids, expected_ids);
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_node_snap_match() {
        let contents = r#"<div>
            <a id="main-link" class="link text-center bg-blue-400 border" href="https://example.com/main-page/" target>Example</a>
            <a class="other-link" href="https://example.com/another-page/">Another Example</a>
        </div>"#;
        let doc = Document::fragment(contents);
        let link_sel = doc.select_single(r#"a[id]"#);
        let link_node = link_sel.nodes().first().unwrap();
        assert!(!link_node.snap_is(r#"a[href="//example.com"]"#));
        assert!(link_node.snap_is(r#"a[href^="https://"]"#));
        assert!(link_node.snap_is(r#"a[href$="/"]"#));
        assert!(link_node.snap_is(r#"a[href*="example.com"]"#));
        assert!(link_node.snap_is(r#"a[id|="main"]"#));
        assert!(link_node.snap_is(r#"a[class~="border"]"#));
        assert!(link_node.snap_is(r#"[class *= "blue-400 bord"]"#));
        assert!(!link_node.snap_is(r#"[class *= "glue-400 bord"]"#));
        assert!(link_node.snap_is(r#"#main-link"#));
        assert!(!link_node.snap_is(r#"#link"#));
        assert!(!link_node.snap_is(r#"a[target="_blank"]"#));
        assert!(link_node.snap_is(r#"a[target]"#));

        let another_sel = doc.select_single(r#"a.other-link"#);
        let another_link_node = another_sel.nodes().first().unwrap();
        let text_node = another_link_node.first_child().unwrap();

        assert!(!another_link_node.snap_is(r#"#main-link"#));
        assert!(!text_node.snap_is(r#"#main-link"#));

    }
}
