mod data;

use data::doc_with_siblings;
use dom_query::Document;

#[test]
fn test_replace_with_html() {
    let doc = doc_with_siblings();

    println!("{}", doc.html());

    let sel = doc.select("#main,#foot");
    println!("{}", sel.length());
    sel.replace_with_html(r#"<div id="replace"></div>"#);

    println!("{}", doc.html());
    println!("======");

    assert_eq!(doc.select("#replace").length(), 2);
}

#[test]
fn test_set_html() {
    let doc = doc_with_siblings();
    let q = doc.select("#main, #foot");
    q.set_html(r#"<div id="replace">test</div>"#);

    assert_eq!(doc.select("#replace").length(), 2);
    assert_eq!(doc.select("#main, #foot").length(), 2);

    let html: &str = &q.text();
    assert_eq!(html, "testtest");
}

#[test]
fn test_set_html_no_match() {
    let doc = doc_with_siblings();
    let q = doc.select("#notthere");
    q.set_html(r#"<div id="replace">test</div>"#);
    assert_eq!(doc.select("#replace").length(), 0);
}

#[test]
fn test_set_html_empty() {
    let doc = doc_with_siblings();
    let q = doc.select("#main");
    q.set_html("");
    assert_eq!(doc.select("#main").length(), 1);
    assert_eq!(doc.select("#main").children().length(), 0);
}

#[test]
fn test_replace_with_selection() {
    let doc = doc_with_siblings();

    let s1 = doc.select("#nf5");
    let sel = doc.select("#nf6");

    sel.replace_with_selection(&s1);

    assert!(sel.is("#nf6"));
    assert_eq!(doc.select("#nf6").length(), 0);
    assert_eq!(doc.select("#nf5").length(), 1);
}

#[test]
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

#[test]
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

#[test]
fn test_set_element_html() {
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
    main_node.set_html(r#"<p id="second">Wonderful</p>"#);
    assert_eq!(doc.select("#main #second").text().as_ref(), "Wonderful");
    assert!(!doc.select("#first").exists());
}