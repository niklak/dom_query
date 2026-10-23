use crate::node::TreeNode;

use super::super::ctx::FormatOpts;
use super::super::serializer::MDSerializer;

impl MDSerializer<'_> {
    pub(in crate::serializing::md) fn write_blockquote(
        &self,
        text: &mut String,
        quote_node: &TreeNode,
    ) {
        let opts = FormatOpts::new();
        let mut quote_buf = String::new();
        self.write(&mut quote_buf, quote_node.id, opts);

        if quote_buf.is_empty() {
            return;
        }

        while !text.ends_with("\n\n") {
            text.push('\n');
        }

        for line in quote_buf.lines() {
            text.push_str("> ");
            text.push_str(line);
            text.push('\n');
        }

        text.push('\n');
    }
}
