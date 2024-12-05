use std::cell::Ref;
use std::ops::Deref;
use std::vec::IntoIter;

use html5ever::Attribute;
use tendril::StrTendril;

use crate::document::Document;
use crate::matcher::{MatchScope, Matcher, Matches};
use crate::node::{ancestor_nodes, child_nodes, NodeRef, TreeNode};
use crate::TreeNodeOps;

/// Selection represents a collection of nodes matching some criteria. The
/// initial Selection object can be created by using [`Document::select`], and then
/// manipulated using methods itself.
#[derive(Debug, Clone, Default)]
pub struct Selection<'a> {
    pub(crate) nodes: Vec<NodeRef<'a>>,
}

impl<'a> From<NodeRef<'a>> for Selection<'a> {
    fn from(node: NodeRef<'a>) -> Selection<'a> {
        Self { nodes: vec![node] }
    }
}

impl<'a> From<Vec<NodeRef<'a>>> for Selection<'a> {
    fn from(nodes: Vec<NodeRef<'a>>) -> Selection<'a> {
        Self { nodes }
    }
}

// property methods
impl Selection<'_> {
    /// Gets the specified attribute's value for the first element in the
    /// selection. To get the value for each element individually, use a looping
    /// construct such as map method.
    pub fn attr(&self, name: &str) -> Option<StrTendril> {
        self.nodes().first().and_then(|node| node.attr(name))
    }

    /// Gets all attributes` values for the first element in the
    /// selection. To get the value for each element individually, use a looping
    /// construct such as map method.
    pub fn attrs(&self) -> Vec<Attribute> {
        self.nodes()
            .first()
            .map_or_else(Vec::new, |node| node.attrs())
    }

    /// Checks if the first element in the selection has an attribute with the name.
    pub fn has_attr(&self, name: &str) -> bool {
        self.nodes()
            .first()
            .map_or(false, |node| node.has_attr(name))
    }

    /// Works like `attr` but returns default value if attribute is not present.
    pub fn attr_or(&self, name: &str, default: &str) -> StrTendril {
        self.attr(name).unwrap_or_else(|| StrTendril::from(default))
    }

    /// Sets the given attribute to each element in the set of matched elements.
    pub fn set_attr(&self, name: &str, val: &str) {
        for node in self.nodes() {
            node.set_attr(name, val);
        }
    }

    /// Removes the named attribute from each element in the set of matched elements.
    pub fn remove_attr(&self, name: &str) {
        for node in self.nodes() {
            node.remove_attr(name);
        }
    }

    /// Removes named attributes from each element in the set of matched elements.
    pub fn remove_attrs(&self, names: &[&str]) {
        for node in self.nodes() {
            node.remove_attrs(names);
        }
    }

    /// Removes all attributes from each element in the set of matched elements.
    pub fn remove_all_attrs(&self) {
        for node in self.nodes() {
            node.remove_all_attrs();
        }
    }

    /// Renames tag of each element in the set of matched elements.
    pub fn rename(&self, name: &str) {
        for node in self.nodes() {
            node.rename(name);
        }
    }

    /// Adds the given class to each element in the set of matched elements.
    /// Multiple class names can be specified, separated by a space via multiple arguments.
    pub fn add_class(&self, class: &str) {
        for node in self.nodes() {
            node.add_class(class);
        }
    }

    /// Determines whether any of the matched elements are assigned the
    /// given class.
    pub fn has_class(&self, class: &str) -> bool {
        self.nodes().iter().any(|node| node.has_class(class))
    }

    /// Removes the given class from each element in the set of matched elements.
    /// Multiple class names can be specified, separated by a space via multiple arguments.
    pub fn remove_class(&self, class: &str) {
        for node in self.nodes() {
            node.remove_class(class);
        }
    }

    /// Returns the number of elements in the selection object.
    pub fn length(&self) -> usize {
        self.nodes().len()
    }

    /// Is an alias for `length`.
    pub fn size(&self) -> usize {
        self.length()
    }

    /// Is there any matched elements.
    pub fn exists(&self) -> bool {
        self.length() > 0
    }

    // Returns true if there are no elements in the selection object.
    // A common, simple and clear function.
    pub fn is_empty(&self) -> bool {
        self.length() == 0
    }

    /// Gets the HTML contents of the first element in the set of matched
    /// elements. It includes the first matching element and its children nodes.
    pub fn html(&self) -> StrTendril {
        self.nodes
            .first()
            .map_or_else(StrTendril::new, |node| node.html())
    }

    /// Gets the HTML contents of the first element in the set of matched
    /// elements. It includes only children nodes.
    pub fn inner_html(&self) -> StrTendril {
        self.nodes
            .first()
            .map_or_else(StrTendril::new, |node| node.inner_html())
    }

    /// Gets the HTML contents of the first element in the set of matched
    /// elements. It includes the first matching element and its children nodes.
    pub fn try_html(&self) -> Option<StrTendril> {
        self.nodes().first().and_then(|node| node.try_html())
    }

    /// Gets the HTML contents of the first element in the set of matched
    /// elements. It includes only children nodes.
    pub fn try_inner_html(&self) -> Option<StrTendril> {
        self.nodes().first().and_then(|node| node.try_inner_html())
    }

    /// Gets the combined text content of each element in the set of matched
    /// elements, including their descendants.
    pub fn text(&self) -> StrTendril {
        let mut s = StrTendril::new();

        for node in self.nodes() {
            s.push_tendril(&node.text());
        }
        s
    }

    /// Gets the combined text content of each element in the set of matched, without their descendants.
    pub fn immediate_text(&self) -> StrTendril {
        let mut s = StrTendril::new();
        for node in self.nodes() {
            s.push_tendril(&node.immediate_text());
        }
        s
    }
}

