mod data;

use data::REPLACEMENT_CONTENTS;
use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_create_element() {
    let contents = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main">
            <div>
        </body>
    </html>"#;

    let doc = Document::from(contents);

    let main_id = doc.select_single("#main").nodes().iter().next().unwrap().id;

    let el = doc.tree.new_element("p");
    el.set_attr("id", "inline");
    doc.tree.append_child_of(&main_id, &el.id);

    assert!(doc.select("#main #inline").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_element_html() {
    let contents = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main">
                <p id="first">It's</p>
            <div>
        </body>
    </html>"#;

    let doc = Document::from(contents);
    let main_sel = doc.select_single("#main");
    let main_node = main_sel.nodes().first().unwrap();
    main_node.append_html(r#"<p id="second">Wonderful</p>"#);
    assert_eq!(doc.select("#main #second").text().as_ref(), "Wonderful");
    assert!(doc.select("#first").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_set_element_html() {
    let contents = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main">
                <p id="first">It's</p>
            </div>
        </body>
    </html>"#;

    let doc = Document::from(contents);
    let main_sel = doc.select_single("#main");
    let main_node = main_sel.nodes().first().unwrap();
    main_node.set_html(r#"<p id="second">Wonderful</p>"#);
    assert_eq!(doc.select("#main #second").text().as_ref(), "Wonderful");
    assert!(!doc.select("#first").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_change_parent_node() {

    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    // create a new `p` element with id:
    let p = doc.tree.new_element("p");
    p.set_attr("id", "outline");

    // taking origin_node's place
    origin_node.append_prev_sibling(&p.id);
    // remove it from it's current parent
    origin_node.remove_from_parent();
    // append it to new p element
    p.append_child(&origin_node.id);

    assert!(doc.select("#outline > #origin > #inline").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_replace_with() {

    // It's actually the same test as `test_change_parent_node`, 
    // using `replace_with` instead of `append_prev_sibling` and `remove_from_parent`
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    // create a new `p` element with id:
    let p = doc.tree.new_element("p");
    p.set_attr("id", "outline");

    // replacing origin_node with `p` node, detaching `origin_node` from the tree
    origin_node.replace_with(&p.id);

    // append `origin_node` it to the new `p` node
    p.append_child(&origin_node.id);

    assert!(doc.select("#outline > #origin > #inline").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_replace_with_html() {

    // It's actually the same test as `test_change_parent_node`, 
    // using `replace_with` instead of `append_prev_sibling` and `remove_from_parent`
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();
    // replacing origin_node with `p` node, detaching `origin_node` from the tree, origin node is detached
    origin_node.replace_with_html(r#"<p id="replaced"><span id="inline">Something</span></p>"#);
    
    // checking if #replaced can be access as next sibling of #before-origin
    assert!(doc.select("#before-origin + #replaced > #inline").exists());
    // checking if #after-origin can be access after it's new previous sibling
    assert!(doc.select("#replaced + #after-origin").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_replace_with_reparent() {

    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    // create a new `p` element with id:
    let p = doc.tree.new_element("p");
    p.set_attr("id", "outline");

    //taking node's place
    // taking origin_node's place
    origin_node.append_prev_sibling(&p.id);
    // remove it from it's current parent
    origin_node.remove_from_parent();
    // attach all children nodes to new p element
    doc.tree.reparent_children_of(&origin_node.id, Some(p.id));

    // #origin is not in the tree now
    assert!(!doc.select("#origin").exists());
    // #inline is a child of #outline now
    assert!(doc.select("#outline > #inline").exists());
}

