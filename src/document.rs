use std::borrow::Cow;
use std::cell::{Cell, Ref, RefCell};

use html5ever::interface::tree_builder;
use html5ever::parse_document;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName};
use tendril::StrTendril;
use tendril::TendrilSink;

use crate::dom_tree::{Element, InnerNode, NodeData, NodeRef, Tree};
use crate::entities::NodeId;
/// Document represents an HTML document to be manipulated.
pub struct Document {
    /// The document's dom tree.
    pub tree: Tree<NodeData>,

    /// Errors that occurred during parsing.
    pub errors: RefCell<Vec<Cow<'static, str>>>,

    /// The document's quirks mode.
    pub quirks_mode: Cell<QuirksMode>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            tree: Tree::new(NodeData::Document),
            errors: RefCell::new(vec![]),
            quirks_mode: Cell::new(tree_builder::NoQuirks),
        }
    }
}

impl<T: Into<StrTendril>> From<T> for Document {
    fn from(html: T) -> Self {
        parse_document(Document::default(), Default::default()).one(html)
    }
}

impl Document {
    /// Return the underlying root document node.
    #[inline]
    pub fn root(&self) -> NodeRef<NodeData> {
        self.tree.root()
    }
}

impl TreeSink for Document {
    type ElemName<'a> = Ref<'a, QualName>;
    // The overall result of parsing.
    type Output = Self;
    // Handle is a reference to a DOM node. The tree builder requires that a `Handle` implements `Clone` to get
    // another reference to the same node.
    type Handle = NodeId;

    // Consume this sink and return the overall result of parsing.
    #[inline]
    fn finish(self) -> Self {
        self
    }

    // Signal a parse error.
    #[inline]
    fn parse_error(&self, msg: Cow<'static, str>) {
        let mut errors = self.errors.borrow_mut();
        errors.push(msg);
    }

    // Get a handle to the `Document` node.
    #[inline]
    fn get_document(&self) -> Self::Handle {
        self.tree.root_id()
    }

    // Get a handle to a template's template contents. The tree builder promises this will never be called with
    // something else than a template element.
    #[inline]
    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        self.tree
            .query_node_or(target, None, |node| {
                node.as_element().and_then(|elem| elem.template_contents)
            })
            .expect("target node is not a template element!")
    }

    // Set the document's quirks mode.
    #[inline]
    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    // Do two handles refer to the same node?.
    #[inline]
    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        *x == *y
    }

    // What is the name of the element?
    // Should never be called on a non-element node; Feel free to `panic!`.
    #[inline]
    fn elem_name(&self, target: &Self::Handle) -> Self::ElemName<'_> {
        self.tree
            .query_node_or(target, None, |node| {
                if node.is_element() {
                    self.tree.get_name(target)
                } else {
                    None
                }
            })
            .expect("target node is not an element!")
    }

    // Create an element.
    // When creating a template element (`name.ns.expanded() == expanded_name!(html"template")`), an
    // associated document fragment called the "template contents" should also be created. Later calls to
    // self.get_template_contents() with that given element return it. See `the template element in the whatwg spec`,
    #[inline]
    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Self::Handle {
        let template_contents = if flags.template {
            Some(self.tree.create_node(NodeData::Document))
        } else {
            None
        };

        let id = self.tree.create_node(NodeData::Element(Element::new(
            name.clone(),
            attrs,
            template_contents,
            flags.mathml_annotation_xml_integration_point,
        )));

        self.tree.set_name(id, name);
        id
    }

    // Create a comment node.
    #[inline]
    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        self.tree.create_node(NodeData::Comment { contents: text })
    }

    // Create a Processing Instruction node.
    #[inline]
    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Self::Handle {
        self.tree.create_node(NodeData::ProcessingInstruction {
            target,
            contents: data,
        })
    }

    // Append a node as the last child of the given node. If this would produce adjacent sibling text nodes, it
    // should concatenate the text instead.
    // The child node will not already have a parent.
    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        // Append to an existing Text node if we have one.

        match child {
            NodeOrText::AppendNode(node_id) => self.tree.append_child_of(parent, &node_id),
            NodeOrText::AppendText(text) => {
                let last_child = self.tree.last_child_of(parent);
                let merged = last_child
                    .and_then(|child| {
                        self.tree
                            .update_node(&child.id, |node| append_to_existing_text(node, &text))
                    })
                    .unwrap_or(false);

                if merged {
                    return;
                }

                self.tree
                    .append_child_data_of(parent, NodeData::Text { contents: text })
            }
        }
    }

    // Append a node as the sibling immediately before the given node.
    // The tree builder promises that `sibling` is not a text node. However its old previous sibling, which would
    // become the new node's previous sibling, could be a text node. If the new node is also a text node, the two
    // should be merged, as in the behavior of `append`.
    fn append_before_sibling(&self, sibling: &Self::Handle, child: NodeOrText<Self::Handle>) {
        match child {
            NodeOrText::AppendText(text) => {
                let prev_sibling = self.tree.prev_sibling_of(sibling);
                let merged = prev_sibling
                    .and_then(|sibling| {
                        self.tree
                            .update_node(&sibling.id, |node| append_to_existing_text(node, &text))
                    })
                    .unwrap_or(false);

                if merged {
                    return;
                }

                let id = self.tree.create_node(NodeData::Text { contents: text });
                self.tree.append_prev_sibling_of(sibling, &id);
            }

            // The tree builder promises we won't have a text node after
            // the insertion point.

            // Any other kind of node.
            NodeOrText::AppendNode(id) => self.tree.append_prev_sibling_of(sibling, &id),
        };
    }

    // When the insertion point is decided by the existence of a parent node of the element, we consider both
    // possibilities and send the element which will be used if a parent node exists, along with the element to be
    // used if there isn't one.
    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        let has_parent = self.tree.parent_of(element).is_some();

        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    // Append a `DOCTYPE` element to the `Document` node.
    #[inline]
    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let root = self.tree.root_id();
        self.tree.append_child_data_of(
            &root,
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            },
        );
    }

    // Add each attribute to the given element, if no attribute with that name already exists. The tree builder
    // promises this will never be called with something else than an element.
    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        self.tree.update_node(target, |node| {
            if let Some(el) = node.as_element_mut() {
                el.add_attrs_if_missing(attrs);
            }
        });
    }

    // Detach the given node from its parent.
    #[inline]
    fn remove_from_parent(&self, target: &Self::Handle) {
        self.tree.remove_from_parent(target);
    }

    // Remove all the children from node and append them to new_parent.
    #[inline]
    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        self.tree.reparent_children_of(node, Some(*new_parent));
    }
}

fn append_to_existing_text(prev: &mut InnerNode<NodeData>, text: &str) -> bool {
    match prev.data {
        NodeData::Text { ref mut contents } => {
            contents.push_slice(text);
            true
        }
        _ => false,
    }
}
