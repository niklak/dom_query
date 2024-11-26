use std::cell::{Ref, RefCell};
use std::fmt::{self, Debug};

use html5ever::LocalName;
use html5ever::{namespace_url, ns, QualName};
use tendril::StrTendril;

use crate::entities::InnerHashMap;
use crate::node::{
    ancestor_nodes, child_nodes, descendant_nodes, AncestorNodes, ChildNodes, DescendantNodes,
};
use crate::node::{Element, NodeData, NodeId, NodeRef, TreeNode};

use super::handler::TreeNodeHandler;

/// fixes node ids
fn fix_node(n: &mut TreeNode, offset: usize) {
    n.id = NodeId::new(n.id.value + offset);
    n.parent = n.parent.map(|id| NodeId::new(id.value + offset));
    n.prev_sibling = n.prev_sibling.map(|id| NodeId::new(id.value + offset));
    n.next_sibling = n.next_sibling.map(|id| NodeId::new(id.value + offset));
    n.first_child = n.first_child.map(|id| NodeId::new(id.value + offset));
    n.last_child = n.last_child.map(|id| NodeId::new(id.value + offset));
}

/// An implementation of arena-tree.
pub struct Tree {
    pub(crate) nodes: RefCell<Vec<TreeNode>>,
}

impl Debug for Tree {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Tree").finish()
    }
}

impl Clone for Tree {
    fn clone(&self) -> Self {
        let nodes = self.nodes.borrow();
        Self {
            nodes: RefCell::new(nodes.clone()),
        }
    }
}

impl Tree {
    /// Creates a new element with the given name, without parent
    pub fn new_element(&self, name: &str) -> NodeRef {
        let name = QualName::new(None, ns!(), LocalName::from(name));
        let el = Element::new(name.clone(), Vec::new(), None, false);

        let id = self.create_node(NodeData::Element(el));

        NodeRef { id, tree: self }
    }

    /// Creates a new text node with the given text, without parent
    pub fn new_text<T: Into<StrTendril>>(&self, text: T) -> NodeRef {
        let text = text.into();
        let id = self.create_node(NodeData::Text { contents: text });
        NodeRef { id, tree: self }
    }

    /// Gets node's name by by id
    pub fn get_name<'a>(&'a self, id: &NodeId) -> Option<Ref<'a, QualName>> {
        Ref::filter_map(self.nodes.borrow(), |nodes| {
            let node = nodes.get(id.value)?;
            if let NodeData::Element(ref el) = node.data {
                Some(&el.name)
            } else {
                None
            }
        })
        .ok()
    }
}

impl Tree {
    /// Returns the root node.
    pub fn root_id(&self) -> NodeId {
        NodeId { value: 0 }
    }

    /// Creates a new tree with the given root.
    /// `T` is [`NodeData`].
    pub fn new(root: NodeData) -> Self {
        let root_id = NodeId::new(0);
        Self {
            nodes: RefCell::new(vec![TreeNode::new(root_id, root)]),
        }
    }
    /// Creates a new node with the given data.
    pub fn create_node(&self, data: NodeData) -> NodeId {
        let mut nodes = self.nodes.borrow_mut();
        let new_child_id = NodeId::new(nodes.len());

        nodes.push(TreeNode::new(new_child_id, data));
        new_child_id
    }

