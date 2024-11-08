mod data;

use dom_query::Document;

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
