use std::cell::Ref;
use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;

use html5ever::serialize;
use html5ever::serialize::SerializeOpts;
use html5ever::serialize::TraversalScope;
use html5ever::Attribute;

use tendril::StrTendril;

use crate::entities::{copy_attrs, into_tendril, StrWrap};
use crate::Document;
use crate::Tree;
use crate::TreeNodeOps;

use super::child_nodes;
use super::id_provider::NodeIdProver;
use super::inner::TreeNode;
use super::node_data::NodeData;
use super::serializing::SerializableNodeRef;
use super::NodeId;

pub type Node<'a> = NodeRef<'a>;

#[derive(Clone, Debug)]
/// Represents a reference to a node in the tree.
/// It keeps a node id and a reference to the tree,
/// which allows to access to the actual tree node with [NodeData].
pub struct NodeRef<'a> {
    pub id: NodeId,
    pub tree: &'a Tree,
}

impl<'a> NodeRef<'a> {
    /// Creates a new node reference.
    pub fn new(id: NodeId, tree: &'a Tree) -> Self {
        Self { id, tree }
    }

    /// Selects the node from the tree and applies a function to it.
    #[inline]
    pub fn query<F, B>(&self, f: F) -> Option<B>
    where
        F: FnOnce(&TreeNode) -> B,
    {
        self.tree.query_node(&self.id, f)
    }

    /// Selects the node from the tree and applies a function to it.
    /// Accepts a default value to return for a case if the node doesn't exist.
    #[inline]
    pub fn query_or<F, B>(&self, default: B, f: F) -> B
    where
        F: FnOnce(&TreeNode) -> B,
    {
        self.tree.query_node_or(&self.id, default, f)
    }

    /// Selects the node from the tree and applies a function to it.
    #[inline]
    pub fn update<F, B>(&self, f: F) -> Option<B>
    where
        F: FnOnce(&mut TreeNode) -> B,
    {
        self.tree.update_node(&self.id, f)
    }

    /// Returns the parent node of the selected node.
    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.tree.parent_of(&self.id)
    }

    /// Returns the child nodes of the selected node.
    #[inline]
    pub fn children(&self) -> Vec<Self> {
        self.tree.children_of(&self.id)
    }

    /// Returns the iterator child nodes of the selected node.
    #[inline]
    pub fn children_it(&self, rev: bool) -> impl Iterator<Item = Self> {
        self.tree
            .child_ids_of_it(&self.id, rev)
            .map(|n| NodeRef::new(n, self.tree))
    }

    /// Returns ancestor nodes of the selected node.
    ///
    /// # Arguments
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    ///
    /// # Returns
    ///
    /// `Vec<Self>`
    #[inline]
    pub fn ancestors(&self, max_depth: Option<usize>) -> Vec<Self> {
        self.tree.ancestors_of(&self.id, max_depth)
    }

    /// Returns the iterator ancestor nodes of the selected node.
    ///
    /// # Arguments
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    ///
    /// # Returns
    /// impl Iterator<Item = Self>
    #[inline]
    pub fn ancestors_it(&self, max_depth: Option<usize>) -> impl Iterator<Item = Self> {
        self.tree
            .ancestor_ids_of_it(&self.id, max_depth)
            .map(|n| NodeRef::new(n, self.tree))
    }

    /// Returns the descendant nodes of the selected node.
    ///
    /// # Returns
    /// `Vec<NodeRef>` -- a vector of descendant nodes
    #[inline]
    pub fn descendants(&self) -> Vec<Self> {
        self.descendants_it().collect()
    }

    /// Returns an iterator of the descendant nodes of the selected node.
    ///
    /// # Returns
    /// impl Iterator<Item = Self> -- an iterator of descendant nodes
    #[inline]
    pub fn descendants_it(&self) -> impl Iterator<Item = Self> {
        self.tree
            .descendant_ids_of_it(&self.id)
            .map(|n| NodeRef::new(n, self.tree))
    }

    /// Returns the first child node of the selected node.
    #[inline]
    pub fn first_child(&self) -> Option<Self> {
        self.tree.first_child_of(&self.id)
    }

    /// Returns the last child node of the selected node.
    #[inline]
    pub fn last_child(&self) -> Option<Self> {
        self.tree.last_child_of(&self.id)
    }

    /// Returns the next sibling node of the selected node.
    #[inline]
    pub fn next_sibling(&self) -> Option<Self> {
        self.tree.next_sibling_of(&self.id)
    }

    /// Returns the previous sibling node of the selected node.
    #[inline]
    pub fn prev_sibling(&self) -> Option<Self> {
        self.tree.prev_sibling_of(&self.id)
    }

    /// Returns the last sibling node of the selected node.
    #[inline]
    pub fn last_sibling(&self) -> Option<Self> {
        self.tree.last_sibling_of(&self.id)
    }
}

