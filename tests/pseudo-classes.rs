use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

const SIMPLE_DIVS_CONTENT: &str = r#"<!DOCTYPE html>
    <html>
        <head>Test</head>
        <body>
           <div>1</div>
           <div>2</div>
           <div>3</div>
        </body>
    </html>"#;

const LINKS_CONTENT: &str = r#"<!DOCTYPE html>
    <html lang="en">
        <head><title>Test</title></head>
        <body>
        <div>
            <a class="link first-link" href="/1">One</a>
            <a class="link" href="/2">Two</a>
            <a class="link" href="/3"><span>Three</span></a>
        </div>
        </body>
    </html>"#;

const EMPTY_HEADINGS_CONTENT: &str = r#"<!DOCTYPE html>
    <html>
        <head>Test</head>
        <body>
           <h1></h1>
           <h2></h2>
           <h3></h3>
           <h1>1</h1>
           <h2>2</h2>
           <h3>3</h3>
        </body>
    </html>
    "#;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_has() {
    let document = Document::from(LINKS_CONTENT);

    let sel = r#"div:has(a[href]) a span"#;
    let span = document.select(sel);

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_has_any_link() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r#"div:has(*:any-link) a span"#;
    let span = document.select(sel).first();

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn pseudo_class_has_bad() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r#"div:hasa(*:any-link) a span"#;
    let span = document.select(sel);

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_contains_text() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r#"div a:has-text("Three")"#;
    let span = document.select(sel);

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn pseudo_class_has_text_fail() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3">It is not <span>how</span> it works</a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div a:has-text("how it works")"#;
    // it is not going to find anything,
    // because it is searching in the each node's text and not in the final text.
    // The last element `a` contains three nodes:
    // `text node ("It is not "), element node ("how") and text node (" it works")`
    let span = document.select(sel).first();

    let text: &str = &span.text();

    assert_eq!(text, "It is not how it works");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_contains() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3">It is not <span>how</span> it works</a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div a:contains("how it works")"#;
    let span = document.select(sel);
    // And `:contains` will match the last `a` element,
    // because it searches in the merged text of the element and its descendants.
    let text: &str = &span.text();

    assert_eq!(text, "It is not how it works");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_not() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r#"div a.link:not(.first-link)"#;
    let span = document.select(sel).first();
    let text: &str = &span.text();

    assert_eq!(text, "Two");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is() {
    //! select 3 empty elements
    let doc: Document = EMPTY_HEADINGS_CONTENT.into();
    let is_sel = doc.select(":is(h1,h2,h3) :empty");
    assert_eq!(is_sel.length(), 3);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_where() {
    //! select 3 empty elements
    let doc: Document = EMPTY_HEADINGS_CONTENT.into();
    let is_sel = doc.select(":where(h1,h2,h3) :empty");
    assert_eq!(is_sel.length(), 3);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_nth_last_child() {
    let doc: Document = SIMPLE_DIVS_CONTENT.into();

    let sel = doc.select("body div:nth-last-child(1)");
    assert_eq!(sel.text(), "3".into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_first_child() {
    let doc: Document = SIMPLE_DIVS_CONTENT.into();

    let sel = doc.select("body div:first-child");
    assert_eq!(sel.text(), "1".into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_last_child() {
    let doc: Document = SIMPLE_DIVS_CONTENT.into();

    let sel = doc.select("body div:last-child");
    assert_eq!(sel.text(), "3".into());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_last_of_type() {
    let doc: Document = SIMPLE_DIVS_CONTENT.into();

    let sel = doc.select("body div:last-of-type");
    assert_eq!(sel.text(), "3".into());
}