    /// Gets node by id
    pub fn get(&self, id: &NodeId) -> Option<NodeRef> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value).map(|_| NodeRef {
            id: *id,
            tree: self,
        });
        node
    }

    /// Gets node by id
    pub fn get_unchecked(&self, id: &NodeId) -> NodeRef {
        NodeRef {
            id: *id,
            tree: self,
        }
    }

    /// Gets the root node
    pub fn root(&self) -> NodeRef {
        self.get_unchecked(&NodeId::new(0))
    }

    /// Gets the ancestors nodes of a node by id.
    ///
    /// # Arguments
    /// * `id` - The id of the node.
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    ///
    /// # Returns
    /// `Vec<NodeRef>` A vector of ancestors nodes.
    pub fn ancestors_of(&self, id: &NodeId, max_depth: Option<usize>) -> Vec<NodeRef> {
        self.ancestor_ids_of_it(id, max_depth)
            .map(|id| NodeRef::new(id, self))
            .collect()
    }

    /// Returns the ancestor node ids of a node by id.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    ///
    /// # Returns
    /// `Vec<NodeId>`
    pub fn ancestor_ids_of(&self, id: &NodeId, max_depth: Option<usize>) -> Vec<NodeId> {
        self.ancestor_ids_of_it(id, max_depth).collect()
    }

    /// Returns an iterator of the ancestor node ids of a node by id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    ///
    /// # Returns
    /// `AncestorNodes<'a, T>`
    pub fn ancestor_ids_of_it(&self, id: &NodeId, max_depth: Option<usize>) -> AncestorNodes<'_> {
        ancestor_nodes(self.nodes.borrow(), id, max_depth)
    }

    /// Returns children of the selected node.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    ///
    /// # Returns
    ///
    /// `Vec<NodeRef<T>>` A vector of children nodes.
    pub fn children_of(&self, id: &NodeId) -> Vec<NodeRef> {
        child_nodes(self.nodes.borrow(), id, false)
            .map(move |id| NodeRef::new(id, self))
            .collect()
    }

    /// Returns an iterator of the child node ids of a node by id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    /// * `rev` - If `true`, returns the children in reverse order.
    pub fn child_ids_of_it(&self, id: &NodeId, rev: bool) -> ChildNodes<'_> {
        child_nodes(self.nodes.borrow(), id, rev)
    }

    /// Returns a vector of the child node ids of a node by id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    pub fn child_ids_of(&self, id: &NodeId) -> Vec<NodeId> {
        child_nodes(self.nodes.borrow(), id, false).collect()
    }

    /// Returns an iterator of the descendant node ids of a node by id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    ///
    /// # Returns
    ///
    /// `DescendantNodes<'a, T>`
    pub fn descendant_ids_of_it(&self, id: &NodeId) -> DescendantNodes<'_> {
        descendant_nodes(self.nodes.borrow(), id)
    }

    /// Gets the first child node of a node by id
    pub fn first_child_of(&self, id: &NodeId) -> Option<NodeRef> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.first_child.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the last child node of a node by id
    pub fn last_child_of(&self, id: &NodeId) -> Option<NodeRef> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.last_child.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the parent node of a node by id
    pub fn parent_of(&self, id: &NodeId) -> Option<NodeRef> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.parent.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the previous sibling node of a node by id
    pub fn prev_sibling_of(&self, id: &NodeId) -> Option<NodeRef> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.prev_sibling.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the next sibling node of a node by id
    pub fn next_sibling_of(&self, id: &NodeId) -> Option<NodeRef> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.next_sibling.map(|id| NodeRef { id, tree: self })
    }

    pub fn last_sibling_of(&self, id: &NodeId) -> Option<NodeRef> {
        let mut next_node = self.next_sibling_of(id);
        let mut last_node = None;
        while let Some(ref node) = next_node {
            let n = self.next_sibling_of(&node.id);
            last_node = next_node;
            next_node = n;
        }
        last_node
    }

    /// A helper function to get the node from the tree and apply a function to it.
    pub fn query_node<F, B>(&self, id: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&TreeNode) -> B,
    {
        let nodes = self.nodes.borrow();
        nodes.get(id.value).map(f)
    }

    /// A helper function to get the node from the tree and apply a function to it.
    /// Accepts a default value to return for a case if the node doesn't exist.
    pub fn query_node_or<F, B>(&self, id: &NodeId, default: B, f: F) -> B
    where
        F: FnOnce(&TreeNode) -> B,
    {
        let nodes = self.nodes.borrow();
        nodes.get(id.value).map_or(default, f)
    }

    /// A helper function to get the node from the tree and apply a function to it that modifies it.
    pub fn update_node<F, B>(&self, id: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&mut TreeNode) -> B,
    {
        let mut nodes = self.nodes.borrow_mut();
        let node = nodes.get_mut(id.value)?;
        let r = f(node);
        Some(r)
    }

    /// This function is some kind of: get two nodes from a tree and apply some closure to them.
    /// Possibly will be removed in the future.
    pub fn compare_node<F, B>(&self, a: &NodeId, b: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&TreeNode, &TreeNode) -> B,
    {
        let nodes = self.nodes.borrow();
        let node_a = nodes.get(a.value)?;
        let node_b = nodes.get(b.value)?;

        Some(f(node_a, node_b))
    }
}