// NodeRef modification methods
impl NodeRef<'_> {
    /// Removes the selected node from its parent node, but keeps it in the tree.
    #[inline]
    pub fn remove_from_parent(&self) {
        self.tree.remove_from_parent(&self.id)
    }

    /// Removes all children nodes of the selected node.
    #[inline]
    pub fn remove_children(&self) {
        self.tree.remove_children_of(&self.id)
    }

    /// Appends another node by id to the parent node of the selected node.
    /// Another node takes place of the selected node.
    #[inline]
    #[deprecated(since = "0.10.0", note = "please use `insert_before` instead")]
    pub fn append_prev_sibling<P: NodeIdProver>(&self, id_provider: P) {
        self.insert_before(id_provider);
    }
    /// Inserts another node by id before the selected node.
    /// Another node takes place of the selected node shifting it to right.
    #[inline]
    pub fn insert_before<P: NodeIdProver>(&self, id_provider: P) {
        self.tree.insert_before_of(&self.id, id_provider.node_id())
    }

    /// Inserts another node by id after the selected node.
    /// Another node takes place of the next sibling of the selected node.
    pub fn insert_after<P: NodeIdProver>(&self, id_provider: P) {
        self.tree.insert_after_of(&self.id, id_provider.node_id())
    }

    /// Appends another node by id to the selected node.
    #[inline]
    pub fn append_child<P: NodeIdProver>(&self, id_provider: P) {
        let new_child_id = id_provider.node_id();
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::remove_from_parent(nodes.deref_mut(), new_child_id);
        TreeNodeOps::append_child_of(nodes.deref_mut(), &self.id, new_child_id);
    }

    /// Appends another node and it's siblings to the selected node.
    #[inline]
    pub fn append_children<P: NodeIdProver>(&self, id_provider: P) {
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::append_children_of(&mut nodes, &self.id, id_provider.node_id());
    }

    /// Prepend another node by id to the selected node.
    #[inline]
    pub fn prepend_child<P: NodeIdProver>(&self, id_provider: P) {
        let new_child_id = id_provider.node_id();
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::remove_from_parent(nodes.deref_mut(), new_child_id);
        TreeNodeOps::prepend_child_of(nodes.deref_mut(), &self.id, new_child_id);
    }

    /// Prepend another node and it's siblings to the selected node.
    pub fn prepend_children<P: NodeIdProver>(&self, id_provider: P) {
        // avoiding call borrow
        let new_child_id = id_provider.node_id();
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::prepend_children_of(&mut nodes, &self.id, new_child_id);
    }

    /// Appends another node and it's siblings to the parent node
    /// of the selected node.
    #[inline]
    #[deprecated(since = "0.10.0", note = "please use `insert_siblings_before` instead")]
    pub fn append_prev_siblings<P: NodeIdProver>(&self, id_provider: P) {
        self.insert_siblings_before(id_provider);
    }

    /// Inserts another node and it's siblings before the current node
    /// shifting itself.
    #[inline]
    pub fn insert_siblings_before<P: NodeIdProver>(&self, id_provider: P) {
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::insert_siblings_before(nodes.deref_mut(), &self.id, id_provider.node_id());
    }

    /// Inserts another node and it's siblings after the current node.
    #[inline]
    pub fn insert_siblings_after<P: NodeIdProver>(&self, id_provider: P) {
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::insert_siblings_after(nodes.deref_mut(), &self.id, id_provider.node_id());
    }

    /// Replaces the current node with other node by id. It'is actually a shortcut of two operations:
    /// [`NodeRef::insert_before`] and [`NodeRef::remove_from_parent`].
    pub fn replace_with<P: NodeIdProver>(&self, id_provider: P) {
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::insert_before_of(nodes.deref_mut(), &self.id, id_provider.node_id());
        TreeNodeOps::remove_from_parent(&mut nodes, &self.id);
    }

    /// Replaces the current node with other node, created from the given fragment html.
    /// Behaves similarly to [`crate::Selection::replace_with_html`] but only for one node.
    pub fn replace_with_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {

        self.merge_html_with_fn(html, |tree_nodes, new_node_id, node| {
            TreeNodeOps::insert_siblings_before(tree_nodes, &node.id, &new_node_id);
            TreeNodeOps::remove_from_parent(tree_nodes, &node.id);
        });
    }

    /// Parses given fragment html and appends its contents to the selected node.
    pub fn append_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        self.merge_html_with_fn(html, |tree_nodes, new_node_id, node|{
            TreeNodeOps::append_children_of(tree_nodes,&node.id, &new_node_id);
        });
    }

    /// Parses given fragment html and appends its contents to the selected node.
    pub fn prepend_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        self.merge_html_with_fn(html, |tree_nodes, new_node_id, node|{
            TreeNodeOps::prepend_children_of(tree_nodes,&node.id, &new_node_id);
        });

    }

    /// Parses given fragment html inserts its contents before to the selected node.
    pub fn before_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        self.merge_html_with_fn(html, |tree_nodes, new_node_id, node|{
            TreeNodeOps::insert_siblings_before(tree_nodes,&node.id, &new_node_id);
        });
    }

    /// Parses given fragment html inserts its contents after to the selected node.
    pub fn after_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        self.merge_html_with_fn(html, |tree_nodes, new_node_id, node|{
            TreeNodeOps::insert_siblings_after(tree_nodes,&node.id, &new_node_id);
        });

    }

    /// Parses given fragment html and sets its contents to the selected node.
    pub fn set_html<T>(&self, html: T)
    where
        T: Into<StrTendril>,
    {
        self.remove_children();
        self.append_html(html);
    }

    /// Parses given text and sets its contents to the selected node.
    ///
    ///
    /// This operation replaces any contents of the selected node with the given text.
    /// Doesn't escapes the text.
    pub fn set_text<T>(&self, text: T)
    where
        T: Into<StrTendril>,
    {
        let mut nodes = self.tree.nodes.borrow_mut();
        TreeNodeOps::set_text(nodes.deref_mut(), &self.id, text);
    }


    /// Parses given fragment html and appends its contents to the selected node.
    fn merge_html_with_fn<T, F>(&self, html: T, f: F)
    where
        T: Into<StrTendril>,
        F: Fn(&mut Vec<TreeNode>, NodeId, &NodeRef),
    {
        let fragment = Document::fragment(html);
        TreeNodeOps::merge_with_fn(self.tree, fragment.tree, |tree_nodes, new_node_id| {
            f(tree_nodes, new_node_id, self);
        });
    }
}

