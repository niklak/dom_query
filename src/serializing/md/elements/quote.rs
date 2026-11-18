use crate::NodeId;

use super::super::block::{BlockWriter, LinePrefix, Separator};
use super::super::ctx::FormatOpts;
use super::super::host::MDHost;

pub(in crate::serializing::md) fn write<H: MDHost>(h: &H, out: &mut BlockWriter, id: NodeId) {
    out.open_prefix(LinePrefix::Quote);
    h.write(out, id, FormatOpts::new());
    out.close_prefix();
    out.owe(Separator::BlankLine);
}