// Tree modification methods
impl Tree {
    /// Creates a new element from data  and appends it to a node by id
    pub fn append_child_data_of(&self, id: &NodeId, data: NodeData) {
        let mut nodes = self.nodes.borrow_mut();

        let last_child_id = nodes.get(id.value).and_then(|node| node.last_child);

        let new_child_id = NodeId::new(nodes.len());
        let mut child = TreeNode::new(new_child_id, data);
        let new_child_id_opt = Some(new_child_id);
        child.prev_sibling = last_child_id;
        child.parent = Some(*id);
        nodes.push(child);

        if let Some(id) = last_child_id {
            if let Some(node) = nodes.get_mut(id.value) {
                node.next_sibling = new_child_id_opt
            };
        }

        if let Some(parent) = nodes.get_mut(id.value) {
            if parent.first_child.is_none() {
                parent.first_child = new_child_id_opt
            }
            parent.last_child = new_child_id_opt;
        }
    }

    /// Appends a child node by `new_child_id` to a node by `id`. `new_child_id` must exist in the tree.
    pub fn append_child_of(&self, id: &NodeId, new_child_id: &NodeId) {
        let mut nodes = self.nodes.borrow_mut();

        let Some(parent) = nodes.get_mut(id.value) else {
            // TODO: panic or not?
            return;
        };

        let last_child_id = parent.last_child;

        if last_child_id.is_none() {
            parent.first_child = Some(*new_child_id);
        }

        parent.last_child = Some(*new_child_id);

        if let Some(id) = last_child_id {
            if let Some(last_child) = nodes.get_mut(id.value) {
                last_child.next_sibling = Some(*new_child_id);
            }
        }

        {
            if let Some(child) = nodes.get_mut(new_child_id.value) {
                child.prev_sibling = last_child_id;
                child.parent = Some(*id);
            }
        }
    }

    /// Prepend a child node by `new_child_id` to a node by `id`. `new_child_id` must exist in the tree.
    pub fn prepend_child_of(&self, id: &NodeId, new_child_id: &NodeId) {
        let mut nodes = self.nodes.borrow_mut();

        let Some(parent) = nodes.get_mut(id.value) else {
            // TODO: panic or not?
            return;
        };
        let first_child_id = parent.first_child;

        if first_child_id.is_none() {
            parent.last_child = Some(*new_child_id);
        }

        parent.first_child = Some(*new_child_id);

        if let Some(id) = first_child_id {
            if let Some(first_child) = nodes.get_mut(id.value) {
                first_child.prev_sibling = Some(*new_child_id);
            }
        }

        {
            if let Some(child) = nodes.get_mut(new_child_id.value) {
                child.next_sibling = first_child_id;
                child.parent = Some(*id);
                child.prev_sibling = None;
            }
        }
    }

    /// Remove a node from the its parent by id. The node remains in the tree.
    /// It is possible to assign it to another node in the tree after this operation.
    pub fn remove_from_parent(&self, id: &NodeId) {
        let nodes = self.nodes.borrow_mut();
        TreeNodeHandler::remove_from_parent(id, nodes);
    }

