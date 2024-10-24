use std::cell::Ref;

use super::inner::InnerNode;
use super::NodeId;

pub struct ChildrenIterator<'a, 'b, T> {
    nodes: &'a Ref<'b, Vec<InnerNode<T>>>,
    next_child_id: Option<NodeId>,
}

impl<'a, 'b, T> ChildrenIterator<'a, 'b, T> {
    pub fn new(nodes: &'a Ref<'b, Vec<InnerNode<T>>>, node_id: &NodeId) -> Self {
        let first_child = nodes
            .get(node_id.value)
            .map_or(None,|node| node.first_child);

        ChildrenIterator {
            nodes,
            next_child_id: first_child,
        }
    }
}

impl<'a, 'b, T> Iterator for ChildrenIterator<'a, 'b, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current_id = self.next_child_id?;
        
        if let Some(node) = self.nodes.get(current_id.value) {
            self.next_child_id = node.next_sibling;
            Some(current_id)
        } else {
            None
        }
    }
}
