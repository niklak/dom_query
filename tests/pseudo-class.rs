use dom_query::Document;

#[test]
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
    assert!(text == "Three");
}

#[test]
fn pseudo_class_has_any_link() {
    let html = r#"
    <div>
        <a href="/1">One</a>
        <a href="/2">Two</a>
        <a href="/3"><span>Three</span></a>
    </div>"#;
    let document = Document::from(html);
    let sel = r#"div:has(*:any-link) a span"#;
    let span = document.select(sel);

    let text: &str = &span.text();
    assert!(text == "Three");
}

#[test]
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
    assert!(text == "Three");
}

#[test]
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
    assert!(text == "Three");
}

#[test]
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
    // it is not going to find anything, because it is searching in the each node's text and not in the final text.
    // The last element `a` contains three nodes: 
    // `text node ("It is not "), element node ("how") and text node (" it works")`
    let span = document.select(sel);

    let text: &str = &span.text();

    assert!(text == "It is not how it works");
}
