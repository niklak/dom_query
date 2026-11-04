use dom_query::Document;
use html5ever::{QualName, local_name, ns};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

const SIMPLE_LIST_CONTENT: &str = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
           <ul class="list">
               <li>1</li>
               <li>2</li>
               <li>3</li>
           </ul>
           <ul>
               <li>4</li>
            </ul>
        </body>
    </html>"#;

const LINKS_CONTENT: &str = r#"<!DOCTYPE html>
    <html lang="en">
        <head><title>Test</title></head>
        <body>
        <div>
            <a class="link first-link" href="/1">One</a>
            <a class="link second-link" href="/2">Two</a>
            <a class="link" href="/3"><span>Three</span></a>
        </div>
        </body>
    </html>"#;

const EMPTY_HEADINGS_CONTENT: &str = r"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
           <h1>
           </h1>
           <h2>
           </h2>
           <h3>
           </h3>
           <h1>1</h1>
           <h2>2</h2>
           <h3>3</h3>
        </body>
    </html>
    ";

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_has() {
    let document = Document::from(LINKS_CONTENT);

    let sel = r"div:has(a[href]) a span";
    let span = document.select(sel);

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_has_any_link() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r"div:has(*:any-link) a span";
    let span = document.select(sel).first();

    let text: &str = &span.text();
    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_link() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r"div *:link";
    let span = document.select(sel).first();

    let text: &str = &span.text();
    assert_eq!(text, "One");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn pseudo_class_has_bad() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r"div:hasa(*:any-link) a span";
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
fn pseudo_class_only_text() {
    //! :only-text pseudo-class allows selecting a node with no child elements except a single **text** child node.

    let html = r#"<!DOCTYPE html>
    <html>
        <head>
            <title>Test</title>
        </head>
        <body>
            <div>
                <a href="/1">One</a>
                <a href="/2">Two</a>
                <a href="/3">Three</a>
            </div>
            <div>
            </div>
            <div>Only text</div>
        </body>
    </html>
    "#;
    let document = Document::from(html);
    let sel = document.select("body div:only-text");
    // this selector must ignore empty elements and elements that contains only whitespace
    // in this example the previous node of div with `Only text` is not empty,
    // it contains whitespace characters, so it will be ignored
    assert_eq!(sel.length(), 1);
    assert_eq!(&sel.inner_html(), "Only text");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pseudo_class_not() {
    let document = Document::from(LINKS_CONTENT);
    let sel = r"div a[class]:not(.first-link,.second-link)";
    let span = document.select(sel).first();
    let text: &str = &span.text();

    assert_eq!(text, "Three");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_is() {
    //! select h1, h2, h3 elements if they are empty
    let doc: Document = EMPTY_HEADINGS_CONTENT.into();
    let is_sel = doc.select(":is(h1,h2,h3):empty");
    assert_eq!(is_sel.length(), 3);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_where() {
    //! select h1, h2, h3 elements if they are empty
    let doc: Document = EMPTY_HEADINGS_CONTENT.into();
    let where_sel = doc.select(":where(h1,h2,h3):empty");
    assert_eq!(where_sel.length(), 3);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_empty() {
    //! select all empty elements inside `body` element
    let doc: Document = EMPTY_HEADINGS_CONTENT.into();
    let where_sel = doc.select("body :empty");
    assert_eq!(where_sel.length(), 3);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_root() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();
    let root = doc.select(":root");
    assert_eq!(root.length(), 1);
    assert!(root.is("html"));
    assert_eq!(doc.select(":root > body").length(), 1);
    assert_eq!(
        doc.select(":not(:root)").length(),
        doc.select("*").length() - 1
    );
    // The document node is not an element, so `<html>` has no parent element.
    assert_eq!(doc.select(":root > html").length(), 0);
    assert_eq!(doc.select("* > html").length(), 0);
    assert_eq!(doc.select(":not(:root) > html").length(), 0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_nth_last_child() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul.list li:nth-last-child(1)");
    assert_eq!(&sel.text(), "3");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_nth_child() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul.list li:nth-child(1)");
    assert_eq!(&sel.text(), "1");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_first_child() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul.list li:first-child");
    assert_eq!(&sel.text(), "1");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_last_child() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul.list li:last-child");
    assert_eq!(&sel.text(), "3");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_last_of_type() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul.list li:last-of-type");
    assert_eq!(&sel.text(), "3");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_only() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul li:only-child");
    assert_eq!(&sel.text(), "4");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_only_of_type() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.select("ul li:only-of-type");
    assert_eq!(&sel.text(), "4");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn test_unsupported_pseudo_class() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    doc.select("*:required");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_try_unsupported_pseudo_class() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    let sel = doc.try_select("*:required");

    assert!(sel.is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn test_unsupported_functional_pseudo_class() {
    let doc: Document = SIMPLE_LIST_CONTENT.into();

    doc.select(":lang(en-US)");
}

fn ids(doc: &Document, sel: &str) -> Vec<String> {
    doc.select(sel)
        .iter()
        .map(|s| s.attr_or("id", "").to_string())
        .collect()
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_checked() {
    let doc = Document::from(
        r#"<input id="c1" type="checkbox" checked>
        <input id="c2" type="CheckBox" checked>
        <input id="r1" type="radio" checked>
        <input id="c3" type="checkbox">
        <input id="t1" type="text" checked>
        <input id="t2" checked>
        <select><option id="o1" selected>a</option><option id="o2">b</option></select>
        <svg><input id="s1" type="checkbox" checked></input></svg>"#,
    );

    assert_eq!(ids(&doc, ":checked"), ["c1", "c2", "r1", "o1"]);
    assert_eq!(
        ids(&doc, "input:not(:checked), option:not(:checked)"),
        ["c3", "t1", "t2", "o2", "s1"]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_disabled() {
    let doc = Document::from(
        r#"<button id="b1" disabled></button><button id="b2"></button>
        <input id="i1" disabled><textarea id="t1" disabled></textarea>
        <select id="s1"><optgroup id="g1" disabled><option id="o1"></option></optgroup>
        <optgroup id="g2"><option id="o2" disabled></option><option id="o3"></option></optgroup>
        </select>
        <select id="s2" disabled><datalist><option id="o4"></option></datalist><option id="o5"><span><option id="o6"></option></span></option></select>
        <div id="d1" disabled></div>
        <svg><input id="v1" disabled></input></svg>"#,
    );

    assert_eq!(
        ids(&doc, ":disabled"),
        ["b1", "i1", "t1", "g1", "o1", "o2", "s2", "o5"]
    );
    assert_eq!(ids(&doc, ":enabled"), ["b2", "s1", "g2", "o3", "o4", "o6"]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_disabled_fieldset() {
    let doc = Document::from(
        r#"<fieldset id="f1" disabled>
            <div></div>
            <legend><input id="l1"></legend>
            <legend><input id="l2"></legend>
            <div><select id="s1"></select></div>
            <fieldset id="f2"><legend><input id="l3"></legend></fieldset>
        </fieldset>
        <fieldset id="f3"><legend><fieldset id="f4" disabled><input id="i1"></fieldset></legend></fieldset>
        <svg><fieldset disabled><foreignObject><input id="i2"></foreignObject></fieldset></svg>"#,
    );

    assert_eq!(
        ids(&doc, ":disabled"),
        ["f1", "l2", "s1", "f2", "l3", "f4", "i1"]
    );
    assert_eq!(ids(&doc, ":enabled"), ["l1", "f3", "i2"]);
    assert_eq!(ids(&doc, "input:not(:disabled)"), ["l1", "i2"]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_disabled_fieldset_deep() {
    const DEPTH: usize = 50_000;
    let doc = Document::from("<fieldset disabled></fieldset>");
    let mut current = doc.select("fieldset").nodes()[0];
    for _ in 0..DEPTH {
        let div = doc.tree.new_element("div");
        doc.tree.append_child_of(&current.id, &div.id);
        current = div;
    }
    current.append_child(&doc.tree.new_element("input"));

    assert_eq!(doc.select("input:disabled").length(), 1);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_disabled_select() {
    let doc = Document::from(
        r#"<select id="s1" disabled><option id="o1"></option><optgroup id="g1"><option id="o2"></option></optgroup></select>
        <fieldset id="f1" disabled><select id="s2"><option id="o3"></option></select></fieldset>
        <select id="s3"><optgroup id="g2" disabled><div><option id="o4"></option></div></optgroup></select>
        <optgroup id="g3"><svg><foreignObject><option id="o5"></option></foreignObject></svg></optgroup>"#,
    );

    assert_eq!(
        ids(&doc, ":disabled"),
        ["s1", "o1", "g1", "o2", "f1", "s2", "o3", "g2", "o4"]
    );
    assert_eq!(ids(&doc, ":enabled"), ["s3", "g3", "o5"]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_disabled_select_nested_optgroups() {
    // `o1` has two optgroup ancestors, so it has no nearest select to inherit `disabled` from.
    let doc = Document::from("<select disabled></select>");
    let select = doc.select("select").nodes()[0];
    // `o2` is cut off from the select by an `hr`; an svg `hr` doesn't cut `o3` off.
    for (ns, id) in [(ns!(html), "o2"), (ns!(svg), "o3")] {
        let hr = doc
            .tree
            .new_element_qualname(QualName::new(None, ns, local_name!("hr")));
        let option = doc.tree.new_element("option");
        option.set_attr("id", id);
        hr.append_child(&option);
        select.append_child(&hr);
    }
    let mut current = select;
    for id in ["g1", "g2", "o1"] {
        let el = doc
            .tree
            .new_element(if id == "o1" { "option" } else { "optgroup" });
        el.set_attr("id", id);
        current.append_child(&el);
        current = el;
    }

    assert_eq!(
        ids(&doc, "optgroup:disabled, option:disabled"),
        ["o3", "g1", "g2"]
    );
    assert_eq!(ids(&doc, ":enabled"), ["o2", "o1"]);
}
