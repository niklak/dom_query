mod data;

use data::{
    doc_with_siblings, EMPTY_BLOCKS_CONTENTS, REPLACEMENT_CONTENTS, REPLACEMENT_SEL_CONTENTS,
};
use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_with_html() {
    let doc = doc_with_siblings();
    let sel = doc.select("#main,#foot");
    sel.replace_with_html(r#"<div class="replace"></div>"#);

    assert_eq!(doc.select(".replace").length(), 2);
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

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_html_multiple() {
    let doc: Document = EMPTY_BLOCKS_CONTENTS.into();
    let q = doc.select("#main div");

    q.append_html(r#"<p class="text">Follow <a href="https://example.com">example.com</a></p>"#);

    assert_eq!(
        doc.select(r#" #main > div > p > a[href="https://example.com"]:has-text("example.com")"#)
            .length(),
        2
    )
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_html_multiple_elements_to_multiple() {
    let doc: Document = EMPTY_BLOCKS_CONTENTS.into();
    let q = doc.select("#main div");

    q.append_html(r#"<span>1</span><span>2</span>"#);

    assert_eq!(doc.select(r#"div span"#).length(), 4)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_html_multiple_elements_to_multiple() {
    let doc: Document = EMPTY_BLOCKS_CONTENTS.into();
    let sel = doc.select("#main div");

    sel.replace_with_html(r#"<p>1</p><p>2</p>"#);

    assert_eq!(doc.select(r#"#main > p:has-text("1")"#).length(), 2);
    assert_eq!(doc.select(r#"#main > p:has-text("2")"#).length(), 2);
    assert_eq!(doc.select(r#"#main > p"#).length(), 4)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_prepend_html_multiple_elements_to_multiple() {
    let doc: Document = EMPTY_BLOCKS_CONTENTS.into();
    let sel = doc.select("#main div");

    // you may prepend html fragment with one element inside,
    sel.prepend_html(r#"<span class="third">3</span>"#);
    // or more
    sel.prepend_html(r#"<span class="first">1</span><span class="second">2</span>"#);

    assert_eq!(doc.select(r#"div > .first + .second + .third"#).length(), 2)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_with_selection() {
    let doc = Document::from(REPLACEMENT_SEL_CONTENTS);

    let sel_dst = doc.select(".ad-content p span");
    let sel_src = doc.select("span.source");

    sel_dst.replace_with_selection(&sel_src);
    assert_eq!(doc.select(".ad-content .source").length(), 2)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_selection_multiple() {
    let doc = Document::from(REPLACEMENT_SEL_CONTENTS);

    let sel_dst = doc.select(".ad-content p");
    let sel_src = doc.select("span.source");

    // sel_src will be detached from it's tree
    sel_dst.append_selection(&sel_src);
    assert_eq!(doc.select(".ad-content .source").length(), 2);
    assert_eq!(doc.select(".ad-content span").length(), 4)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_with_another_tree_selection() {
    let doc_dst = Document::from(REPLACEMENT_SEL_CONTENTS);

    let contents_src = r#"
    <span class="source">example</span>
    <span class="source">example</span>"#;

    let doc_src = Document::from(contents_src);

    let sel_dst = doc_dst.select(".ad-content p span");
    let sel_src = doc_src.select("span.source");

    sel_dst.replace_with_selection(&sel_src);
    assert_eq!(doc_dst.select(".ad-content .source").length(), 4)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_another_tree_selection() {
    let doc_dst = Document::from(REPLACEMENT_SEL_CONTENTS);

    let contents_src = r#"
    <span class="source">example</span>
    <span class="source">example</span>"#;

    let doc_src = Document::from(contents_src);

    let sel_dst = doc_dst.select(".ad-content p");
    let sel_src = doc_src.select("span.source");

    sel_dst.append_selection(&sel_src);
    assert_eq!(doc_dst.select(".ad-content .source").length(), 4);
    assert_eq!(doc_dst.select(".ad-content span").length(), 6)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_append_another_tree_selection_empty() {
    let doc_dst = Document::from(REPLACEMENT_SEL_CONTENTS);

    let contents_src = r#"
    <span class="source">example</span>
    <span class="source">example</span>"#;

    let doc_src = Document::from(contents_src);

    let sel_dst = doc_dst.select(".ad-content p");

    // selecting non-existing elements
    let sel_src = doc_src.select("span.src");
    assert!(!sel_src.exists());

    // sel_dst remained without changes
    sel_dst.append_selection(&sel_src);
    assert_eq!(doc_dst.select(".ad-content span").length(), 2)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_replace_with_another_tree_selection_empty() {
    let doc_dst = Document::from(REPLACEMENT_SEL_CONTENTS);

    let contents_src = r#"
    <span class="source">example</span>
    <span class="source">example</span>"#;

    let doc_src = Document::from(contents_src);

    let sel_dst = doc_dst.select(".ad-content p span");
    // selecting non-existing elements
    let sel_src = doc_src.select("span.src");
    assert!(!sel_src.exists());
    sel_dst.replace_with_selection(&sel_src);
    // sel_dst remained without changes
    assert_eq!(doc_dst.select(".ad-content span").length(), 2)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_rename_selection() {
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
fn test_selection_set_text() {
    let doc: Document = REPLACEMENT_CONTENTS.into();
    let sel = doc.select("div > p");
    sel.set_text("New Text");
    // expecting 3 paragraphs with having new text
    assert_eq!(doc.select(r#"p:has-text("New Text")"#).length(), 3);

    // nothing is found, so nothing is changed
    let sel = doc.select("div > p > span");
    sel.set_text("New Inline Text");
    assert_eq!(doc.select(r#"p:has-text("New Inline Text")"#).length(), 0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_before_html() {
    let doc: Document = REPLACEMENT_CONTENTS.into();
    let sel = doc.select("#main > p");

    // inserting a thematic break and a simple break before each paragraph
    sel.before_html(r#"<hr><br>"#);
    assert_eq!(doc.select(r#"#main > hr + br + p"#).length(), 3)
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_after_html() {
    let doc: Document = REPLACEMENT_CONTENTS.into();
    let sel = doc.select("#main > p");

    // inserting two br elements after each paragraph
    sel.after_html(r#"<br><br>"#);
    assert_eq!(doc.select(r#"#main > p + br + br"#).length(), 3)
}
