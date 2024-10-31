mod data;

use data::doc_with_siblings;
use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_with_html() {
    let doc = doc_with_siblings();

    println!("{}", doc.html());

    let sel = doc.select("#main,#foot");
    println!("{}", sel.length());
    sel.replace_with_html(r#"<div id="replace"></div>"#);

    println!("{}", doc.html());
    println!("======");

    assert_eq!(doc.select("#replace").length(), 2);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_set_html() {
    let doc = doc_with_siblings();
    let q = doc.select("#main, #foot");
    q.set_html(r#"<div id="replace">test</div>"#);

    assert_eq!(doc.select("#replace").length(), 2);
    assert_eq!(doc.select("#main, #foot").length(), 2);

    let html: &str = &q.text();
    assert_eq!(html, "testtest");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_set_html_no_match() {
    let doc = doc_with_siblings();
    let q = doc.select("#notthere");
    q.set_html(r#"<div id="replace">test</div>"#);
    assert_eq!(doc.select("#replace").length(), 0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_set_html_empty() {
    let doc = doc_with_siblings();
    let q = doc.select("#main");
    q.set_html("");
    assert_eq!(doc.select("#main").length(), 1);
    assert_eq!(doc.select("#main").children().length(), 0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_with_selection() {
    let doc = doc_with_siblings();

    let s1 = doc.select("#nf5");
    let sel = doc.select("#nf6");

    sel.replace_with_selection(&s1);

    assert!(sel.is("#nf6"));
    assert_eq!(doc.select("#nf6").length(), 0);
    assert_eq!(doc.select("#nf5").length(), 1);
    s1.append_selection(&sel);
    // after appending detached element, it can be matched
    assert!(sel.is("#nf6"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn remove_descendant_attributes() {
    let contents = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main" style="bg-color:black">
                <p style="color:red">Red</p>
                <p style="color:white">White</p>
            </div>
        </body>
    </html>"#;

    // remove descendant attributes, but keep parent
    let doc = Document::from(contents);

    let main_sel = doc.select_single("#main");
    let children_sel = main_sel.select("[style]");

    let style_in_sel = children_sel
        .nodes()
        .iter()
        .any(|node| node.has_attr("style"));

    assert!(style_in_sel);

    children_sel.remove_attr("style");

    let style_in_sel = children_sel
        .nodes()
        .iter()
        .any(|node| node.has_attr("style"));

    assert!(!style_in_sel);

    assert!(main_sel.has_attr("style"));
}
