use tendril::StrTendril;

use crate::{Document, NodeRef};
use super::serialize_md;

impl NodeRef<'_> {
    /// Produces a *Markdown* representation of the node and its descendants,  
    /// skipping elements matching the specified `skip_tags` list along with their descendants.  
    ///  
    /// - If `skip_tags` is `None`, the default list is used: `["script", "style", "meta", "head"]`.  
    /// - To process all elements without exclusions, pass `Some(&[])`.
    pub fn md(&self, skip_tags: Option<&[&str]>) -> StrTendril {
        serialize_md(self, false, skip_tags)
    }
}

impl Document {
    /// Produces a *Markdown* representation of the [`Document`],  
    /// skipping elements matching the specified `skip_tags` list along with their descendants.  
    ///  
    /// - If `skip_tags` is `None`, the default list is used: `["script", "style", "meta", "head"]`.  
    /// - To process all elements without exclusions, pass `Some(&[])`.
    pub fn md(&self, skip_tags: Option<&[&str]>) -> StrTendril {
        self.root().md(skip_tags)
    }
}
