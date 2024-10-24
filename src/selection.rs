use std::vec::IntoIter;

use html5ever::Attribute;
use tendril::StrTendril;

use crate::matcher::{MatchScope, Matcher, Matches};
use crate::Document;

use crate::node::{Node, NodeData, NodeRef};

/// Selection represents a collection of nodes matching some criteria. The
/// initial Selection object can be created by using [`crate::document::Document::select`], and then
/// manipulated using methods itself.
#[derive(Debug, Clone, Default)]
pub struct Selection<'a> {
    pub(crate) nodes: Vec<Node<'a>>,
}

impl<'a> From<Node<'a>> for Selection<'a> {
    fn from(node: Node<'a>) -> Selection {
        Self { nodes: vec![node] }
    }
}

impl<'a> From<Vec<NodeRef<'a, NodeData>>> for Selection<'a> {
    fn from(nodes: Vec<NodeRef<'a, NodeData>>) -> Selection {
        Self { nodes }
    }
}

// property methods
impl<'a> Selection<'a> {
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
}

//matching methods
impl<'a> Selection<'a> {
    /// Checks the current matched set of elements against a selector and
    /// returns true if at least one of these elements matches.
    pub fn is(&self, sel: &str) -> bool {
        if self.length() == 0 {
            return false;
        }
        return Matcher::new(sel).map_or(false, |matcher| self.is_matcher(&matcher));
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
    pub fn is_selection(&self, sel: &Selection) -> bool {
        if self.length() == 0 || sel.length() == 0 {
            return false;
        }
        let m: Vec<usize> = sel.nodes().iter().map(|node| node.id.value).collect();
        self.nodes().iter().any(|node| m.contains(&node.id.value))
    }
}

//manipulating methods
impl<'a> Selection<'a> {
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
    /// It returns the removed elements.
    ///
    /// This follows the same rules as `append`.
    pub fn replace_with_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        let dom = Document::fragment(html);

        for (i, node) in self.nodes().iter().enumerate() {
            if i + 1 == self.size() {
                node.append_prev_siblings_from_another_tree(dom.tree);
                break;
            } else {
                node.append_prev_siblings_from_another_tree(dom.tree.clone());
            }
        }

        self.remove()
    }

    /// Replaces each element in the set of matched element with
    /// the nodes from the given selection.
    ///
    /// This follows the same rules as `append`.
    pub fn replace_with_selection(&self, sel: &Selection) {
        for node in self.nodes() {
            for prev_sibling in sel.nodes() {
                node.append_prev_sibling(&prev_sibling.id);
            }
        }

        self.remove()
    }

    /// Parses the html and appends it to the set of matched elements.
    pub fn append_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        let dom = Document::fragment(html);

        for (i, node) in self.nodes().iter().enumerate() {
            if i + 1 == self.size() {
                node.append_children_from_another_tree(dom.tree);
                break;
            } else {
                node.append_children_from_another_tree(dom.tree.clone());
            }
        }
    }

    /// Appends the elements in the selection to the end of each element
    /// in the set of matched elements.
    pub fn append_selection(&self, sel: &Selection) {
        for node in self.nodes() {
            for child in sel.nodes() {
                node.append_child(&child.id);
            }
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
    pub fn nodes(&self) -> &[Node<'a>] {
        &self.nodes
    }

    /// Creates an iterator over these matched elements.
    pub fn iter(&self) -> Selections<Node<'a>> {
        Selections::new(self.nodes.clone().into_iter())
    }

    /// Gets the parent of each element in the selection. It returns a
    /// mew Selection object containing these elements.
    pub fn parent(&self) -> Selection<'a> {
        let mut result = Vec::with_capacity(self.length());
        let mut set = Vec::with_capacity(self.length());

        for node in self.nodes() {
            if let Some(parent) = node.parent() {
                if !set.contains(&parent.id) {
                    set.push(parent.id);
                    result.push(parent);
                }
            }
        }

        Self { nodes: result }
    }

    /// Gets the child elements of each element in the selection.
    /// It returns a new Selection object containing these elements.
    pub fn children(&self) -> Selection<'a> {
        let mut result = Vec::with_capacity(self.length());
        let mut set = Vec::with_capacity(self.length());

        for node in self.nodes() {
            for child in node.children() {
                if !set.contains(&child.id) && child.is_element() {
                    set.push(child.id);
                    result.push(child);
                }
            }
        }

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
        let mut result = Vec::with_capacity(self.length());
        let mut set = Vec::with_capacity(self.length());

        for node in self.nodes() {
            if let Some(sibling) = node.next_element_sibling() {
                if !set.contains(&sibling.id) {
                    set.push(sibling.id);
                    result.push(sibling);
                }
            }
        }

        Self { nodes: result }
    }

    /// Gets the immediately previous sibling of each element in the
    /// selection. It returns a new Selection object containing these elements.
    pub fn prev_sibling(&self) -> Selection<'a> {
        let mut result = Vec::with_capacity(self.length());
        let mut set = Vec::with_capacity(self.length());

        for node in self.nodes() {
            if let Some(sibling) = node.prev_element_sibling() {
                if !set.contains(&sibling.id) {
                    set.push(sibling.id);
                    result.push(sibling);
                }
            }
        }

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
    pub fn get(&self, index: usize) -> Option<&Node<'a>> {
        self.nodes.get(index)
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

impl<'a> Iterator for Selections<Node<'a>> {
    type Item = Selection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Selection::from)
    }
}

impl<'a> DoubleEndedIterator for Selections<Node<'a>> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(Selection::from)
    }
}
