mod data;

use data::ANCESTORS_CONTENTS;
use dom_query::{Document, NodeData};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_first_element_child_edge_cases() {
    let html = r#"
        <div id="empty"></div>
        <div id="text-only">Some text</div>
        <div id="multiple">
            <span>First</span>
            <span>Second</span>
        </div>
        <div id="nested">
            <div>
                <span>Nested</span>
            </div>
        </div>
    "#;

    let doc: Document = html.into();

    // Test empty parent
    let empty_sel = doc.select("#empty");
    let empty = empty_sel.nodes().first().unwrap();
    assert!(empty.first_element_child().is_none());

    // Test text-only parent
    let text_only_sel = doc.select("#text-only");
    let text_only = text_only_sel.nodes().first().unwrap();
    assert!(text_only.first_element_child().is_none());

    // Test multiple children
    let multiple_sel = doc.select("#multiple");
    let multiple = multiple_sel.nodes().first().unwrap();
    let first = multiple.first_element_child().unwrap();
    assert_eq!(first.text(), "First".into());
    assert!(first.is_element());

    // Test nested elements
    let nested_sel = doc.select("#nested");
    let nested = nested_sel.nodes().first().unwrap();
    let first_nested = nested.first_element_child().unwrap();
    assert!(first_nested.is_element());
    assert_eq!(
        first_nested.first_element_child().unwrap().text(),
        "Nested".into()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_descendants_iter() {
    let doc: Document = ANCESTORS_CONTENTS.into();

    let ancestor = doc.select("#great-ancestor");
    assert!(ancestor.exists());

    let ancestor_node = ancestor.nodes().first().unwrap();

    // with no depth limit
    let descendants_id_names = ancestor_node
        .descendants_it()
        .filter(|n| n.is_element())
        .map(|n| n.attr_or("id", "").to_string())
        .collect::<Vec<_>>();

    let expected_id_names = vec![
        "grand-parent",
        "parent",
        "first-child",
        "second-child",
        "grand-parent-sibling",
    ];
    assert_eq!(descendants_id_names, expected_id_names);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_descendants() {
    let doc: Document = ANCESTORS_CONTENTS.into();

    let ancestor = doc.select("#great-ancestor");
    assert!(ancestor.exists());

    let ancestor_node = ancestor.nodes().first().unwrap();

    let expected_id_names = vec![
        "grand-parent-sibling",
        "second-child",
        "first-child",
        "parent",
        "grand-parent",
    ];

    // if you want to reuse descendants then use `descendants` which returns a vector of nodes
    let descendants = ancestor_node.descendants();

    let text_nodes_count = descendants
        .iter()
        .filter(|n| n.is_text() && n.text().trim() != "")
        .count();
    let offsets_count = descendants
        .iter()
        .filter(|n| n.is_text() && n.text().trim() == "")
        .count();
    // Descendants include not only element nodes, but also text nodes.
    // Whitespace characters between element nodes are also considered as text nodes.
    // Therefore, the number of descendants is usually not equal to the number of element descendants.
    assert_eq!(
        descendants.len(),
        expected_id_names.len() + text_nodes_count + offsets_count
    );

    // with no depth limit
    let descendants_id_names = descendants
        .iter()
        .rev()
        .filter(|n| n.is_element())
        .map(|n| n.attr_or("id", "").to_string())
        .collect::<Vec<_>>();

    assert_eq!(descendants_id_names, expected_id_names);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_last_child() {
    let doc: Document = ANCESTORS_CONTENTS.into();

    let parent_sel = doc.select_single("#parent");
    assert!(parent_sel.exists());
    let last_child = parent_sel.nodes().first().and_then(|n| n.last_child());

    // when dealing with formatted documents, the last child may be a text node like "\n   "
    assert!(last_child.unwrap().is_text());

    let parent_sel = doc.select_single("#grand-parent-sibling");
    assert!(parent_sel.exists());
    let last_child = parent_sel.nodes().first().and_then(|n| n.last_child());

    assert!(last_child.is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_last_sibling() {
    let doc: Document = ANCESTORS_CONTENTS.into();
    let first_sel = doc.select_single("#first-child");
    assert!(first_sel.exists());
    let last_sibling = first_sel.nodes().first().and_then(|n| n.last_sibling());
    // when dealing with formatted documents, the last node may be a text node like "\n   "
    assert!(last_sibling.unwrap().is_text());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is_comment() {
    let doc: Document = ANCESTORS_CONTENTS.into();
    let ancestor_sel = doc.select_single("body");
    let ancestor_node = ancestor_sel.nodes().first().unwrap();
    let first_comment = ancestor_node
        .children_it(false)
        .find(|n| n.is_comment())
        .unwrap();

    let comment = first_comment.query_or("".to_string(), |n| match n.data {
        NodeData::Comment { ref contents } => contents.to_string(),
        _ => "".to_string(),
    });

    assert_eq!(comment, "Ancestors");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_element_children() {
    let doc: Document = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="main"><div>1</div><div>2</div><div>3</div>Inline text</div>
        <body>
    </html>"#
        .into();
    let sel = doc.select_single("div.main");

    // our main node
    let main_node = sel.nodes().first().unwrap();
    // `Node::children` includes all children nodes of its, not only element, but also text
    // tabs and newlines considered as text.
    assert_eq!(main_node.children().len(), 4);

    // `Node::element_children` includes only elements nodes
    assert_eq!(main_node.element_children().len(), 3);
}
