use std::cell::{Ref, RefCell};
use std::fmt::{self, Debug};
use std::io;

use html5ever::serialize;
use html5ever::serialize::SerializeOpts;
use html5ever::LocalName;
use markup5ever::serialize::TraversalScope;
use markup5ever::serialize::TraversalScope::{ChildrenOnly, IncludeNode};
use markup5ever::serialize::{Serialize, Serializer};
use markup5ever::{namespace_url, ns, Attribute, QualName};
use tendril::StrTendril;

use crate::entities::{HashSetFx, NodeId, NodeIdMap};

/// Alias for `NodeRef`.
pub type Node<'a> = NodeRef<'a, NodeData>;

fn children_of<T>(nodes: &Ref<Vec<InnerNode<T>>>, id: &NodeId) -> Vec<NodeId> {
    let mut children = vec![];

    if let Some(node) = nodes.get(id.value) {
        let mut next_child_id = node.first_child;

        while let Some(node_id) = next_child_id {
            if let Some(node) = nodes.get(node_id.value) {
                next_child_id = node.next_sibling;
                children.push(node_id);
            }
        }
    }
    children
}

fn fix_id(id: Option<NodeId>, offset: usize) -> Option<NodeId> {
    id.map(|old| NodeId::new(old.value + offset))
}

fn contains_class(classes: &str, target_class: &str) -> bool {
    classes.split_whitespace().any(|c| c == target_class)
}

/// An implementation of arena-tree.
pub struct Tree<T> {
    nodes: RefCell<Vec<InnerNode<T>>>,
    names: NodeIdMap,
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
            names: self.names.clone(),
        }
    }
}

impl<T: Debug> Tree<T> {
    pub fn root_id(&self) -> NodeId {
        NodeId { value: 0 }
    }

    pub fn new(root: T) -> Self {
        let root_id = NodeId::new(0);
        Self {
            nodes: RefCell::new(vec![InnerNode::new(root_id, root)]),
            names: NodeIdMap::default(),
        }
    }

    pub fn create_node(&self, data: T) -> NodeId {
        let mut nodes = self.nodes.borrow_mut();
        let new_child_id = NodeId::new(nodes.len());

        nodes.push(InnerNode::new(new_child_id, data));
        new_child_id
    }

    pub fn set_name(&mut self, id: NodeId, name: QualName) {
        self.names.insert(id, name);
    }

    pub fn get_name(&self, id: &NodeId) -> &QualName {
        // TODO: what do we have here?
        self.names.get(id).unwrap()
    }

