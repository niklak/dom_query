use tendril::StrTendril;

use crate::{Document, Selection};

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
