use html5ever::QualName;

/// Alias for `FxHashMap<NodeId, QualName>`
#[cfg(feature = "hashbrown")]
mod inline {
    use hashbrown::{HashMap, HashSet};
    use rustc_hash::FxHasher;
    use std::hash::BuildHasherDefault;
    pub type NodeIdMap = HashMap<crate::NodeId, super::QualName, BuildHasherDefault<FxHasher>>;
    pub type NodeIdSet = HashSet<crate::NodeId, BuildHasherDefault<FxHasher>>;
    pub type HashSetFx<K> = HashSet<K, BuildHasherDefault<FxHasher>>;
}

#[cfg(not(feature = "hashbrown"))]
mod inline {
    use rustc_hash::{FxHashMap, FxHashSet};
    pub type NodeIdMap = FxHashMap<crate::NodeId, super::QualName>;
    pub type NodeIdSet = FxHashSet<crate::NodeId>;
    pub type HashSetFx<K> = FxHashSet<K>;
}

pub(crate) use inline::{HashSetFx, NodeIdMap, NodeIdSet};
