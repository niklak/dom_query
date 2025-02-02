use std::cell::Ref;
use std::io;

use html5ever::serialize::TraversalScope;
use html5ever::serialize::{Serialize, Serializer};

use html5ever::{local_name, QualName};
use tendril::StrTendril;

use crate::TreeNodeOps;

use super::node_data::NodeData;
use super::node_ref::NodeRef;
use super::{child_nodes, NodeId};

enum SerializeOp<'a> {
    Open(NodeId),
    Close(&'a QualName),
}
/// Serializable wrapper of Node.
pub struct SerializableNodeRef<'a>(NodeRef<'a>);

impl<'a> From<NodeRef<'a>> for SerializableNodeRef<'a> {
    fn from(h: NodeRef<'a>) -> SerializableNodeRef<'a> {
        SerializableNodeRef(h)
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
                                child_nodes(Ref::clone(&nodes), &id, true).map(SerializeOp::Open),
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
                                child_nodes(Ref::clone(&nodes), &id, true).map(SerializeOp::Open),
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

pub(crate) fn format_text(root_node: &NodeRef, include_node: bool) -> StrTendril {
    let id = root_node.id;
    let nodes = root_node.tree.nodes.borrow();
    let mut ops = if include_node {
        vec![SerializeOp::Open(id)]
    } else {
        child_nodes(Ref::clone(&nodes), &id, true)
            .map(SerializeOp::Open)
            .collect()
    };

    let mut text = StrTendril::new();

    while let Some(op) = ops.pop() {
        match op {
            SerializeOp::Open(id) => {
                let node = match nodes.get(id.value) {
                    Some(node) => node,
                    None => continue,
                };

                match node.data {
                    NodeData::Text { ref contents } => {
                        let follows_newline = text.ends_with('\n') || text.is_empty();
                        let normalized = normalize_text(contents.as_ref(), follows_newline);
                        text.push_tendril(&normalized);
                    }
                    NodeData::Element(ref e) => {
                        ops.push(SerializeOp::Close(&e.name));

                        if matches!(e.name.local, local_name!("pre")) {
                            text.push_tendril(&TreeNodeOps::text_of(Ref::clone(&nodes), id));
                            continue;
                        }

                        ops.extend(
                            child_nodes(Ref::clone(&nodes), &id, true).map(SerializeOp::Open),
                        );
                    }
                    _ => {}
                }
            }
            SerializeOp::Close(name) => {
                if text.ends_with("\n\n") {
                    continue;
                }
                if matches!(
                    name.local,
                    local_name!("article")
                        | local_name!("blockquote")
                        | local_name!("section")
                        | local_name!("div")
                        | local_name!("p")
                        | local_name!("pre")
                        | local_name!("h1")
                        | local_name!("h2")
                        | local_name!("h3")
                        | local_name!("h4")
                        | local_name!("h5")
                        | local_name!("h6")
                        | local_name!("ul")
                        | local_name!("ol")
                        | local_name!("dl")
                        | local_name!("table")
                ) {
                    text.push_slice("\n\n");
                } else if matches!(
                    name.local,
                    local_name!("br") | local_name!("hr") | local_name!("li") | local_name!("tr")
                ) {
                    text.push_char('\n');
                }
            }
        }
    }
    if !include_node {
        while !text.is_empty() && text.ends_with(char::is_whitespace) {
            text.pop_back(1);
        }
    }
    text
}

fn normalize_text(text: &str, follows_newline: bool) -> StrTendril {
    let push_start_whitespace = !follows_newline && text.starts_with(char::is_whitespace);
    let push_end_whitespace = text.ends_with(char::is_whitespace);

    let mut result = StrTendril::with_capacity(text.len() as u32);
    let mut iter = text.split_whitespace();

    if let Some(first) = iter.next() {
        if push_start_whitespace {
            result.push_char(' ');
        }
        result.push_slice(first);
        for word in iter {
            result.push_char(' ');
            result.push_slice(word);
        }
    }
    if result.is_empty() {
        return result;
    }
    if push_end_whitespace {
        result.push_char(' ');
    }
    result
}
