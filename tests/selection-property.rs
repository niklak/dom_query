mod data;

use data::doc;
use data::doc_with_siblings;

use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_attr_exists() {
    let doc = doc();
    assert!(doc.select("a").attr("href").is_some());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_attr_or() {
    let doc = doc();
    let attr1: &str = &doc.select("a").attr_or("fake-attribute", "alternative");
    let attr2: &str = &doc.select("zz").attr_or("fake-attribute", "alternative");
    assert_eq!(attr1, "alternative");
    assert_eq!(attr2, "alternative");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_attr_not_exist() {
    let doc = doc();
    assert!(doc.select("div.row-fluid").attr("href").is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_remove_attr() {
    let doc = doc_with_siblings();
    let sel = doc.select("div");

    sel.remove_attr("id");

    assert!(sel.attr("id").is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_remove_attr_empty_string() {
    let doc = doc_with_siblings();
    let sel = doc.select("div");

    sel.remove_attr("");

    assert!(sel.attr("id").is_some());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_set_attr() {
    let doc = doc_with_siblings();
    let sel = doc.select("#main");
    sel.set_attr("id", "not-main");

    let id: &str = &sel.attr("id").expect("got an attribute");
    assert_eq!(id, "not-main");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_set_attr2() {
    let doc = doc_with_siblings();
    let sel = doc.select("#main");

    sel.set_attr("foo", "bar");

    let id: &str = &sel.attr("foo").expect("got an attribute");
    assert_eq!(id, "bar");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_text() {
    let doc = doc();
    let txt: &str = &doc.select("h1").text();

    assert_eq!(txt.trim(), "Provok.in");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_add_class() {
    let doc = doc_with_siblings();
    let sel = doc.select("#main");

    sel.add_class("main main main");
    let class: &str = &sel.attr("class").unwrap();
    assert_eq!(class, "main");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_add_class_similar() {
    let doc = doc_with_siblings();
    let sel = doc.select("#nf5");

    sel.add_class("odd");
    println!("{}", sel.html());

    assert!(sel.has_class("odd"));
    assert!(sel.has_class("odder"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_add_empty_class() {
    let doc = doc_with_siblings();
    let sel = doc.select("#main");

    sel.add_class("");
    assert!(sel.attr("class").is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_add_classes() {
    let doc = doc_with_siblings();
    let sel = doc.select("#main");

    sel.add_class("a b");
    assert!(sel.has_class("a"));
    assert!(sel.has_class("b"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_has_class() {
    let doc = doc();
    let sel = doc.select("div");
    assert!(sel.has_class("span12"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn has_class_none() {
    let doc = doc();
    let sel = doc.select("toto");
    assert!(!sel.has_class("toto"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn has_class_not_first() {
    let doc = doc();
    let sel = doc.select(".alert");
    assert!(sel.has_class("alert-error"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_remove_class() {
    let doc = doc_with_siblings();
    let sel = doc.select("#nf1");
    sel.remove_class("one row");

    assert!(sel.has_class("even"));
    assert!(!sel.has_class("one"));
    assert!(!sel.has_class("row"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_remove_class_similar() {
    let doc = doc_with_siblings();
    let sel = doc.select("#nf5, #nf6");
    assert_eq!(sel.length(), 2);

    sel.remove_class("odd");
    assert!(sel.has_class("odder"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_remove_attrs() {
    let doc: Document = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div id="main" class="main" style="color:green;">Green content</div>
        <body>
    </html>"#
        .into();
    let sel = doc.select("div#main");

    sel.remove_attrs(&["id", "style"]);

    assert_eq!(
        sel.html(),
        r#"<div class="main">Green content</div>"#.into()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_has_attr() {
    let doc: Document = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
             <p hidden>This paragraph should be hidden.</p> 
        <body>
    </html>"#
        .into();
    let sel = doc.select("p");
    let is_hidden = sel.has_attr("hidden");
    assert!(is_hidden);
    let has_title = sel.has_attr("title");
    assert!(!has_title);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_rename_tags() {
    let doc: Document = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="content">
                <div>1</div>
                <div>2</div>
                <div>3</div>
            </div>
        <body>
    </html>"#
        .into();
    let sel = doc.select("div.content > div");

    assert_eq!(sel.length(), 3);

    sel.rename("p");

    assert_eq!(doc.select("div.content > div").length(), 0);

    assert_eq!(doc.select("div.content > p").length(), 3);
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