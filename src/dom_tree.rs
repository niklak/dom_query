use std::cell::{Ref, RefCell};
use std::fmt::{self, Debug};

use html5ever::LocalName;
use html5ever::{namespace_url, ns, QualName};

use crate::node::{ancestor_nodes, child_nodes, AncestorNodes, ChildNodes};
use crate::node::{Element, InnerNode, Node, NodeData, NodeId, NodeRef};

fn fix_id(id: Option<NodeId>, offset: usize) -> Option<NodeId> {
    id.map(|old| NodeId::new(old.value + offset))
}

/// fixes node ids
fn fix_node<T: Debug>(n: &mut InnerNode<T>, offset: usize) {
    n.id = n.id.map(|id| NodeId::new(id.value + offset));
    n.prev_sibling = n.prev_sibling.map(|id| NodeId::new(id.value + offset));
    n.next_sibling = n.next_sibling.map(|id| NodeId::new(id.value + offset));
    n.first_child = n.first_child.map(|id| NodeId::new(id.value + offset));
    n.last_child = n.last_child.map(|id| NodeId::new(id.value + offset));
}

/// An implementation of arena-tree.
pub struct Tree<T> {
    pub(crate) nodes: RefCell<Vec<InnerNode<T>>>,
}

impl<T: Debug> Debug for Tree<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Tree").finish()
    }
}

impl<T: Clone> Clone for Tree<T> {
    fn clone(&self) -> Self {
        let nodes = self.nodes.borrow();
        Self {
            nodes: RefCell::new(nodes.clone()),
        }
    }
}

impl Tree<NodeData> {
    /// Creates a new element with the given name.
    pub fn new_element(&self, name: &str) -> Node {
        let name = QualName::new(None, ns!(), LocalName::from(name));
        let el = Element::new(name.clone(), Vec::new(), None, false);

        let id = self.create_node(NodeData::Element(el));

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

impl<T: Debug> Tree<T> {
    /// Returns the root node.
    pub fn root_id(&self) -> NodeId {
        NodeId { value: 0 }
    }

    /// Creates a new tree with the given root.
    /// `T` is [`NodeData`].
    pub fn new(root: T) -> Self {
        let root_id = NodeId::new(0);
        Self {
            nodes: RefCell::new(vec![InnerNode::new(root_id, root)]),
        }
    }
    /// Creates a new node with the given data.
    pub fn create_node(&self, data: T) -> NodeId {
        let mut nodes = self.nodes.borrow_mut();
        let new_child_id = NodeId::new(nodes.len());

        nodes.push(InnerNode::new(new_child_id, data));
        new_child_id
    }

    /// Gets node by id
    pub fn get(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value).map(|_| NodeRef {
            id: *id,
            tree: self,
        });
        node
    }

    /// Gets node by id
    pub fn get_unchecked(&self, id: &NodeId) -> NodeRef<T> {
        NodeRef {
            id: *id,
            tree: self,
        }
    }

    /// Gets the root node
    pub fn root(&self) -> NodeRef<T> {
        self.get_unchecked(&NodeId::new(0))
    }

    /// Gets the ancestors nodes of a node by id.
    ///
    /// # Arguments
    /// * `id` - The id of the node.
    /// * `max_depth` - The maximum depth of the ancestors. If `None`, or Some(0) the maximum depth is unlimited.
    ///
    /// # Returns
    /// `Vec<NodeRef<T>>` A vector of ancestors nodes.
    pub fn ancestors_of(&self, id: &NodeId, max_depth: Option<usize>) -> Vec<NodeRef<T>> {
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
    pub fn ancestor_ids_of_it(
        &self,
        id: &NodeId,
        max_depth: Option<usize>,
    ) -> AncestorNodes<'_, T> {
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
    pub fn children_of(&self, id: &NodeId) -> Vec<NodeRef<T>> {
        child_nodes(self.nodes.borrow(), id)
            .map(move |id| NodeRef::new(id, self))
            .collect()
    }

    /// Returns an iterator of the child node ids of a node by id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    pub fn child_ids_of_it(&self, id: &NodeId) -> ChildNodes<'_, T> {
        child_nodes(self.nodes.borrow(), id)
    }

    /// Returns a vector of the child node ids of a node by id
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the node.
    pub fn child_ids_of(&self, id: &NodeId) -> Vec<NodeId> {
        child_nodes(self.nodes.borrow(), id).collect()
    }

    /// Gets the first child node of a node by id
    pub fn first_child_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.first_child.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the last child node of a node by id
    pub fn last_child_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.last_child.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the parent node of a node by id
    pub fn parent_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.parent.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the previous sibling node of a node by id
    pub fn prev_sibling_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.prev_sibling.map(|id| NodeRef { id, tree: self })
    }

