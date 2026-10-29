use std::cell::Ref;

use html5ever::QualName;

use crate::node::{TreeNode, child_nodes};
use crate::{NodeId, Tree};

use super::ctx::FormatOpts;

pub(super) trait MDHost {
    fn tree(&self) -> &Tree;
    fn nodes(&self) -> Ref<'_, Vec<TreeNode>>;
    fn is_skipped(&self, name: &QualName) -> bool;
    /// The last list written, and whether it took the alternate marker.
    fn last_list(&self) -> Option<(NodeId, bool)>;
    fn set_last_list(&self, list: NodeId, alt_marker: bool);

    /// Writes `id` when `opts.include_node`, otherwise its children; trims unless included.
    fn write(&self, out: &mut String, id: NodeId, opts: FormatOpts);

    /// Writes each child in document order, without trimming.
    fn write_children(&self, out: &mut String, id: NodeId, opts: FormatOpts) {
        let nodes = self.nodes();
        for child in child_nodes(nodes, &id, false) {
            self.write(out, child, opts);
        }
    }
}
