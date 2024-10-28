#[cfg(feature = "hashbrown")]
mod inline {
    use hashbrown::HashSet;
    pub type NodeIdSet = HashSet<crate::NodeId>;
    pub type HashSetFx<K> = HashSet<K>;
}

#[cfg(not(feature = "hashbrown"))]
mod inline {
    use foldhash::HashSet;
    pub type NodeIdSet = HashSet<crate::NodeId>;
    pub type HashSetFx<K> = HashSet<K>;
}

pub(crate) use inline::{HashSetFx, NodeIdSet};