//matching methods
impl<'a> Selection<'a> {
    /// Checks the current matched set of elements against a selector and
    /// returns true if at least one of these elements matches.
    pub fn is(&self, sel: &str) -> bool {
        Matcher::new(sel).map_or(false, |matcher| self.is_matcher(&matcher))
    }

    /// Checks the current matched set of elements against a matcher and
    /// returns true if at least one of these elements matches.
    pub fn is_matcher(&self, matcher: &Matcher) -> bool {
        if self.length() > 0 {
            return self.nodes().iter().any(|node| matcher.match_element(node));
        }
        false
    }

    /// Checks the current matches set of elements against a selection and
    /// returns true if at least one of these elements matches.
    pub fn is_selection(&self, other: &Selection) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        let m: Vec<usize> = other.nodes().iter().map(|node| node.id.value).collect();
        self.nodes().iter().any(|node| m.contains(&node.id.value))
    }

    /// Filters the current set of matched elements to those that match the
    /// given CSS selector.
    ///
    /// # Panics
    ///
    /// # Arguments
    ///
    /// * `sel` - The CSS selector to match against.
    ///
    /// # Returns
    ///
    /// A new Selection object containing the matched elements.
    pub fn filter(&self, sel: &str) -> Selection<'a> {
        if self.is_empty() {
            return self.clone();
        }
        let matcher = Matcher::new(sel).expect("Invalid CSS selector");
        self.filter_matcher(&matcher)
    }

    /// Reduces the current set of matched elements to those that match the
    /// given CSS selector.
    ///
    /// # Arguments
    ///
    /// * `sel` - The CSS selector to match against.
    ///
    /// # Returns
    ///
    ///  `None` if the selector was invalid, otherwise a new `Selection` object containing the matched elements.
    pub fn try_filter(&self, sel: &str) -> Option<Selection<'a>> {
        if self.is_empty() {
            return Some(self.clone());
        }
        Matcher::new(sel).ok().map(|m| self.filter_matcher(&m))
    }

    /// Reduces the current set of matched elements to those that match the
    /// given matcher.
    ///
    /// # Arguments
    ///
    /// * `matcher` - The matcher to match against.
    ///
    /// # Returns
    ///
    /// A new Selection object containing the matched elements.
    pub fn filter_matcher(&self, matcher: &Matcher) -> Selection<'a> {
        if self.is_empty() {
            return self.clone();
        }
        let nodes = self
            .nodes()
            .iter()
            .filter(|&node| matcher.match_element(node))
            .cloned()
            .collect();
        Selection { nodes }
    }

    /// Reduces the set of matched elements to those that match a node in the specified `Selection`.
    /// It returns a new `Selection` for this subset of elements.
    pub fn filter_selection(&self, other: &Selection) -> Selection<'a> {
        if self.is_empty() || other.is_empty() {
            return self.clone();
        }
        let m: Vec<usize> = other.nodes().iter().map(|node| node.id.value).collect();
        let nodes = self
            .nodes()
            .iter()
            .filter(|&node| m.contains(&node.id.value))
            .cloned()
            .collect();
        Selection { nodes }
    }

    /// Adds nodes that match the given CSS selector to the current selection.
    ///
    /// # Panics
    ///
    /// If matcher contains invalid CSS selector it panics.
    ///
    /// # Arguments
    ///
    /// * `sel` - The CSS selector to match against.
    ///
    /// # Returns
    ///
    /// The new `Selection` containing the original nodes and the new nodes.
    pub fn add(&self, sel: &str) -> Selection<'a> {
        if self.is_empty() {
            return self.clone();
        }
        let matcher = Matcher::new(sel).expect("Invalid CSS selector");
        self.add_matcher(&matcher)
    }

    /// Adds nodes that match the given CSS selector to the current selection.
    ///
    /// If matcher contains invalid CSS selector it returns `None`.
    ///
    /// # Arguments
    ///
    /// * `sel` - The CSS selector to match against.
    ///
    /// # Returns
    ///
    /// The new `Selection` containing the original nodes and the new nodes.
    pub fn try_add(&self, sel: &str) -> Option<Selection> {
        if self.is_empty() {
            return Some(self.clone());
        }
        Matcher::new(sel).ok().map(|m| self.add_matcher(&m))
    }

    /// Adds nodes that match the given matcher to the current selection.
    ///
    /// # Arguments
    ///
    /// * `matcher` - The matcher to match against.
    ///
    /// # Returns
    ///
    /// The new `Selection` containing the original nodes and the new nodes.
    pub fn add_matcher(&self, matcher: &Matcher) -> Selection<'a> {
        if self.is_empty() {
            return self.clone();
        }
        let root = self.nodes().first().unwrap().tree.root();
        let other_nodes: Vec<NodeRef> =
            Matches::from_one(root, matcher, MatchScope::IncludeNode).collect();
        let new_nodes = self.merge_nodes(other_nodes);
        Selection { nodes: new_nodes }
    }

    /// Adds a selection to the current selection.
    ///
    /// Behaves like `Union` for sets.
    ///
    /// # Arguments
    ///
    /// * `other` - The selection to add to the current selection.
    ///
    /// # Returns
    ///
    /// A new `Selection` object containing the combined elements.
    pub fn add_selection(&self, other: &'a Selection) -> Selection<'a> {
        if self.is_empty() {
            return other.clone();
        }

        if other.is_empty() {
            return self.clone();
        }

        self.ensure_same_tree(other);

        let other_nodes = other.nodes();
        let new_nodes = self.merge_nodes(other_nodes.to_vec());

        Selection { nodes: new_nodes }
    }

    fn merge_nodes(&self, other_nodes: Vec<NodeRef<'a>>) -> Vec<NodeRef<'a>> {
        let m: Vec<usize> = self.nodes().iter().map(|node| node.id.value).collect();
        let add_nodes: Vec<NodeRef> = other_nodes
            .iter()
            .filter(|&node| !m.contains(&node.id.value))
            .cloned()
            .collect();

        let mut new_nodes = self.nodes().to_vec();
        new_nodes.extend(add_nodes);
        new_nodes
    }
}

//manipulating methods
impl Selection<'_> {
    /// Removes the set of matched elements from the document.
    pub fn remove(&self) {
        for node in &self.nodes {
            node.remove_from_parent()
        }
    }

    /// Set the html contents of each element in the selection to specified parsed HTML.
    pub fn set_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        for node in self.nodes() {
            node.remove_children();
        }

        self.append_html(html)
    }

    /// Replaces each element in the set of matched elements with
    /// the parsed HTML.
    ///
    /// This follows the same rules as `append`.
    pub fn replace_with_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        let fragment = Document::fragment(html);

        for node in self.nodes().iter() {
            node.tree.merge_with_fn(fragment.tree.clone(), |node_id| {
                node.insert_siblings_before(&node_id)
            });
        }

        self.remove()
    }

    /// Replaces each element in the set of matched element with
    /// the nodes from the given selection.
    ///
    /// This follows the same rules as `append`.
    ///
    pub fn replace_with_selection(&self, sel: &Selection) {
        //! Note: goquery's behavior is taken as the basis.
        if sel.is_empty() {
            return;
        }

        sel.remove();

        let sel_nodes = sel.nodes();
        for node in self.nodes() {
            node.tree
                .copy_nodes_with_fn(sel_nodes, |new_node_id| node.insert_before(&new_node_id));
        }

        self.remove()
    }

    /// Appends the elements in the selection to the end of each element
    /// in the set of matched elements.
    pub fn append_selection(&self, sel: &Selection) {
        //! Note: goquery's behavior is taken as the basis.

        if sel.is_empty() {
            return;
        }

        sel.remove();
        let sel_nodes = sel.nodes();
        for node in self.nodes() {
            node.tree
                .copy_nodes_with_fn(sel_nodes, |new_node_id| node.append_children(&new_node_id));
        }
    }

    /// Parses the html and appends it to the set of matched elements.
    pub fn append_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        let fragment = Document::fragment(html);

        for node in self.nodes().iter() {
            node.tree.merge_with_fn(fragment.tree.clone(), |node_id| {
                node.append_children(&node_id)
            });
        }
    }

    /// Parses the html and prepends it to the set of matched elements.
    pub fn prepend_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        let fragment = Document::fragment(html);

        for node in self.nodes().iter() {
            node.tree.merge_with_fn(fragment.tree.clone(), |node_id| {
                node.prepend_children(&node_id)
            });
        }
    }
}

