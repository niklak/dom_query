mod data;

use data::ANCESTORS_CONTENTS;
use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_first_element_child() {
    let doc: Document = ANCESTORS_CONTENTS.into();

    let parent_sel = doc.select("#parent");
    let parent_node = parent_sel.nodes().first().unwrap();
    // Because any indentation marks between HTML elements are considered as text nodes,
    // there is a necessity to distinguish the first child node and the first child element node.
    let first_child = parent_node.first_child().unwrap();
    // striping indentation
    assert_eq!(first_child.text().trim(), "");

    let first_element_child = parent_node.first_element_child().unwrap();
    assert_eq!(first_element_child.text(), "Child".into());
}
