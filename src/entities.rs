
/// Alias for `FxHashMap<NodeId, QualName>`
#[cfg(feature = "hashbrown")]
mod inline {
    use hashbrown::HashSet;
    use rustc_hash::FxHasher;
    use std::hash::BuildHasherDefault;
    pub type NodeIdSet = HashSet<crate::NodeId, BuildHasherDefault<FxHasher>>;
    pub type HashSetFx<K> = HashSet<K, BuildHasherDefault<FxHasher>>;
}

#[cfg(not(feature = "hashbrown"))]
mod inline {
    use rustc_hash::FxHashSet;
    pub type NodeIdSet = FxHashSet<crate::NodeId>;
    pub type HashSetFx<K> = FxHashSet<K>;
}

pub(crate) use inline::{HashSetFx, NodeIdSet};
