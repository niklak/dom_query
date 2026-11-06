use dom_query::Document;
use html5ever::tree_builder::{NodeOrText, TreeSink};
use tendril::StrTendril;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

mod data;
use data::HEADING_CONTENTS;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_doc_str() {
    let doc = Document::from(HEADING_CONTENTS);
    assert!(doc.root().is_document());
    // document has a <!DOCTYPE html>
    let doc_type_el = doc.root().first_child().unwrap();
    assert!(doc_type_el.is_doctype());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_doc_no_doctype() {
    let contents = r#"
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="content">
                <h1>Test Page</h1>
            </div>
            <div class="content">
                <p>This is a test page contents.</p>
            </div
        </body>
    </html>"#;
    let doc = Document::from(contents);
    assert!(doc.root().is_document());
    // if the source doesn't have a DocType, then the Document also doesn't have one
    let doc_type_el = doc.root().first_child().unwrap();
    assert!(!doc_type_el.is_doctype());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_fragment_str() {
    let fragment = Document::fragment(HEADING_CONTENTS);
    assert!(fragment.root().is_fragment());
    // <!DOCTYPE html> is dropped in fragments
    assert!(!fragment.root().first_child().unwrap().is_doctype());
    let element_name = fragment.root().first_child().unwrap().node_name().unwrap();
    assert_eq!(&element_name, "html");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_doc_string() {
    let contents = String::from(HEADING_CONTENTS);
    let doc = Document::from(contents);
    assert!(doc.root().is_document());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_fragment_string() {
    let contents = String::from(HEADING_CONTENTS);
    let fragment = Document::fragment(contents);
    assert!(!fragment.root().first_child().unwrap().is_doctype());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_doc_str_tendril() {
    let contents = StrTendril::from(HEADING_CONTENTS);
    let doc = Document::from(contents);
    assert!(doc.root().is_document());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_fragment_str_tendril() {
    let contents = StrTendril::from(HEADING_CONTENTS);
    let fragment = Document::fragment(contents);
    assert!(!fragment.root().first_child().unwrap().is_doctype());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_doc_meta_charset() {
    let contents = r#"<!DOCTYPE html>
    <html>
        <head>
            <title>Test</title>
            <meta charset="UTF-8" />
            <link type="text/css" rel="stylesheet" charset="UTF-8" href="/static/translateelement.css" />
            <script type="text/javascript" charset="UTF-8" src="/static/js/element/main.js"></script>
        </head>
        <body>
            <p>This is a test page contents.</p>
        </body>
    </html>"#;
    let doc = Document::from(contents);
    assert!(doc.root().is_document());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_misnested_formatting() {
    // the adoption agency algorithm moves the div out of <b> and its children into a new <b>
    let doc = Document::from("<b><div>x</b>y");
    assert_eq!(
        doc.select("body").inner_html().as_ref(),
        "<b></b><div><b>x</b>y</div>"
    );
    doc.tree.validate().unwrap();
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_foster_parented_text_merge() {
    // text foster-parented out of a table joins the text node before the table
    let doc = Document::from("x<table>y</table>");
    let body = doc.select_single("body").nodes()[0];
    assert_eq!(body.children().len(), 2);
    assert_eq!(body.first_child().unwrap().text().as_ref(), "xy");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_foster_parent_without_table_parent() {
    // only a script detaching the table sends the parser down this path, so drive the sink directly
    let doc = Document::from("<div></div>");
    let div = doc.select_single("div").nodes()[0];
    let table = doc.tree.new_element("table");
    doc.append_based_on_parent_node(&table.id, &div.id, NodeOrText::AppendText("x".into()));
    assert_eq!(div.inner_html().as_ref(), "x");
    assert!(table.first_child().is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn parse_processing_instruction() {
    // the html tokenizer reads `<?` as a comment, so only a direct sink call creates one
    let doc = Document::from("<div></div>");
    let div = doc.select_single("div").nodes()[0];
    let pi = doc.create_pi("xml-stylesheet".into(), "href=\"a.css\"".into());
    div.append_child(&pi);
    assert_eq!(
        div.inner_html().as_ref(),
        "<?xml-stylesheet href=\"a.css\">"
    );
}

#[cfg(feature = "atomic")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn doc_is_send() {
    fn is_send<T: Send>() {}
    is_send::<Document>();
}