    pub fn get(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value).map(|_| NodeRef {
            id: *id,
            tree: self,
        });
        node
    }

    pub fn get_unchecked(&self, id: &NodeId) -> NodeRef<T> {
        NodeRef {
            id: *id,
            tree: self,
        }
    }

    pub fn root(&self) -> NodeRef<T> {
        self.get_unchecked(&NodeId::new(0))
    }

    pub fn children_of(&self, id: &NodeId) -> Vec<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        children_of(&nodes, id)
            .into_iter()
            .map(|id| NodeRef::new(id, self))
            .collect()
    }

    pub fn first_child_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.first_child.map(|id| NodeRef { id, tree: self })
    }

    pub fn last_child_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.last_child.map(|id| NodeRef { id, tree: self })
    }

    pub fn parent_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.parent.map(|id| NodeRef { id, tree: self })
    }

    pub fn prev_sibling_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.prev_sibling.map(|id| NodeRef { id, tree: self })
    }

    pub fn next_sibling_of(&self, id: &NodeId) -> Option<NodeRef<T>> {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        node.next_sibling.map(|id| NodeRef { id, tree: self })
    }

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

    pub fn append_children_from_another_tree(&self, id: &NodeId, tree: Tree<T>) {
        let mut nodes = self.nodes.borrow_mut();
        let mut new_nodes = tree.nodes.into_inner();
        assert!(
            !new_nodes.is_empty(),
            "The tree should have at least one root node"
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
                //???
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

            node.id = fix_id(node.id, offset);
            node.prev_sibling = fix_id(node.prev_sibling, offset);
            node.next_sibling = fix_id(node.next_sibling, offset);
            node.first_child = fix_id(node.first_child, offset);
            node.last_child = fix_id(node.last_child, offset);
        }

        // Put all the new nodes except the root node into the nodes.
        nodes.extend(new_nodes);
    }

    pub fn append_prev_siblings_from_another_tree(&self, id: &NodeId, tree: Tree<T>) {
        let mut nodes = self.nodes.borrow_mut();

        let mut new_nodes = tree.nodes.into_inner();
        assert!(
            !new_nodes.is_empty(),
            "The tree should have at least one root node"
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

            node.id = fix_id(node.id, offset);
            node.first_child = fix_id(node.first_child, offset);
            node.last_child = fix_id(node.last_child, offset);
            node.prev_sibling = fix_id(node.prev_sibling, offset);
            node.next_sibling = fix_id(node.next_sibling, offset);
        }

        // Update last child's next_sibling.
        new_nodes[last_valid_child].next_sibling = Some(*id);

        // Put all the new nodes except the root node into the nodes.
        nodes.extend(new_nodes);
    }

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

    pub fn remove_children_of(&self, id: &NodeId) {
        self.reparent_children_of(id, None)
    }

    pub fn query_node<F, B>(&self, id: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&InnerNode<T>) -> B,
    {
        let nodes = self.nodes.borrow();
        let node = nodes.get(id.value)?;
        let r = f(node);
        Some(r)
    }

    pub fn update_node<F, B>(&self, id: &NodeId, f: F) -> Option<B>
    where
        F: FnOnce(&mut InnerNode<T>) -> B,
    {
        let mut nodes = self.nodes.borrow_mut();
        let node = nodes.get_mut(id.value)?;
        let r = f(node);
        Some(r)
    }

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

pub struct InnerNode<T> {
    pub id: Option<NodeId>,
    pub parent: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub data: T,
}

impl<T> InnerNode<T> {
    fn new(id: NodeId, data: T) -> Self {
        InnerNode {
            id: Some(id),
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            data,
        }
    }
}

impl<T: Debug> Debug for InnerNode<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Node")
            .field("id", &self.id)
            .field("parent", &self.parent)
            .field("prev_sibling", &self.prev_sibling)
            .field("next_sibling", &self.next_sibling)
            .field("first_child", &self.first_child)
            .field("last_child", &self.last_child)
            .field("data", &self.data)
            .finish()
    }
}

impl InnerNode<NodeData> {
    pub fn is_document(&self) -> bool {
        matches!(self.data, NodeData::Document)
    }

    pub fn is_element(&self) -> bool {
        matches!(self.data, NodeData::Element(_))
    }

    pub fn is_text(&self) -> bool {
        matches!(self.data, NodeData::Text { .. })
    }

    pub fn is_comment(&self) -> bool {
        matches!(self.data, NodeData::Comment { .. })
    }
}

impl<T: Clone> Clone for InnerNode<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            parent: self.parent,
            prev_sibling: self.prev_sibling,
            next_sibling: self.next_sibling,
            first_child: self.first_child,
            last_child: self.last_child,
            data: self.data.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeRef<'a, T> {
    pub id: NodeId,
    pub tree: &'a Tree<T>,
}

impl<'a, T: Debug> NodeRef<'a, T> {
    pub fn new(id: NodeId, tree: &'a Tree<T>) -> Self {
        Self { id, tree }
    }

