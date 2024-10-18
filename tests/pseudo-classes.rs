use dom_query::Document;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_has() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3"><span>Three</span></a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div:has(a[href="/1"]) a span"#;
    let span = document.select(sel);

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_has_any_link() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3"><span>Three</span></a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div:has(*:any-link) a span"#;
    let span = document.select(sel).first();

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn pseudo_class_has_bad() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3"><span>Three</span></a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div:hasa(*:any-link) a span"#;
    let span = document.select(sel);

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_contains_text() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3"><span>Three</span></a>
    </div>"#;
    let document = Document::from(html);
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
    let html = r#"
    <div>
        <a class="link first-link" href="/1">One</a>
        <a class="link" href="/2">Two</a>
        <a class="link" href="/3">Three</a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div a.link:not(.first-link)"#;
    let span = document.select(sel).first();
    let text: &str = &span.text();

    assert_eq!(text, "Two");
}
