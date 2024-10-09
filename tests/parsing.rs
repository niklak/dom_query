use dom_query::Document;
use tendril::StrTendril;


const HTML_CONTENTS: &str = r#"<!DOCTYPE html>
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

#[test]
fn parse_doc_str() {
    let doc = Document::from(HTML_CONTENTS);
    assert!(doc.root().is_document());
    // document has a <!DOCTYPE>
    let doc_type_el = doc.root().first_child().unwrap();
    assert!(doc_type_el.is_doctype());
}

#[test]
fn parse_fragment_str() {
    let fragment = Document::fragment(HTML_CONTENTS);
    assert!(fragment.root().is_fragment());
    // <!DOCTYPE> is dropped in fragments
    assert!(!fragment.root().first_child().unwrap().is_doctype());
    let element_name = fragment.root().first_child().unwrap().node_name().unwrap();
    assert_eq!(element_name, "html".into());
}

#[test]
fn parse_doc_string() {
    let contents = String::from(HTML_CONTENTS);
    let doc = Document::from(contents);
    assert!(doc.root().is_document());
}

#[test]
fn parse_fragment_string() {
    let contents = String::from(HTML_CONTENTS);
    let fragment = Document::fragment(contents);
    assert!(!fragment.root().first_child().unwrap().is_doctype());
}

#[test]
fn parse_doc_str_tendril() {
    let contents = StrTendril::from(HTML_CONTENTS);
    let doc = Document::from(contents);
    assert!(doc.root().is_document());
}

#[test]
fn parse_fragment_str_tendril() {
    let contents = StrTendril::from(HTML_CONTENTS);
    let fragment = Document::fragment(contents);
    assert!(!fragment.root().first_child().unwrap().is_doctype());
}