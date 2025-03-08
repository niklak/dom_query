
use super::selector::{MiniSelector, MiniSelectorList};
use crate::NodeRef;

pub fn find_descendant_ids<'a, 'b>(
    node: &'a NodeRef,
    path: &'b str,
)  -> Result<Vec<NodeRef<'a>>, nom::Err<nom::error::Error<&'b str>>> where 'b: 'a {
    // Start with the provided node ID as the initial working set
    let mut descendants = node.descendants_it();
    // Final collection of matching node IDs
    let mut res = vec![];

    // Parse the CSS selector list and process each selector sequentially
    let selectors = MiniSelectorList::new(path)?;

    while let Some(node) = descendants.next() {

        if selectors.match_node(&node) {
            res.push(node);
        }
    }
    Ok(res)
}

impl <'a>NodeRef<'a> {
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
    pub fn find_descendants<'b>(&'a self, css_path: &'b str) -> Vec<NodeRef<'a>> where 'b: 'a {
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
    pub fn try_find_descendants<'b>(
        &self,
        css_path: &'b str,
    ) -> Result<Vec<NodeRef>, nom::Err<nom::error::Error<&'a str>>>  where 'b: 'a {
        let found_ids = find_descendant_ids( self, css_path)?;
        let res = found_ids;
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
    pub fn mini_is(&self, css_sel: &str) -> bool {
        MiniSelector::new(css_sel).map_or(false, |sel| self.mini_match(&sel))
    }

    /// Checks if this node matches the given CSS selector.
    ///
    /// This method uses the given [`MiniSelector`] for matching elements.
    /// It is faster than [`NodeRef::is_match`] method but has limitations.
    ///
    /// # Arguments
    ///
    /// * `matcher` - The `MiniSelector` used to match this node.
    ///
    /// # Returns
    ///
    /// `true` if this node matches the given CSS selector, `false` otherwise.
    pub fn mini_match(&self, matcher: &MiniSelector) -> bool {
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
            println!("{}: {}", got_ids.len(), expected_ids.len());
            assert_eq!(got_ids, expected_ids);
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_node_mini_match() {
        let contents = r#"<div>
            <a id="main-link" class="link text-center bg-blue-400 border" href="https://example.com/main-page/" target>Example</a>
            <a class="other-link" href="https://example.com/another-page/">Another Example</a>
        </div>"#;
        let doc = Document::fragment(contents);
        let link_sel = doc.select_single(r#"a[id]"#);
        let link_node = link_sel.nodes().first().unwrap();
        assert!(!link_node.mini_is(r#"a[href="//example.com"]"#));
        assert!(link_node.mini_is(r#"a[href^="https://"]"#));
        assert!(link_node.mini_is(r#"a[href$="/"]"#));
        assert!(link_node.mini_is(r#"a[href*="example.com"]"#));
        assert!(link_node.mini_is(r#"a[id|="main"]"#));
        assert!(link_node.mini_is(r#"a[class~="border"]"#));
        assert!(link_node.mini_is(r#"[class *= "blue-400 bord"]"#));
        assert!(!link_node.mini_is(r#"[class *= "glue-400 bord"]"#));
        assert!(link_node.mini_is(r#"#main-link"#));
        assert!(!link_node.mini_is(r#"#link"#));
        assert!(!link_node.mini_is(r#"a[target="_blank"]"#));
        assert!(link_node.mini_is(r#"a[target]"#));
        assert!(!link_node.mini_is(r#"a[href^="https://"][href*="examplxe"][href$="/"]"#));
        assert!(link_node.mini_is(r#"a[href^="https://"][href*="example"][href$="/"]"#));

        let another_sel = doc.select_single(r#"a.other-link"#);
        let another_link_node = another_sel.nodes().first().unwrap();
        let text_node = another_link_node.first_child().unwrap();

        assert!(!another_link_node.mini_is(r#"#main-link"#));
        assert!(!text_node.mini_is(r#"#main-link"#));
    }
}