    #[inline]
    pub fn query<F, B>(&self, f: F) -> Option<B>
    where
        F: FnOnce(&InnerNode<T>) -> B,
    {
        self.tree.query_node(&self.id, f)
    }
    #[inline]
    pub fn update<F, B>(&self, f: F) -> Option<B>
    where
        F: FnOnce(&mut InnerNode<T>) -> B,
    {
        self.tree.update_node(&self.id, f)
    }
    #[inline]
    pub fn parent(&self) -> Option<Self> {
        self.tree.parent_of(&self.id)
    }
    #[inline]
    pub fn children(&self) -> Vec<Self> {
        self.tree.children_of(&self.id)
    }
    #[inline]
    pub fn first_child(&self) -> Option<Self> {
        self.tree.first_child_of(&self.id)
    }
    #[inline]
    pub fn next_sibling(&self) -> Option<Self> {
        self.tree.next_sibling_of(&self.id)
    }
    #[inline]
    pub fn remove_from_parent(&self) {
        self.tree.remove_from_parent(&self.id)
    }
    #[inline]
    pub fn remove_children(&self) {
        self.tree.remove_children_of(&self.id)
    }
    #[inline]
    pub fn append_prev_sibling(&self, id: &NodeId) {
        self.tree.append_prev_sibling_of(&self.id, id)
    }
    #[inline]
    pub fn append_child(&self, id: &NodeId) {
        self.tree.append_child_of(&self.id, id)
    }
    #[inline]
    pub fn append_children_from_another_tree(&self, tree: Tree<T>) {
        self.tree.append_children_from_another_tree(&self.id, tree)
    }
    #[inline]
    pub fn append_prev_siblings_from_another_tree(&self, tree: Tree<T>) {
        self.tree
            .append_prev_siblings_from_another_tree(&self.id, tree)
    }
}

impl<'a> Node<'a> {
    pub fn next_element_sibling(&self) -> Option<Node<'a>> {
        let nodes = self.tree.nodes.borrow();
        let mut node = nodes.get(self.id.value)?;

        let r = loop {
            if let Some(id) = node.next_sibling {
                node = nodes.get(id.value)?;
                if node.is_element() {
                    break Some(NodeRef::new(id, self.tree));
                }
            } else {
                break None;
            }
        };
        r
    }

    pub fn prev_element_sibling(&self) -> Option<Node<'a>> {
        let nodes = self.tree.nodes.borrow();
        let mut node = nodes.get(self.id.value)?;

        let r = loop {
            if let Some(id) = node.prev_sibling {
                node = nodes.get(id.value)?;
                if node.is_element() {
                    break Some(NodeRef::new(id, self.tree));
                }
            } else {
                break None;
            }
        };
        r
    }
}

impl<'a> Node<'a> {
    pub fn node_name(&self) -> Option<StrTendril> {
        self.query(|node| match node.data {
            NodeData::Element(ref e) => {
                let name: &str = &e.name.local;
                Some(StrTendril::from(name))
            }
            _ => None,
        })?
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.query(|node| match node.data {
            NodeData::Element(ref e) => e
                .attrs
                .iter()
                .find(|attr| &attr.name.local == "class")
                .map(|attr| contains_class(&attr.value, class))
                .unwrap_or(false),
            _ => false,
        })
        .unwrap_or(false)
    }

    pub fn add_class(&self, class: &str) {
        if class.trim().is_empty() {
            return;
        }

        self.update(|node| {
            if let NodeData::Element(ref mut e) = node.data {
                let mut attr = e.attrs.iter_mut().find(|attr| &attr.name.local == "class");

                let set: HashSetFx<&str> = class
                    .split(' ')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                if attr.is_some() {
                    let value = &mut attr.as_mut().unwrap().value;
                    for v in set {
                        if !contains_class(value, v) {
                            value.push_slice(" ");
                            value.push_slice(v);
                        }
                    }
                } else {
                    let classes: Vec<&str> = set.into_iter().collect();
                    let value = StrTendril::from(classes.join(" "));
                    // The namespace on the attribute name is almost always ns!().
                    let name = QualName::new(None, ns!(), LocalName::from("class"));

                    e.attrs.push(Attribute { name, value })
                }
            }
        });
    }