    #[deprecated(since = "0.10.0", note = "please use `insert_before_of` instead")]
    /// Append a sibling node in the tree before the given node.
    pub fn append_prev_sibling_of(&self, id: &NodeId, new_sibling_id: &NodeId) {
        self.insert_before_of(id, new_sibling_id);
    }

    /// Append a sibling node in the tree before the given node.
    pub fn insert_before_of(&self, id: &NodeId, new_sibling_id: &NodeId) {
        self.remove_from_parent(new_sibling_id);
        let mut nodes = self.nodes.borrow_mut();
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let parent_id = node.parent;
        let prev_sibling_id = node.prev_sibling;

        node.prev_sibling = Some(*new_sibling_id);

        if let Some(new_sibling) = nodes.get_mut(new_sibling_id.value) {
            new_sibling.parent = parent_id;
            new_sibling.prev_sibling = prev_sibling_id;
            new_sibling.next_sibling = Some(*id);
        };

        if let Some(parent) = parent_id.and_then(|id| nodes.get_mut(id.value)) {
            if parent.first_child == Some(*id) {
                parent.first_child = Some(*new_sibling_id);
            }
        }

        if let Some(prev_sibling) = prev_sibling_id.and_then(|id| nodes.get_mut(id.value)) {
            prev_sibling.next_sibling = Some(*new_sibling_id);
        }
    }

    /// Append a sibling node in the tree after the given node.
    pub fn insert_after_of(&self, id: &NodeId, new_sibling_id: &NodeId) {
        self.remove_from_parent(new_sibling_id);
        let mut nodes = self.nodes.borrow_mut();
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let parent_id = node.parent;
        let next_sibling_id = node.next_sibling;

        node.next_sibling = Some(*new_sibling_id);

        if let Some(new_sibling) = nodes.get_mut(new_sibling_id.value) {
            new_sibling.parent = parent_id;
            new_sibling.prev_sibling = Some(*id);
            new_sibling.next_sibling = next_sibling_id;
        };

        if let Some(parent) = parent_id.and_then(|id| nodes.get_mut(id.value)) {
            if parent.last_child == Some(*id) {
                parent.last_child = Some(*new_sibling_id);
            }
        }

        if let Some(next_sibling) = next_sibling_id.and_then(|id| nodes.get_mut(id.value)) {
            next_sibling.prev_sibling = Some(*new_sibling_id);
        }
    }

    /// Changes the parent of children nodes of a node.
    pub fn reparent_children_of(&self, id: &NodeId, new_parent_id: Option<NodeId>) {
        let mut nodes = self.nodes.borrow_mut();

        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let first_child_id = node.first_child;
        let last_child_id = node.last_child;
        node.first_child = None;
        node.last_child = None;

        if let Some(new_parent_id) = new_parent_id {
            if let Some(new_parent) = nodes.get_mut(new_parent_id.value) {
                new_parent.first_child = first_child_id;
                new_parent.last_child = last_child_id;
            }
        }
        let mut next_child_id = first_child_id;
        while let Some(child_id) = next_child_id {
            if let Some(child) = nodes.get_mut(child_id.value) {
                child.parent = new_parent_id;
                next_child_id = child.next_sibling;
            }
        }
    }

    /// Detaches the children of a node.
    pub fn remove_children_of(&self, id: &NodeId) {
        self.reparent_children_of(id, None)
    }
}


impl Tree {
    pub fn text_of(&self, id: NodeId) -> StrTendril {
        let nodes = self.nodes.borrow();
        TreeNodeHandler::text_of(id, nodes)
    }
}

impl Tree {
    /// Adds nodes from another tree to the current tree
    pub(crate) fn merge(&self, other: Tree) {
        // `parse_fragment` returns a document that looks like:
        // <:root>                     id -> 0
        //  <body>                     id -> 1
        //      <html>                 id -> 2
        //          things we need.
        //      </html>
        //  </body>
        // <:root>
        let mut nodes = self.nodes.borrow_mut();

        let mut other_nodes = other.nodes.into_inner();

        let offset = nodes.len();
        let skip: usize = 3;
        let id_offset = offset - skip;

        for node in other_nodes.iter_mut().skip(skip) {
            fix_node(node, id_offset);
        }
        nodes.extend(other_nodes.into_iter().skip(skip));
    }

