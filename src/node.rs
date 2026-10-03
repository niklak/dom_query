mod id_provider;
mod inner;
mod iters;
mod node_data;
mod node_ref;
mod selector;
mod serializing;
mod text_formatting;

use std::fmt::Debug;
use std::num::NonZeroU32;

pub use id_provider::NodeIdProver;
pub use inner::TreeNode;
pub use iters::{
    AncestorNodes, ChildNodes, DescendantNodes, ancestor_nodes, child_nodes, descendant_nodes,
};
pub use node_data::{Element, NodeData};
pub use node_ref::{Node, NodeRef};
pub use serializing::SerializableNodeRef;
pub(crate) use serializing::SerializeOp;
pub(crate) use text_formatting::format_text;

/// Represents a Node ID.
///
/// A tree holds at most `u32::MAX - 1` nodes.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NodeId {
    /// Index + 1, so `Option<NodeId>` fits in 4 bytes via the niche.
    raw: NonZeroU32,
}

impl NodeId {
    // The assert bounds `value`, so the cast cannot truncate.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) const fn new(value: usize) -> Self {
        assert!(value < u32::MAX as usize, "dom_query: too many nodes");
        match NonZeroU32::new(value as u32 + 1) {
            Some(raw) => Self { raw },
            None => unreachable!(),
        }
    }

    /// Returns the node's index in the tree's node array.
    #[inline]
    pub(crate) const fn value(self) -> usize {
        self.raw.get() as usize - 1
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeId")
            .field("value", &self.value())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_round_trips_and_keeps_order() {
        for value in [0, 1, 2, 1_000, u32::MAX as usize - 1] {
            assert_eq!(NodeId::new(value).value(), value);
        }
        assert!(NodeId::new(0) < NodeId::new(1));
    }

    #[test]
    fn node_id_debug_shows_the_index() {
        assert_eq!(format!("{:?}", NodeId::new(3)), "NodeId { value: 3 }");
    }

    #[test]
    #[should_panic(expected = "too many nodes")]
    fn node_id_rejects_index_past_u32_range() {
        let _ = NodeId::new(u32::MAX as usize);
    }

    #[test]
    fn option_node_id_uses_the_niche() {
        assert_eq!(size_of::<NodeId>(), 4);
        assert_eq!(size_of::<Option<NodeId>>(), 4);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(size_of::<TreeNode>(), 80);
    }
}