    pub fn remove_class(&self, class: &str) {
        if class.trim().is_empty() {
            return;
        }

        self.update(|node| {
            if let NodeData::Element(ref mut e) = node.data {
                if let Some(attr) = e.attrs.iter_mut().find(|attr| &attr.name.local == "class") {
                    let mut set: HashSetFx<&str> = attr
                        .value
                        .split(' ')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let removes = class.split(' ').map(|s| s.trim()).filter(|s| !s.is_empty());

                    for remove in removes {
                        set.remove(remove);
                    }

                    attr.value = StrTendril::from(set.into_iter().collect::<Vec<&str>>().join(" "));
                }
            }
        });
    }

    pub fn attr(&self, name: &str) -> Option<StrTendril> {
        self.query(|node| match node.data {
            NodeData::Element(ref e) => e
                .attrs
                .iter()
                .find(|attr| &attr.name.local == name)
                .map(|attr| attr.value.clone()),
            _ => None,
        })?
    }

    pub fn attrs(&self) -> Vec<Attribute> {
        self.query(|node| match node.data {
            NodeData::Element(ref e) => e.attrs.to_vec(),
            _ => vec![],
        })
        .unwrap_or_default()
    }

    pub fn set_attr(&self, name: &str, val: &str) {
        self.update(|node| {
            if let NodeData::Element(ref mut e) = node.data {
                let updated = e.attrs.iter_mut().any(|attr| {
                    if &attr.name.local == name {
                        attr.value = StrTendril::from(val);
                        true
                    } else {
                        false
                    }
                });

                if !updated {
                    let value = StrTendril::from(val);
                    // The namespace on the attribute name is almost always ns!().
                    let name = QualName::new(None, ns!(), LocalName::from(name));

                    e.attrs.push(Attribute { name, value })
                }
            }
        });
    }

    pub fn remove_attr(&self, name: &str) {
        self.update(|node| {
            if let NodeData::Element(ref mut e) = node.data {
                e.attrs.retain(|attr| &attr.name.local != name);
            }
        });
    }
}

impl<'a> Node<'a> {
    pub fn is_document(&self) -> bool {
        self.query(|node| node.is_document()).unwrap_or(false)
    }

    pub fn is_element(&self) -> bool {
        self.query(|node| node.is_element()).unwrap_or(false)
    }

    pub fn is_text(&self) -> bool {
        self.query(|node| node.is_text()).unwrap_or(false)
    }
    pub fn is_comment(&self) -> bool {
        self.query(|node| node.is_comment()).unwrap_or(false)
    }
}

impl<'a> Node<'a> {
    /// Returns the HTML representation of the DOM tree.
    /// Panics if serialization fails.
    pub fn html(&self) -> StrTendril {
        let inner: SerializableNodeRef = self.clone().into();

        let mut result = vec![];
        serialize(
            &mut result,
            &inner,
            SerializeOpts {
                scripting_enabled: true,
                traversal_scope: TraversalScope::IncludeNode,
                create_missing_parent: false,
            },
        )
        .unwrap();
        StrTendril::try_from_byte_slice(&result).unwrap()
    }