impl NodeRef<'_> {
    /// Returns the next sibling, that is an [`NodeData::Element`] of the selected node.
    pub fn next_element_sibling(&self) -> Option<Self> {
        let nodes = self.tree.nodes.borrow();
        TreeNodeOps::next_element_sibling_of(nodes.deref(), &self.id)
            .map(|id| NodeRef::new(id, self.tree))
    }

    /// Returns the previous sibling, that is an [`NodeData::Element`] of the selected node.
    pub fn prev_element_sibling(&self) -> Option<Self> {
        let nodes = self.tree.nodes.borrow();
        TreeNodeOps::prev_element_sibling_of(nodes.deref(), &self.id)
            .map(|id| NodeRef::new(id, self.tree))
    }

    /// Returns the first child, that is an [`NodeData::Element`] of the selected node.
    pub fn first_element_child(&self) -> Option<Self> {
        let nodes = self.tree.nodes.borrow();
        TreeNodeOps::first_element_child_of(nodes.deref(), &self.id)
            .map(|id| NodeRef::new(id, self.tree))
    }

    /// Returns children, that are [`NodeData::Element`]s of the selected node.
    pub fn element_children(&self) -> Vec<Self> {
        self.children_it(false).filter(|n| n.is_element()).collect()
    }
}

impl NodeRef<'_> {
    /// Returns the name of the selected node if it is an [`NodeData::Element`] otherwise `None`.
    pub fn node_name(&self) -> Option<StrTendril> {
        let nodes = self.tree.nodes.borrow();
        nodes
            .get(self.id.value)
            .and_then(|node| node.as_element().map(|e| e.node_name()))
    }

    /// Checks if node has a specified class
    pub fn has_class(&self, class: &str) -> bool {
        self.query_or(false, |node| {
            node.as_element().map_or(false, |e| e.has_class(class))
        })
    }

    /// Adds a class to the node
    pub fn add_class(&self, class: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.add_class(class);
            }
        });
    }

    /// Removes a class from the node
    pub fn remove_class(&self, class: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_class(class);
            }
        });
    }

    /// Returns the value of the specified attribute
    pub fn attr(&self, name: &str) -> Option<StrTendril> {
        self.query_or(None, |node| node.as_element().and_then(|e| e.attr(name)))
    }

    /// Returns the value of the specified attribute
    pub fn attr_or<T>(&self, name: &str, default: T) -> StrTendril
    where
        tendril::Tendril<tendril::fmt::UTF8>: std::convert::From<T>,
    {
        self.query_or(None, |node| node.as_element().and_then(|e| e.attr(name)))
            .unwrap_or_else(|| StrTendril::from(default))
    }

    /// Returns all attributes
    pub fn attrs(&self) -> Vec<Attribute> {
        self.query_or(vec![], |node| {
            node.as_element().map_or(vec![], |e| copy_attrs(&e.attrs))
        })
    }

    /// Sets the value of the specified attribute to the node.
    pub fn set_attr(&self, name: &str, val: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.set_attr(name, val);
            }
        });
    }

    /// Removes the specified attribute from the element.
    pub fn remove_attr(&self, name: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_attr(name);
            }
        });
    }

    /// Removes the specified attributes from the element.
    pub fn remove_attrs(&self, names: &[&str]) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_attrs(names);
            }
        });
    }

    /// Removes all attributes from the element.
    pub fn remove_all_attrs(&self) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.remove_all_attrs();
            }
        });
    }

    /// Checks if node has a specified attribute
    pub fn has_attr(&self, name: &str) -> bool {
        self.query_or(false, |node| {
            node.as_element().map_or(false, |e| e.has_attr(name))
        })
    }

    /// Renames the node if node is an [`NodeData::Element`].
    pub fn rename(&self, name: &str) {
        self.update(|node| {
            if let Some(element) = node.as_element_mut() {
                element.rename(name);
            }
        });
    }
}

