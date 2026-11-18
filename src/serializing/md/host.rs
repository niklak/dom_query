use std::cell::Ref;
use std::ops::{Deref, DerefMut};

use html5ever::QualName;

use crate::node::TreeNode;
use crate::{NodeId, Tree};

use super::block::BlockWriter;
use super::ctx::FormatOpts;

/// A scratch buffer that returns to the host's pool when dropped.
pub(super) struct ScratchBuf<'h, H: MDHost> {
    host: &'h H,
    buf: String,
}

impl<H: MDHost> Deref for ScratchBuf<'_, H> {
    type Target = String;
    fn deref(&self) -> &String {
        &self.buf
    }
}

impl<H: MDHost> DerefMut for ScratchBuf<'_, H> {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.buf
    }
}

impl<H: MDHost> Drop for ScratchBuf<'_, H> {
    fn drop(&mut self) {
        self.host.put_buf(std::mem::take(&mut self.buf));
    }
}

pub(super) trait MDHost: Sized {
    /// An empty buffer with the capacity of an earlier one; see `scratch`.
    fn take_buf(&self) -> String;
    fn put_buf(&self, buf: String);

    fn scratch(&self) -> ScratchBuf<'_, Self> {
        ScratchBuf {
            host: self,
            buf: self.take_buf(),
        }
    }

    fn tree(&self) -> &Tree;
    fn nodes(&self) -> Ref<'_, Vec<TreeNode>>;
    fn is_skipped(&self, name: &QualName) -> bool;
    /// The last list written and whether the next same-type list must use the
    /// alternate marker.
    fn last_list(&self) -> Option<(NodeId, bool)>;
    /// Records the last written list; `alt_marker` is the marker that list used,
    /// and the next same-type list gets the opposite one.
    fn set_last_list(&self, list: NodeId, alt_marker: bool);

    /// Writes `id` when `opts.include_node`, otherwise its children.
    fn write(&self, out: &mut BlockWriter, id: NodeId, opts: FormatOpts);

    /// Writes `id` as a document of its own, in a buffer from the pool.
    fn write_buf(&self, id: NodeId, opts: FormatOpts) -> String {
        let mut out = BlockWriter::document(self.take_buf(), opts);
        self.write(&mut out, id, opts);
        out.finish()
    }

    fn write_children(&self, out: &mut BlockWriter, id: NodeId, mut opts: FormatOpts) {
        opts.include_node = false;
        self.write(out, id, opts);
    }
}