    pub fn text(&self) -> StrTendril {
        let mut ops = vec![self.id];
        let mut text = StrTendril::new();
        let nodes = self.tree.nodes.borrow();
        while !ops.is_empty() {
            let id = ops.remove(0);
            if let Some(node) = nodes.get(id.value) {
                match node.data {
                    NodeData::Element(_) => {
                        for child in children_of(&nodes, &id).into_iter().rev() {
                            ops.insert(0, child);
                        }
                    }

                    NodeData::Text { ref contents } => text.push_tendril(contents),

                    _ => continue,
                }
            }
        }
        text
    }
    pub fn has_text(&self, needle: &str) -> bool {
        let mut ops = vec![self.id];
        let nodes = self.tree.nodes.borrow();
        while !ops.is_empty() {
            let id = ops.remove(0);
            if let Some(node) = nodes.get(id.value) {
                match node.data {
                    NodeData::Element(_) => {
                        for child in children_of(&nodes, &id).into_iter().rev() {
                            ops.insert(0, child);
                        }
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
}

/// The different kinds of nodes in the DOM.
#[derive(Debug, Clone)]
pub enum NodeData {
    /// The `Tree` itself - the root node of a HTML tree.
    Document,

    /// A `DOCTYPE` with name, public id, and system id. See
    /// [tree type declaration on wikipedia][dtd wiki].
    ///
    /// [dtd wiki]: https://en.wikipedia.org/wiki/Tree_type_declaration
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },

    /// A text node.
    Text { contents: StrTendril },

    /// A comment.
    Comment { contents: StrTendril },

    /// An element with attributes.
    Element(Element),

    /// A Processing instruction.
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

/// An element with attributes.
#[derive(Debug, Clone)]
pub struct Element {
    pub name: QualName,
    pub attrs: Vec<Attribute>,

    /// For HTML \<template\> elements, the [template contents].
    ///
    /// [template contents]: https://html.spec.whatwg.org/multipage/#template-contents
    pub template_contents: Option<NodeId>,

    /// Whether the node is a [HTML integration point].
    ///
    /// [HTML integration point]: https://html.spec.whatwg.org/multipage/#html-integration-point
    #[allow(dead_code)]
    mathml_annotation_xml_integration_point: bool,
}

impl Element {
    pub fn new(
        name: QualName,
        attrs: Vec<Attribute>,
        template_contents: Option<NodeId>,
        mathml_annotation_xml_integration_point: bool,
    ) -> Element {
        Element {
            name,
            attrs,
            template_contents,
            mathml_annotation_xml_integration_point,
        }
    }
}

enum SerializeOp {
    Open(NodeId),
    Close(QualName),
}
/// Serializable wrapper of Node.
pub struct SerializableNodeRef<'a>(Node<'a>);

impl<'a> From<NodeRef<'a, NodeData>> for SerializableNodeRef<'a> {
    fn from(h: NodeRef<'a, NodeData>) -> SerializableNodeRef {
        SerializableNodeRef(h)
    }
}

impl<'a> Serialize for SerializableNodeRef<'a> {
    fn serialize<S>(&self, serializer: &mut S, traversal_scope: TraversalScope) -> io::Result<()>
    where
        S: Serializer,
    {
        let nodes = self.0.tree.nodes.borrow();
        let id = self.0.id;
        let mut ops = match traversal_scope {
            IncludeNode => vec![SerializeOp::Open(id)],
            ChildrenOnly(_) => children_of(&nodes, &id)
                .into_iter()
                .map(SerializeOp::Open)
                .collect(),
        };

        while !ops.is_empty() {
            match ops.remove(0) {
                SerializeOp::Open(id) => {
                    let node_opt = &nodes.get(id.value);
                    let node = match node_opt {
                        Some(node) => node,
                        None => continue,
                    };

                    match node.data {
                        NodeData::Element(ref e) => {
                            serializer.start_elem(
                                e.name.clone(),
                                e.attrs.iter().map(|at| (&at.name, &at.value[..])),
                            )?;

                            ops.insert(0, SerializeOp::Close(e.name.clone()));

                            for child_id in children_of(&nodes, &id).into_iter().rev() {
                                ops.insert(0, SerializeOp::Open(child_id));
                            }

                            Ok(())
                        }
                        NodeData::Doctype { ref name, .. } => serializer.write_doctype(name),
                        NodeData::Text { ref contents } => serializer.write_text(contents),
                        NodeData::Comment { ref contents } => serializer.write_comment(contents),
                        NodeData::ProcessingInstruction {
                            ref target,
                            ref contents,
                        } => serializer.write_processing_instruction(target, contents),
                        NodeData::Document => {
                            for child_id in children_of(&nodes, &id).into_iter().rev() {
                                ops.insert(0, SerializeOp::Open(child_id));
                            }
                            continue;
                        }
                    }
                }
                SerializeOp::Close(name) => serializer.end_elem(name),
            }?
        }

        Ok(())
    }
}
