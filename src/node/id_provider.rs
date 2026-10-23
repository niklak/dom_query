use crate::Tree;

use super::{NodeId, node_ref::NodeRef};

/// A trait that provides the [`NodeId`]
pub trait NodeIdProver {
    /// Returns the [`NodeId`]
    fn node_id(&self) -> &NodeId;

    /// Returns the tree that owns the [`NodeId`], when the provider knows it.
    fn tree(&self) -> Option<&Tree> {
        None
    }
}

impl NodeIdProver for &NodeRef<'_> {
    fn node_id(&self) -> &NodeId {
        &self.id
    }

    fn tree(&self) -> Option<&Tree> {
        Some(self.tree)
    }
}

impl NodeIdProver for &NodeId {
    fn node_id(&self) -> Self {
        self
    }
}
