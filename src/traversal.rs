use std::vec::IntoIter;

use crate::matcher::{MatchScope, Matcher, Matches};
use crate::{Document, Node, Selection};

impl Document {
    /// Gets the descendants of the root document node in the current, filter by a selector.
    /// It returns a new selection object containing these matched elements.
    ///
    /// # Panics
    ///
    /// Panics if failed to parse the given CSS selector.
    pub fn select(&self, sel: &str) -> Selection {
        let matcher = Matcher::new(sel).expect("Invalid CSS selector");
        self.select_matcher(&matcher)
    }

    /// Alias for `select`, it gets the descendants of the root document node in the current, filter by a selector.
    /// It returns a new selection object containing these matched elements.
    ///
    /// # Panics
    ///
    /// Panics if failed to parse the given CSS selector.
    pub fn nip(&self, sel: &str) -> Selection {
        self.select(sel)
    }

    /// Gets the descendants of the root document node in the current, filter by a selector.
    /// It returns a new selection object containing these matched elements.
    pub fn try_select(&self, sel: &str) -> Option<Selection> {
        Matcher::new(sel).ok().and_then(|matcher| {
            let selection = self.select_matcher(&matcher);
            if !selection.is_empty() {
                Some(selection)
            } else {
                None
            }
        })
    }

    /// Gets the descendants of the root document node in the current, filter by a matcher.
    /// It returns a new selection object containing these matched elements.
    pub fn select_matcher(&self, matcher: &Matcher) -> Selection {
        let root = self.tree.root();
        let nodes = Matches::from_one(root, matcher, MatchScope::IncludeNode).collect();

        Selection { nodes }
    }
    /// Gets the descendants of the root document node in the current, filter by a matcher.
    /// It returns a new selection object containing elements of the single (first) match.    
    pub fn select_single_matcher(&self, matcher: &Matcher) -> Selection {
        let node = Matches::from_one(self.tree.root(), matcher, MatchScope::IncludeNode).next();

        match node {
            Some(node) => Selection { nodes: vec![node] },
            None => Selection { nodes: vec![] },
        }
    }

    /// Gets the descendants of the root document node in the current, filter by a selector.
    /// It returns a new selection object containing elements of the single (first) match.
    ///
    /// # Panics
    ///
    /// Panics if failed to parse the given CSS selector.
    pub fn select_single(&self, sel: &str) -> Selection {
        let matcher = Matcher::new(sel).expect("Invalid CSS selector");
        self.select_single_matcher(&matcher)
    }
}

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
