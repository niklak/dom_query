use std::cell::Ref;
use std::io;

use html5ever::QualName;
use html5ever::serialize::TraversalScope;
use html5ever::serialize::{Serialize, Serializer};

use super::node_data::NodeData;
use super::node_ref::NodeRef;
use super::{NodeId, child_nodes};

pub(crate) enum SerializeOp<'a> {
    Open(NodeId),
    Close(&'a QualName),
}
/// Serializable wrapper of Node.
pub struct SerializableNodeRef<'a>(NodeRef<'a>);

impl<'a> From<NodeRef<'a>> for SerializableNodeRef<'a> {
    fn from(h: NodeRef<'a>) -> Self {
        Self(h)
    }
}

impl Serialize for SerializableNodeRef<'_> {
    fn serialize<S>(&self, serializer: &mut S, traversal_scope: TraversalScope) -> io::Result<()>
    where
        S: Serializer,
    {
        let nodes = self.0.tree.nodes.borrow();
        let id = self.0.id;

        // Initialize ops stack
        let mut ops = match traversal_scope {
            TraversalScope::IncludeNode => vec![SerializeOp::Open(id)],
            TraversalScope::ChildrenOnly(_) => child_nodes(Ref::clone(&nodes), &id, true)
                .map(SerializeOp::Open)
                .collect(),
        };
        while let Some(op) = ops.pop() {
            match op {
                SerializeOp::Open(id) => {
                    let Some(node) = nodes.get(id.value()) else {
                        continue;
                    };

                    match &node.data {
                        NodeData::Element(e) => {
                            serializer.start_elem(
                                e.name.clone(),
                                e.attrs.iter().map(|at| (&at.name, &at.value[..])),
                            )?;

                            ops.push(SerializeOp::Close(&e.name));
                            ops.extend(
                                child_nodes(Ref::clone(&nodes), &id, true).map(SerializeOp::Open),
                            );

                            if let Some(tpl_doc_root) = e.template_contents {
                                ops.push(SerializeOp::Open(tpl_doc_root));
                            }

                            Ok(())
                        }
                        NodeData::Doctype { name, .. } => serializer.write_doctype(name),
                        NodeData::Text { contents } => serializer.write_text(contents),
                        NodeData::Comment { contents } => serializer.write_comment(contents),
                        NodeData::ProcessingInstruction { target, contents } => {
                            serializer.write_processing_instruction(target, contents)
                        }
                        NodeData::Document | NodeData::Fragment => {
                            // Push children in reverse order
                            ops.extend(
                                child_nodes(Ref::clone(&nodes), &id, true).map(SerializeOp::Open),
                            );
                            Ok(())
                        }
                    }?;
                }
                SerializeOp::Close(name) => serializer.end_elem(name.clone())?,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use html5ever::serialize::{SerializeOpts, serialize};

    use super::*;
    use crate::Document;

    #[test]
    fn test_template_serialization() {
        let contents = r"<html>
                <head></head>
                <body>
                    <template>
                        <p>template content</p>
                    </template>
                </body>
            </html>";
        let doc = Document::from(contents);
        let got_html = doc.html();
        assert_eq!(
            got_html
                .split_ascii_whitespace()
                .collect::<Vec<&str>>()
                .join(""),
            contents
                .split_ascii_whitespace()
                .collect::<Vec<&str>>()
                .join("")
        );
    }

    #[test]
    fn test_serialize_write_error() {
        let doc = Document::from("<p>a</p>");
        let p = doc.select_single("p").nodes()[0];
        let opts = SerializeOpts {
            traversal_scope: TraversalScope::IncludeNode,
            ..Default::default()
        };
        // an element fails on its start tag, a text node on its text: a zero-length
        // buffer can't take a single byte
        for node in [p, p.first_child().unwrap()] {
            let mut buf = [0u8; 0];
            let res = serialize(&mut buf[..], &SerializableNodeRef::from(node), opts.clone());
            assert!(res.is_err());
        }
    }
}