impl NodeRef<'_> {
    /// Returns true if this node is a document.
    pub fn is_document(&self) -> bool {
        self.query_or(false, |node| node.is_document())
    }

    /// Returns true if this node is a fragment.
    pub fn is_fragment(&self) -> bool {
        self.query_or(false, |node| node.is_fragment())
    }

    /// Returns true if this node is an element.
    pub fn is_element(&self) -> bool {
        self.query_or(false, |node| node.is_element())
    }

    /// Returns true if this node is a text node.
    pub fn is_text(&self) -> bool {
        self.query_or(false, |node| node.is_text())
    }
    /// Returns true if this node is a comment.
    pub fn is_comment(&self) -> bool {
        self.query_or(false, |node| node.is_comment())
    }
    /// Returns true if this node is a DOCTYPE.
    pub fn is_doctype(&self) -> bool {
        self.query_or(false, |node| node.is_doctype())
    }

    /// Checks if node may have children nodes.
    pub fn may_have_children(&self) -> bool {
        self.query_or(false, |node| node.may_have_children())
    }
}

impl NodeRef<'_> {
    /// Returns the HTML representation of the DOM tree.
    /// Panics if serialization fails.
    pub fn html(&self) -> StrTendril {
        self.serialize_html(TraversalScope::IncludeNode).unwrap()
    }

    /// Returns the HTML representation of the DOM tree without the outermost node.
    /// Panics if serialization fails.
    pub fn inner_html(&self) -> StrTendril {
        self.serialize_html(TraversalScope::ChildrenOnly(None))
            .unwrap()
    }

    // Returns the HTML representation of the DOM tree, if it succeeds or `None`.
    pub fn try_html(&self) -> Option<StrTendril> {
        self.serialize_html(TraversalScope::IncludeNode)
    }

    // Returns the HTML representation of the DOM tree without the outermost node, if it succeeds or `None`.
    pub fn try_inner_html(&self) -> Option<StrTendril> {
        self.serialize_html(TraversalScope::ChildrenOnly(None))
    }

    fn serialize_html(&self, traversal_scope: TraversalScope) -> Option<StrTendril> {
        let inner: SerializableNodeRef = self.clone().into();
        let mut result = vec![];
        serialize(
            &mut result,
            &inner,
            SerializeOpts {
                scripting_enabled: false,
                create_missing_parent: false,
                traversal_scope,
            },
        )
        .ok()?;
        StrTendril::try_from_byte_slice(&result).ok()
    }

    /// Returns the text of the node and its descendants.
    pub fn text(&self) -> StrTendril {
        let nodes = self.tree.nodes.borrow();
        TreeNodeOps::text_of(nodes, self.id)
    }

    /// Returns the text of the node without its descendants.
    pub fn immediate_text(&self) -> StrTendril {
        let mut text = StrWrap::new();

        self.children_it(false).for_each(|n| {
            n.query(|inner| {
                if let NodeData::Text { ref contents } = inner.data {
                    text.push_tendril(contents)
                }
            });
        });

        into_tendril(text)
    }

    /// Checks if the node contains the specified text
    pub fn has_text(&self, needle: &str) -> bool {
        let mut ops = vec![self.id];
        let nodes = self.tree.nodes.borrow();
        while let Some(id) = ops.pop() {
            if let Some(node) = nodes.get(id.value) {
                match node.data {
                    NodeData::Element(_) => {
                        // since here we don't care about the order we can skip .rev()
                        // and intermediate collecting into vec.
                        ops.extend(child_nodes(Ref::clone(&nodes), &id, false));
                    }

                    NodeData::Text { ref contents } => {
                        if contents.contains(needle) {
                            return true;
                        }
                    }

                    _ => continue,
                }
            }
        }
        false
    }

    /// Checks if the node contains only text node
    pub fn has_only_text(&self) -> bool {
        let nodes = self.tree.nodes.borrow();
        if child_nodes(Ref::clone(&nodes), &self.id, false).count() == 1 {
            let first_child = nodes
                .get(self.id.value)
                .and_then(|n| n.first_child)
                .and_then(|id| nodes.get(id.value));
            first_child.map_or(false, |n| {
                n.is_text()
                    && !TreeNodeOps::text_of(Ref::clone(&nodes), n.id)
                        .trim()
                        .is_empty()
            })
        } else {
            false
        }
    }

    /// Checks if the node is an empty element.
    ///
    /// Determines if the node is an element, has no child elements, and any text nodes
    /// it contains consist only of whitespace.
    pub fn is_empty_element(&self) -> bool {
        let nodes = self.tree.nodes.borrow();
        let Some(node) = nodes.get(self.id.value) else {
            return false;
        };
        node.is_element()
            && !child_nodes(Ref::clone(&nodes), &self.id, false)
                .flat_map(|id| nodes.get(id.value))
                .any(|child| {
                    child.is_element()
                        || (child.is_text()
                            && !TreeNodeOps::text_of(Ref::clone(&nodes), child.id)
                                .trim()
                                .is_empty())
                })
    }

    /// Merges adjacent text nodes and removes empty text nodes.
    ///
    /// Normalization is necessary to ensure that adjacent text nodes are merged into one text node.
    pub fn normalize(&self) {
        let mut child = self.first_child();
        let mut text: StrTendril = StrTendril::new();

        while let Some(ref node) = child {
            let next_node = node.next_sibling();

            if node.is_text() {
                text.push_tendril(&node.text());
                if !next_node.as_ref().map_or(false, |n| n.is_text()) && !text.is_empty() {
                    let t = text;
                    text = StrTendril::new();
                    node.set_text(t);
                } else {
                    node.remove_from_parent();
                }
            } else if node.may_have_children() {
                node.normalize();
            }
            child = next_node;
        }
    }
}
