use std::io;

use html5ever::serialize::TraversalScope;
use html5ever::serialize::{Serialize, Serializer};
use html5ever::QualName;

use super::node_data::NodeData;
use super::node_ref::{Node, NodeRef};
use super::NodeId;

enum SerializeOp<'a> {
    Open(NodeId),
    Close(&'a QualName),
}
/// Serializable wrapper of Node.
pub struct SerializableNodeRef<'a>(Node<'a>);

impl<'a> From<NodeRef<'a>> for SerializableNodeRef<'a> {
    fn from(h: NodeRef<'a>) -> SerializableNodeRef<'a> {
        SerializableNodeRef(h)
    }
}

impl<'a> Serialize for SerializableNodeRef<'a> {
    fn serialize<S>(&self, serializer: &mut S, traversal_scope: TraversalScope) -> io::Result<()>
    where
        S: Serializer,
    {
        let nodes = self.0.tree.nodes.borrow();
        let id = self.0.id;

        // Initialize ops stack
        let mut ops = match traversal_scope {
            TraversalScope::IncludeNode => vec![SerializeOp::Open(id)],
            TraversalScope::ChildrenOnly(_) => {
                // For children only, add all child nodes
                self.0
                    .tree
                    .child_ids_of(&id)
                    .into_iter()
                    .rev()
                    .map(SerializeOp::Open)
                    .collect()
            }
        };
        while let Some(op) = ops.pop() {
            match op {
                SerializeOp::Open(id) => {
                    let node = match nodes.get(id.value) {
                        Some(node) => node,
                        None => continue,
                    };

                    match &node.data {
                        NodeData::Element(e) => {
                            serializer.start_elem(
                                e.name.clone(),
                                e.attrs.iter().map(|at| (&at.name, &at.value[..])),
                            )?;

                            ops.push(SerializeOp::Close(&e.name));
                            ops.extend(
                                self.0
                                    .tree
                                    .child_ids_of(&id)
                                    .into_iter()
                                    .rev()
                                    .map(SerializeOp::Open),
                            );

                            Ok(())
                        }
                        NodeData::Doctype { ref name, .. } => serializer.write_doctype(name),
                        NodeData::Text { ref contents } => serializer.write_text(contents),
                        NodeData::Comment { ref contents } => serializer.write_comment(contents),
                        NodeData::ProcessingInstruction {
                            ref target,
                            ref contents,
                        } => serializer.write_processing_instruction(target, contents),
                        NodeData::Document | NodeData::Fragment => {
                            // Push children in reverse order
                            ops.extend(
                                self.0
                                    .tree
                                    .child_ids_of(&id)
                                    .into_iter()
                                    .rev()
                                    .map(SerializeOp::Open),
                            );
                            continue;
                        }
                    }?;
                }
                SerializeOp::Close(name) => serializer.end_elem(name.clone())?,
            }
        }

        Ok(())
    }
}