    /// Get the new id, that is not in the Tree.
    ///
    /// This function doesn't add a new id.
    /// it is just a convenient wrapper to get the new id.
    pub(crate) fn get_new_id(&self) -> NodeId {
        NodeId::new(self.nodes.borrow().len())
    }

    /// Adds nodes from another tree to the current tree and
    /// then applies a function to the first  merged node
    pub(crate) fn merge_with_fn<F>(&self, other: Tree, f: F)
    where
        F: FnOnce(NodeId),
    {
        let new_node_id = self.get_new_id();
        self.merge(other);
        f(new_node_id);
    }

    ///Adds a copy of the node and its children to the current tree
    ///
    /// # Arguments
    ///
    /// * `node` - reference to a node in the some tree
    ///
    /// # Returns
    ///
    /// * `NodeId` - id of the new node, that was added into the current tree
    pub(crate) fn copy_node(&self, node: &NodeRef) -> NodeId {
        let base_id = self.get_new_id();
        let mut next_id_val = base_id.value;

        let mut id_map: InnerHashMap<usize, usize> = InnerHashMap::default();
        id_map.insert(node.id.value, next_id_val);

        let mut ops = vec![node.clone()];

        while let Some(op) = ops.pop() {
            for child in op.children_it(false) {
                next_id_val += 1;
                id_map.insert(child.id.value, next_id_val);
            }

            ops.extend(op.children_it(true));
        }

        // source tree may be the same tree that owns the copy_node fn, and may be not.
        let source_tree = node.tree;
        let new_nodes = self.copy_tree_nodes(source_tree, &id_map);

        let mut nodes = self.nodes.borrow_mut();
        nodes.extend(new_nodes);

        base_id
    }

    fn copy_tree_nodes(
        &self,
        source_tree: &Tree,
        id_map: &InnerHashMap<usize, usize>,
    ) -> Vec<TreeNode> {
        let mut new_nodes: Vec<TreeNode> = vec![];
        let source_nodes = source_tree.nodes.borrow();
        let tree_nodes_it = id_map.iter().flat_map(|(old_id, new_id)| {
            source_nodes.get(*old_id).map(|sn| TreeNode {
                id: NodeId::new(*new_id),
                parent: sn
                    .parent
                    .and_then(|old_id| id_map.get(&old_id.value).map(|id| NodeId::new(*id))),
                prev_sibling: sn
                    .prev_sibling
                    .and_then(|old_id| id_map.get(&old_id.value).map(|id| NodeId::new(*id))),
                next_sibling: sn
                    .next_sibling
                    .and_then(|old_id| id_map.get(&old_id.value).map(|id| NodeId::new(*id))),
                first_child: sn
                    .first_child
                    .and_then(|old_id| id_map.get(&old_id.value).map(|id| NodeId::new(*id))),
                last_child: sn
                    .last_child
                    .and_then(|old_id| id_map.get(&old_id.value).map(|id| NodeId::new(*id))),
                data: sn.data.clone(),
            })
        });
        new_nodes.extend(tree_nodes_it);
        new_nodes.sort_by_key(|k| k.id.value);
        new_nodes
    }

    /// Copies nodes from another tree to the current tree and applies the given function
    /// to each copied node. The function is called with the ID of each copied node.
    ///
    /// # Arguments
    ///
    /// * `other_nodes` - slice of nodes to be copied
    /// * `f` - function to be applied to each copied node
    pub(crate) fn copy_nodes_with_fn<F>(&self, other_nodes: &[NodeRef], f: F)
    where
        F: Fn(NodeId),
    {
        for other_node in other_nodes {
            let new_node_id = self.copy_node(other_node);
            f(new_node_id);
        }
    }
}