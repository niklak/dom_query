#[cfg(feature = "hashbrown")]
mod inline {
    use hashbrown::{HashMap, HashSet};
    pub type NodeIdSet = HashSet<crate::NodeId>;
    pub type HashSetFx<K> = HashSet<K>;
    pub type HashMapFx<K, V> = HashMap<K, V>;
}

#[cfg(not(feature = "hashbrown"))]
mod inline {
    use foldhash::{HashMap, HashSet};
    pub type NodeIdSet = HashSet<crate::NodeId>;
    pub type HashSetFx<K> = HashSet<K>;
    pub type HashMapFx<K, V> = HashMap<K, V>;
}

pub(crate) use inline::{HashMapFx, HashSetFx, NodeIdSet};
