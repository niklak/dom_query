mod data;

use data::{ANCESTORS_CONTENTS, REPLACEMENT_CONTENTS};
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
fn test_append_existing_element() {
    let doc = Document::from(REPLACEMENT_CONTENTS);
    let origin_sel = doc.select_single("p#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    assert_eq!(doc.select_single("#origin").text(), "Something".into());

    let span_sel = doc.select_single(" #after-origin span");
    let span_node = span_sel.nodes().first().unwrap();

    origin_node.append_child(span_node);

    assert_eq!(doc.select_single("#origin").text(), "SomethingAbout".into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_existing_children() {
    let doc = Document::from(REPLACEMENT_CONTENTS);
    let origin_sel = doc.select_single("p#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    assert_eq!(doc.select_single("#origin").text(), "Something".into());

    let span_sel = doc.select_single(" #after-origin span");
    let span_node = span_sel.nodes().first().unwrap();

    // this thing adds a child element and its sibling after existing child nodes.
    origin_node.append_children(span_node);

    assert_eq!(
        doc.select_single("#origin").text(),
        "SomethingAboutMe".into()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_prepend_existing_element() {
    let doc = Document::from(REPLACEMENT_CONTENTS);
    let origin_sel = doc.select_single("p#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    assert_eq!(doc.select_single("#origin").text(), "Something".into());

    let span_sel = doc.select_single(" #after-origin span");
    let span_node = span_sel.nodes().first().unwrap();

    origin_node.prepend_child(span_node);

    assert_eq!(doc.select_single("#origin").text(), "AboutSomething".into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_prepend_existing_children() {
    let doc = Document::from(REPLACEMENT_CONTENTS);
    let origin_sel = doc.select_single("p#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    assert_eq!(doc.select_single("#origin").text(), "Something".into());

    let span_sel = doc.select_single(" #after-origin span");
    let span_node = span_sel.nodes().first().unwrap();

    // this thing adds a child element and its sibling before existing child nodes.
    origin_node.prepend_children(span_node);

    assert_eq!(
        doc.select_single("#origin").text(),
        "AboutMeSomething".into()
    );
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
    origin_node.insert_before(&p.id);
    // remove it from it's current parent
    origin_node.remove_from_parent();
    // append it to new p element
    p.append_child(&origin_node.id);

    assert!(doc.select("#outline > #origin > #inline").exists());
}

#[allow(deprecated)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_change_parent_node_old() {
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

#[allow(deprecated)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_change_parent_nodes_old() {
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    // create a new `p` element with id:
    let p = doc.tree.new_element("p");
    p.set_attr("id", "outline");

    // taking origin_node's place
    origin_node.append_prev_siblings(&p.id);
    // remove it from it's current parent
    origin_node.remove_from_parent();
    // append it to new p element
    p.append_child(&origin_node.id);

    assert!(doc.select("#outline > #origin > #inline").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_replace_with_by_node_id() {
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
fn test_node_replace_with_by_node() {
    // It's actually the same test as `test_node_replace_with_by_node`,
    // using but using &node instead of &node.id in node methods.
    // which i find more readable
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel: dom_query::Selection<'_> = doc.select_single("#origin");
    let origin_node: &dom_query::NodeRef<'_> = origin_sel.nodes().first().unwrap();

    // create a new `p` element with id:
    let p: dom_query::NodeRef<'_> = doc.tree.new_element("p");
    p.set_attr("id", "outline");

    // replacing origin_node with `p` node, detaching `origin_node` from the tree
    origin_node.replace_with(&p);

    // append `origin_node` it to the new `p` node
    p.append_child(origin_node);

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
    origin_node.insert_before(&p.id);
    // remove it from it's current parent
    origin_node.remove_from_parent();
    // attach all children nodes to new p element
    doc.tree.reparent_children_of(&origin_node.id, Some(p.id));

    // #origin is not in the tree now
    assert!(!doc.select("#origin").exists());
    // #inline is a child of #outline now
    assert!(doc.select("#outline > #inline").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_replace_text_node() {
    let content = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main">
                <p><a href="javascript:void(0)">Some text</a></p>
            </div>
        </body>
    </html>"#;
    let doc = Document::from(content);
    // :only-text pseudo-class allows to select nodes that contain only one text node
    let a_sel = doc.select_single(r#"a[href^="javascript:"]:only-text"#);
    assert!(a_sel.exists());
    let a_node = a_sel.nodes().first().unwrap();
    let text_node = a_node.first_child().unwrap();
    assert!(text_node.is_text());
    a_node.replace_with(&text_node);

    assert_eq!(doc.select("#main > p").inner_html(), "Some text".into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_set_text() {
    //! replacing existing content with text content
    let content = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div id="main">
                <div id="content"><b>Original</b> content</div>
            </div>
        </body>
    </html>"#;
    let doc = Document::from(content);
    let content_sel = doc.select("#content");
    let content_node = content_sel.nodes().first().unwrap();

    let text = "New content";
    content_node.set_text(text);
    assert_eq!(content_node.inner_html(), text.into());
    assert_eq!(doc.select("#content").inner_html(), text.into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_prepend() {
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    // create a new `span` element with id:
    let span = doc.tree.new_element("span");
    span.set_attr("id", "first");

    //taking node's place
    // taking origin_node's place
    origin_node.prepend_child(&span);

    // #origin is not in the tree now
    assert!(doc.select("#origin").exists());
    // #inline is a child of #outline now
    assert!(doc.select("#origin > #first  + #inline").exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_prepend_html() {
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let origin_sel = doc.select_single("#origin");
    let origin_node = origin_sel.nodes().first().unwrap();

    // you may prepend html fragment with one element inside,
    origin_node.prepend_html(r#"<span id="third">3</span>"#);

    // or more...
    origin_node.prepend_html(r#"<span id="first">1</span><span id="second">2</span>"#);
    assert!(doc
        .select("#origin > #first + #second + #third + #inline")
        .exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_insert_before() {
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let sel = doc.select_single("#before-origin");
    let node = sel.nodes().first().unwrap();

    let new_node = doc.tree.new_element("p");
    new_node.set_attr("id", "before-before-origin");

    node.insert_before(&new_node);

    assert!(doc
        .select("#before-before-origin + #before-origin + #origin + #after-origin")
        .exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_insert_after() {
    let doc = Document::from(REPLACEMENT_CONTENTS);

    let sel = doc.select_single("#after-origin");
    let node = sel.nodes().first().unwrap();

    let new_node = doc.tree.new_element("p");
    new_node.set_attr("id", "after-after-origin");

    node.insert_after(&new_node);

    assert!(doc
        .select("#before-origin + #origin + #after-origin + #after-after-origin")
        .exists());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_remove_descendants() {
    // The purpose of this test is to ensure that there is no BorrowMutError
    // during iteration through descendants, if `descendants()` is used
    let doc = Document::from(ANCESTORS_CONTENTS);

    let body_sel = doc.select_single("body");
    let body_node = body_sel.nodes().first().unwrap();

    for (i, node) in body_node.descendants().iter().enumerate() {
        // Modifying descendant elements during iteration.
        node.update(|n| {
            n.as_element_mut()
                .map(|el| el.set_attr("data-descendant", &i.to_string()))
        });
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn test_node_remove_descendants_it_panic() {
    // The purpose of this test is to ensure that there is a BorrowMutError
    // during iteration through descendants, if `descendants_it()` is used.

    // This can be resolved at any time by borrowing from `RefCell`
    // on each iteration, but this will be a little bit slower.
    let doc = Document::from(ANCESTORS_CONTENTS);

    let body_sel = doc.select_single("body");
    let body_node = body_sel.nodes().first().unwrap();

    for (i, node) in body_node.descendants_it().enumerate() {
        // Modifying descendant elements during iteration.
        node.update(|n| {
            n.as_element_mut()
                .map(|el| el.set_attr("data-descendant", &i.to_string()))
        });
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_node_normalize() {
    let doc = Document::from(ANCESTORS_CONTENTS);

    let first_child_sel = doc.select_single("#first-child");
    let first_child = first_child_sel.nodes().first().unwrap();

    assert_eq!(first_child.children_it(false).count(), 1);

    let empty_text = doc.tree.new_text("");
    let add_text = doc.tree.new_text(" and a tail");
    first_child.append_child(&empty_text);
    first_child.append_child(&add_text);

    assert_eq!(first_child.children_it(false).count(), 3);
    doc.normalize();

    assert_eq!(first_child.children_it(false).count(), 1);

    let grand_sel = doc.select_single("#grand-parent-sibling");
    let grand_node = grand_sel.nodes().first().unwrap();
    assert_eq!(grand_node.children_it(false).count(), 0);

    let total_empty_text_nodes = 5;

    for _ in 0..total_empty_text_nodes {
        let empty_text = doc.tree.new_text("");
        grand_node.append_child(&empty_text);
    }

    assert_eq!(
        grand_node.children_it(false).count(),
        total_empty_text_nodes
    );

    grand_node.normalize();
    assert_eq!(grand_node.children_it(false).count(), 0);
}