    /// Gets the next sibling node of a node by id
    pub fn next_sibling_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.next_sibling.map(|id| NodeRef { id, tree: self })
    }

    /// Creates a new element from data  and appends it to a node by id
    pub fn append_child_data_of(&self, id: &NodeId, data: T) {
        let mut nodes = self.nodes.borrow_mut();

        let last_child_id = nodes.get(id.value).and_then(|node| node.last_child);

        let new_child_id = NodeId::new(nodes.len());
        let mut child = InnerNode::new(new_child_id, data);
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
        let last_child_id = nodes.get_mut(id.value).and_then(|node| node.last_child);

        if let Some(id) = last_child_id {
            if let Some(last_child) = nodes.get_mut(id.value) {
                last_child.next_sibling = Some(*new_child_id);
            }
        }

        if let Some(parent) = nodes.get_mut(id.value) {
            if last_child_id.is_none() {
                parent.first_child = Some(*new_child_id);
            }

            parent.last_child = Some(*new_child_id);

            if let Some(child) = nodes.get_mut(new_child_id.value) {
                child.prev_sibling = last_child_id;
                child.parent = Some(*id);
            }
        }
    }

    /// Appends children nodes from another tree. Another tree is a tree from document fragment.
    pub fn append_children_from_another_tree(&self, id: &NodeId, tree: Tree<T>) {
        let mut nodes = self.nodes.borrow_mut();
        let mut new_nodes = tree.nodes.into_inner();
        assert!(
            !new_nodes.is_empty(),
            "Another tree should have at least one root node"
        );
        assert!(
            !nodes.is_empty(),
            "The tree should have at least one root node"
        );

        let offset = nodes.len();

        // `parse_fragment` returns a document that looks like:
        // <:root>                     id -> 0
        //  <body>                     id -> 1
        //      <html>                 id -> 2
        //          things we need.
        //      </html>
        //  </body>
        // <:root>
        const TRUE_ROOT_ID: usize = 2;
        let node_root_id = NodeId::new(TRUE_ROOT_ID);
        let root = match new_nodes.get(node_root_id.value) {
            Some(node) => node,
            None => return,
        };

        let first_child_id = fix_id(root.first_child, offset);
        let last_child_id = fix_id(root.last_child, offset);

        // Update new parent's first and last child id.

        let parent = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        if parent.first_child.is_none() {
            parent.first_child = first_child_id;
        }

        let parent_last_child_id = parent.last_child;
        parent.last_child = last_child_id;

        // Update next_sibling_id
        if let Some(last_child_id) = parent_last_child_id {
            if let Some(last_child) = nodes.get_mut(last_child_id.value) {
                last_child.next_sibling = first_child_id;
            }
        }

        let mut first_valid_child = false;

        // Fix nodes's ref id.
        for node in new_nodes.iter_mut() {
            node.parent = node.parent.and_then(|parent_id| match parent_id.value {
                i if i < TRUE_ROOT_ID => None,
                i if i == TRUE_ROOT_ID => Some(*id),
                i => fix_id(Some(NodeId::new(i)), offset),
            });

            // Update prev_sibling_id
            if !first_valid_child && node.parent == Some(*id) {
                first_valid_child = true;

                node.prev_sibling = parent_last_child_id;
            }

            fix_node(node, offset);
        }

        // Put all the new nodes except the root node into the nodes.
        nodes.extend(new_nodes);
    }

    pub fn append_prev_siblings_from_another_tree(&self, id: &NodeId, tree: Tree<T>) {
        let mut nodes = self.nodes.borrow_mut();
        let mut new_nodes = tree.nodes.into_inner();
        assert!(
            !new_nodes.is_empty(),
            "Another tree should have at least one root node"
        );
        assert!(
            !nodes.is_empty(),
            "The tree should have at least one root node"
        );

        let offset = nodes.len();

        // `parse_fragment` returns a document that looks like:
        // <:root>                     id -> 0
        //  <body>                     id -> 1
        //      <html>                 id -> 2
        //          things we need.
        //      </html>
        //  </body>
        // <:root>
        const TRUE_ROOT_ID: usize = 2;
        let node_root_id = NodeId::new(TRUE_ROOT_ID);
        let root = match new_nodes.get(node_root_id.value) {
            Some(node) => node,
            None => return,
        };

        let first_child_id = fix_id(root.first_child, offset);
        let last_child_id = fix_id(root.last_child, offset);

        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };

        let prev_sibling_id = node.prev_sibling;
        let parent_id = node.parent;

        // Update node's previous sibling.
        node.prev_sibling = last_child_id;

        // Update prev sibling's next sibling
        if let Some(prev_sibling_id) = prev_sibling_id {
            if let Some(prev_sibling) = nodes.get_mut(prev_sibling_id.value) {
                prev_sibling.next_sibling = first_child_id;
            }

        // Update parent's first child.
        } else if let Some(parent_id) = parent_id {
            if let Some(parent) = nodes.get_mut(parent_id.value) {
                parent.first_child = first_child_id;
            }
        }

        let mut last_valid_child = 0;
        let mut first_valid_child = true;
        // Fix nodes's ref id.
        for (i, node) in new_nodes.iter_mut().enumerate() {
            node.parent = node
                .parent
                .and_then(|old_parent_id| match old_parent_id.value {
                    i if i < TRUE_ROOT_ID => None,
                    i if i == TRUE_ROOT_ID => parent_id,
                    i => fix_id(Some(NodeId::new(i)), offset),
                });

            // Update first child's prev_sibling
            if !first_valid_child && node.parent == Some(*id) {
                first_valid_child = true;
                node.prev_sibling = prev_sibling_id;
            }

            if node.parent == parent_id {
                last_valid_child = i;
            }

            fix_node(node, offset);
        }

        // Update last child's next_sibling.
        new_nodes[last_valid_child].next_sibling = Some(*id);

        // Put all the new nodes except the root node into the nodes.
        nodes.extend(new_nodes);
    }

    /// Remove a node from the its parent by id. The node remains in the tree.
    /// It is possible to assign it to another node in the tree after this operation.
    pub fn remove_from_parent(&self, id: &NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        let node = match nodes.get_mut(id.value) {
            Some(node) => node,
            None => return,
        };
        let parent_id = node.parent;
        let prev_sibling_id = node.prev_sibling;
        let next_sibling_id = node.next_sibling;

        node.parent = None;
        node.next_sibling = None;
        node.prev_sibling = None;

        if let Some(parent_id) = parent_id {
            if let Some(parent) = nodes.get_mut(parent_id.value) {
                if parent.first_child == Some(*id) {
                    parent.first_child = next_sibling_id;
                }

                if parent.last_child == Some(*id) {
                    parent.last_child = prev_sibling_id;
                }
            }
        }

        if let Some(prev_sibling_id) = prev_sibling_id {
            if let Some(prev_sibling) = nodes.get_mut(prev_sibling_id.value) {
                prev_sibling.next_sibling = next_sibling_id;
            }
        }

        if let Some(next_sibling_id) = next_sibling_id {
            if let Some(next_sibling) = nodes.get_mut(next_sibling_id.value) {
                next_sibling.prev_sibling = prev_sibling_id;
            };
        }
    }

    /// Append a sibling node in the tree before the given node.
    pub fn append_prev_sibling_of(&self, id: &NodeId, new_sibling_id: &NodeId) {
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

        if let Some(parent_id) = parent_id {
            if let Some(parent) = nodes.get_mut(parent_id.value) {
                if parent.first_child == Some(*id) {
                    parent.first_child = Some(*new_sibling_id);
                }
            };
        }

        if let Some(prev_sibling_id) = prev_sibling_id {
            if let Some(prev_sibling) = nodes.get_mut(prev_sibling_id.value) {
                prev_sibling.next_sibling = Some(*new_sibling_id);
            };
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

    /// A helper function to get the node from the tree and apply a function to it.
    pub fn query_node<F, B>(&self, id: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&InnerNode<T>) -> B,
    {
        let nodes = self.nodes.borrow();
        nodes.get(id.value).map(f)
    }

    /// A helper function to get the node from the tree and apply a function to it.
    /// Accepts a default value to return for a case if the node doesn't exist.
    pub fn query_node_or<F, B>(&self, id: &NodeId, default: B, f: F) -> B
    where
        F: FnOnce(&InnerNode<T>) -> B,
    {
        let nodes = self.nodes.borrow();
        nodes.get(id.value).map_or(default, f)
    }

    /// A helper function to get the node from the tree and apply a function to it that modifies it.
    pub fn update_node<F, B>(&self, id: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&mut InnerNode<T>) -> B,
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
        F: FnOnce(&InnerNode<T>, &InnerNode<T>) -> B,
    {
        let nodes = self.nodes.borrow();
        let node_a = nodes.get(a.value)?;
        let node_b = nodes.get(b.value)?;

        Some(f(node_a, node_b))
    }
}
