use std::io;

use html5ever::serialize::TraversalScope;
use html5ever::serialize::{Serialize, Serializer};
use html5ever::QualName;

use super::children_of;
use super::node_data::NodeData;
use super::node_ref::{Node, NodeRef};
use super::NodeId;

enum SerializeOp {
    Open(NodeId),
    Close(QualName),
}
/// Serializable wrapper of Node.
pub struct SerializableNodeRef<'a>(Node<'a>);

impl<'a> From<NodeRef<'a, NodeData>> for SerializableNodeRef<'a> {
    fn from(h: NodeRef<'a, NodeData>) -> SerializableNodeRef {
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
        let mut ops = match traversal_scope {
            TraversalScope::IncludeNode => vec![SerializeOp::Open(id)],
            TraversalScope::ChildrenOnly(_) => children_of(&nodes, &id)
                .into_iter()
                .map(SerializeOp::Open)
                .collect(),
        };

        while !ops.is_empty() {
            match ops.remove(0) {
                SerializeOp::Open(id) => {
                    let node_opt = &nodes.get(id.value);
                    let node = match node_opt {
                        Some(node) => node,
                        None => continue,
                    };

                    match node.data {
                        NodeData::Element(ref e) => {
                            serializer.start_elem(
                                e.name.clone(),
                                e.attrs.iter().map(|at| (&at.name, &at.value[..])),
                            )?;

                            ops.insert(0, SerializeOp::Close(e.name.clone()));

                            for child_id in children_of(&nodes, &id).into_iter().rev() {
                                ops.insert(0, SerializeOp::Open(child_id));
                            }

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
                            for child_id in children_of(&nodes, &id).into_iter().rev() {
                                ops.insert(0, SerializeOp::Open(child_id));
                            }
                            continue;
                        }
                    }
                }
                SerializeOp::Close(name) => serializer.end_elem(name),
            }?
        }

        Ok(())
    }
}
