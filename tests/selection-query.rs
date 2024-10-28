mod data;

use data::{doc, HEADING_CONTENTS};

use dom_query::Document;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;



#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is() {
    let doc = doc();
    let sel = doc.select(".footer p:nth-child(1)");
    print!("{}", sel.length());
    assert!(sel.is("p"), "Expected .footer p:nth-child(1) to be a p.");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is_invalid() {
    let doc = doc();
    let sel = doc.select(".footer p:nth-child(1)");
    assert!(
        !sel.is(""),
        "is should not succeed with invalid selector string"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is_selection() {
    let doc = doc();
    let sel = doc.select("div");
    let sel2 = doc.select(".pvk-gutter");

    assert!(
        sel.is_selection(&sel2),
        "Expected some div to have a pvk-gutter class."
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is_selection_not() {
    let doc = doc();
    let sel = doc.select("div");
    let sel2 = doc.select("a");

    assert!(
        !sel.is_selection(&sel2),
        "Expected some div NOT to be an anchor."
    );
}


#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_filter_selection() {
    let doc: Document = HEADING_CONTENTS.into();
    // we captured 2 divs with `.content` class
    let sel = doc.select("div.content");
    assert!(sel.select("h1").exists());

    // create a new filtered selection, that will contain only elements with `.text-content` class
    let filtered_sel = sel.filter("div.text-content");

    // filtered selection does not contain any `h1`
    assert!(!filtered_sel.select("h1").exists());
    // while original selection contains `h1`
    assert!(sel.select("h1").exists());
}


#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_try_filter_selection() {
    let doc: Document = HEADING_CONTENTS.into();
    // we captured 2 divs with `.content` class
    let sel = doc.select("div.content");
    assert!(sel.select("h1").exists());
    let filtered_sel = sel.try_filter("div.text-content").unwrap();
    assert!(!filtered_sel.select("h1").exists());
    assert!(sel.select("h1").exists());
}