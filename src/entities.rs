use markup5ever::QualName;

/// Node ID.
#[derive(Copy, Debug, Clone, Eq, PartialEq, Hash)]
pub struct NodeId {
    pub(crate) value: usize,
}

impl NodeId {
    pub(crate) fn new(value: usize) -> Self {
        NodeId { value }
    }
}

/// Alias for `FxHashMap<NodeId, QualName>`
#[cfg(feature = "hashbrown")]
mod inline {
    use hashbrown::{HashMap, HashSet};
    use rustc_hash::FxHasher;
    use std::hash::BuildHasherDefault;
    pub type NodeIdMap = HashMap<super::NodeId, super::QualName, BuildHasherDefault<FxHasher>>;
    pub type NodeIdSet = HashSet<super::NodeId, BuildHasherDefault<FxHasher>>;
    pub type HashSetFx<K> = HashSet<K, BuildHasherDefault<FxHasher>>;
}

#[cfg(not(feature = "hashbrown"))]
mod inline {
    use rustc_hash::{FxHashMap, FxHashSet};
    pub type NodeIdMap = FxHashMap<super::NodeId, super::QualName>;
    pub type NodeIdSet = FxHashSet<super::NodeId>;
    pub type HashSetFx<K> = FxHashSet<K>;
}

pub(crate) use inline::{HashSetFx, NodeIdMap, NodeIdSet};