// traversing methods
impl<'a> Selection<'a> {
    /// Gets the descendants of each element in the current set of matched
    /// elements, filter by a selector. It returns a new Selection object
    /// containing these matched elements.
    ///
    /// # Panics
    ///
    /// Panics if failed to parse the given CSS selector.
    pub fn select<'b>(&self, sel: &'b str) -> Selection<'a>
    where
        'a: 'b,
    {
        let matcher = Matcher::new(sel).expect("Invalid CSS selector");
        self.select_matcher(&matcher)
    }

    /// Gets the descendants of each element in the current set of matched
    /// elements, filter by a matcher. It returns a new Selection object
    /// containing these matched elements.
    pub fn select_matcher(&self, matcher: &Matcher) -> Selection<'a> {
        Selection {
            nodes: Matches::from_list(
                self.nodes.clone().into_iter(),
                matcher,
                MatchScope::ChildrenOnly,
            )
            .collect(),
        }
    }

    /// Alias for `select`, it gets the descendants of each element in the current set of matched
    /// elements, filter by a selector. It returns a new Selection object
    /// containing these matched elements.
    ///
    /// # Panics
    ///
    /// Panics if failed to parse the given CSS selector.
    pub fn nip(&self, sel: &'a str) -> Selection<'a> {
        self.select(sel)
    }

    /// Gets the descendants of each element in the current set of matched
    /// elements, filter by a selector. It returns a new Selection object
    /// containing these matched elements.
    pub fn try_select(&self, sel: &str) -> Option<Selection<'a>> {
        Matcher::new(sel).ok().and_then(|matcher| {
            let selection = self.select_matcher(&matcher);
            if selection.is_empty() {
                None
            } else {
                Some(selection)
            }
        })
    }

    /// Gets the descendants of each element in the current set of matched
    /// elements, filter by a matcher. It returns a new Selection object
    /// containing elements of the single (first) match..
    pub fn select_single_matcher(&self, matcher: &Matcher) -> Selection<'a> {
        let node = Matches::from_list(
            self.nodes.clone().into_iter(),
            matcher,
            MatchScope::ChildrenOnly,
        )
        .next();

        match node {
            Some(node) => Selection { nodes: vec![node] },
            None => Selection { nodes: vec![] },
        }
    }

    /// Gets the descendants of each element in the current set of matched elements, filter by a selector.
    /// It returns a new selection object containing elements of the single (first) match.
    ///
    /// # Panics
    ///
    /// Panics if failed to parse the given CSS selector.
    pub fn select_single(&self, sel: &str) -> Selection<'a> {
        let matcher = Matcher::new(sel).expect("Invalid CSS selector");
        self.select_single_matcher(&matcher)
    }

    /// Returns a slice of underlying nodes.
    pub fn nodes(&self) -> &[NodeRef<'a>] {
        &self.nodes
    }

    /// Creates an iterator over these matched elements.
    pub fn iter(&self) -> Selections<NodeRef<'a>> {
        Selections::new(self.nodes.clone().into_iter())
    }

    /// Gets the parent of each element in the selection. It returns a
    /// mew Selection object containing these elements.
    pub fn parent(&self) -> Selection<'a> {
        self.derive_selection(|tree_nodes, node| {
            let tree_node = tree_nodes.get(node.id.value)?;
            tree_node.parent.map(|id| NodeRef {
                id,
                tree: node.tree,
            })
        })
    }

    /// Gets the child elements of each element in the selection.
    /// It returns a new Selection object containing these elements.
    pub fn children(&self) -> Selection<'a> {
        let Some(first) = self.nodes().first() else {
            return Default::default();
        };

        let mut set = Vec::with_capacity(self.length());
        let tree_nodes = first.tree.nodes.borrow();

        for node in self.nodes() {
            for child in child_nodes(Ref::clone(&tree_nodes), &node.id, false)
                .flat_map(|id| tree_nodes.get(id.value))
            {
                if !set.contains(&child.id) && child.is_element() {
                    set.push(child.id);
                }
            }
        }

        let result = set.iter().map(|id| NodeRef::new(*id, first.tree)).collect();
        Self { nodes: result }
    }

    /// Gets the ancestor elements of each element in the selection.
    ///
    /// # Arguments
    ///
    /// * `max_depth` - The maximum depth of the ancestors to retrieve.
    ///
    /// # Returns
    ///
    /// A new `Selection` object containing these elements.
    pub fn ancestors(&self, max_depth: Option<usize>) -> Selection<'a> {
        let Some(first) = self.nodes().first() else {
            return Default::default();
        };

        let mut set = Vec::with_capacity(self.length());
        let tree_nodes = first.tree.nodes.borrow();

        for node in self.nodes() {
            for child in ancestor_nodes(Ref::clone(&tree_nodes), &node.id, max_depth)
                .flat_map(|id| tree_nodes.get(id.value))
            {
                if !set.contains(&child.id) && child.is_element() {
                    set.push(child.id);
                }
            }
        }

        let result = set.iter().map(|id| NodeRef::new(*id, first.tree)).collect();
        Self { nodes: result }
    }

    #[deprecated(since = "0.1.6", note = "Please use `next_sibling`")]
    /// Gets the immediately following sibling of each element in the
    /// selection. It returns a new Selection object containing these elements.
    pub fn next(&self) -> Selection<'a> {
        self.next_sibling()
    }

    /// Gets the immediately following sibling of each element in the
    /// selection. It returns a new Selection object containing these elements.
    pub fn next_sibling(&self) -> Selection<'a> {
        self.derive_selection(|tree_nodes, node| {
            TreeNodeOps::next_element_sibling_of(tree_nodes.deref(), &node.id).map(|id| NodeRef {
                id,
                tree: node.tree,
            })
        })
    }

    /// Gets the immediately previous sibling of each element in the
    /// selection. It returns a new Selection object containing these elements.
    pub fn prev_sibling(&self) -> Selection<'a> {
        self.derive_selection(|tree_nodes, node| {
            TreeNodeOps::prev_element_sibling_of(tree_nodes.deref(), &node.id).map(|id| NodeRef {
                id,
                tree: node.tree,
            })
        })
    }

    fn derive_selection<'b, F>(&self, f: F) -> Selection<'a>
    where
        F: Fn(Ref<Vec<TreeNode>>, &NodeRef<'a>) -> Option<NodeRef<'a>>,
    {
        let Some(first) = self.nodes().first() else {
            return Default::default();
        };

        let mut set = Vec::with_capacity(self.length());
        let tree_nodes = first.tree.nodes.borrow();
        for node in self.nodes() {
            if let Some(derive) = f(Ref::clone(&tree_nodes), node) {
                if !set.contains(&derive.id) {
                    set.push(derive.id);
                }
            }
        }

        let result = set.iter().map(|id| NodeRef::new(*id, first.tree)).collect();
        Self { nodes: result }
    }

    /// Reduces the set of matched elements to the first in the set.
    /// It returns a new selection object, and an empty selection object if the
    /// selection is empty.
    pub fn first(&self) -> Selection<'a> {
        if self.length() > 0 {
            Selection::from(self.nodes[0].clone())
        } else {
            Default::default()
        }
    }

    /// Reduces the set of matched elements to the last in the set.
    /// It returns a new selection object, and an empty selection object if the
    /// selection is empty.
    pub fn last(&self) -> Selection<'a> {
        if self.length() > 0 {
            Selection::from(self.nodes[self.length() - 1].clone())
        } else {
            Default::default()
        }
    }

    /// Retrieves the underlying node at the specified index.
    pub fn get(&self, index: usize) -> Option<&NodeRef<'a>> {
        self.nodes.get(index)
    }
}

impl Selection<'_> {
    /// Ensures that the two selections are from the same tree.
    ///
    /// # Panics
    ///
    /// Panics if the selections are from different trees or if they are empty.
    fn ensure_same_tree(&self, other: &Selection) {
        let tree = self.nodes().first().unwrap().tree;
        let other_tree = other.nodes().first().unwrap().tree;
        if !std::ptr::eq(tree, other_tree) {
            panic!("Selections must be from the same tree");
        }
    }
}

/// Iterator over a collection of matched elements.
pub struct Selections<I> {
    iter: IntoIter<I>,
}

impl<I> Selections<I> {
    fn new(iter: IntoIter<I>) -> Self {
        Self { iter }
    }
}

impl<'a> Iterator for Selections<NodeRef<'a>> {
    type Item = Selection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Selection::from)
    }
}

impl DoubleEndedIterator for Selections<NodeRef<'_>> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(Selection::from)
    }
}
