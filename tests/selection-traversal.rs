mod data;

use data::doc;
use data::doc_wiki;
use dom_query::Document;

const DOC_WITH_LISTS: &str = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
        <body>
            <div>
                <ul class="list">
                    <li>1</li><li>2</li><li>3</li>
                </ul>
                <ul class="list">
                    <li>4</li><li>5</li><li>6</li>
                </ul>
            <div>
        </body>
    </html>"#;

#[test]
fn test_select() {
    let doc = doc();
    let sel = doc.select("div.row-fluid");
    assert_eq!(sel.length(), 9);
}

#[test]
fn test_select_not_self() {
    let doc = doc();
    let sel = doc.select("h1").select("h1");
    assert_eq!(sel.length(), 0);
}

#[test]
#[should_panic]
fn test_select_invalid() {
    let doc = doc();
    let sel = doc.select(":+ ^");
    assert_eq!(sel.length(), 0);
}

#[test]
fn test_select_big() {
    let doc = doc_wiki();
    let sel = doc.select("li");
    assert_eq!(sel.length(), 420);
    let sel = doc.select("span");
    assert_eq!(sel.length(), 706);
}

#[test]
fn test_chained_select() {
    let doc = doc();
    let sel = doc.select("div.hero-unit").select(".row-fluid");
    assert_eq!(sel.length(), 4);
}

#[test]
#[should_panic]
fn test_chained_select_invalid() {
    let doc = doc();
    let sel = doc.select("div.hero-unit").select("");
    assert_eq!(sel.length(), 0);
}

#[test]
fn test_children() {
    let doc = doc();
    let sel = doc.select(".pvk-content").children();
    assert_eq!(sel.length(), 5)
}

#[test]
fn test_parent() {
    let doc = doc();
    let sel = doc.select(".container-fluid").parent();
    assert_eq!(sel.length(), 3)
}

#[test]
fn test_parent_body() {
    let doc = doc();
    let sel = doc.select("body").parent();
    assert_eq!(sel.length(), 1)
}

#[test]
fn test_next() {
    let doc = doc();
    let sel = doc.select("h1").next_sibling();
    assert_eq!(sel.length(), 1)
}

#[test]
fn test_next2() {
    let doc = doc();
    let sel = doc.select(".close").next_sibling();
    assert_eq!(sel.length(), 1)
}

#[test]
fn test_next_none() {
    let doc = doc();
    let sel = doc.select("small").next_sibling();
    assert_eq!(sel.length(), 0)
}

#[test]
fn test_nth_child() {
    let doc: Document = r#"<!DOCTYPE html>
    <html lang="en">
        <head></head>
    
        <body>
            <div id="bggrad"></div>
            <div class="container container-header"></div>
            <div class="container container-main">
                <nav class="navbar navbar-default navbar-static-top"></nav>
                <div class="row">
                    <div class="col-xs-12"></div>
                    <div class="col-xs-12"></div>
                    <div class="col-md-10">
                        <a href="\#">foo</a>
                    </div>
                </div>
            </div>
        </body>
    </html>"#
        .into();

    let a = doc
        .select("body > div.container.container-main > div.row:nth-child(2) > div.col-md-10 > a");

    assert!(a.length() == 1);
}

#[test]
fn test_doc_select_single() {
    let doc: Document = DOC_WITH_LISTS.into();

    let single_selection_count = doc.select_single(".list").length();
    assert_eq!(single_selection_count, 1);

    let multiple_selection_count = doc.select(".list").length();
    assert_eq!(multiple_selection_count, 2);
}
#[test]
fn test_select_single() {
    let doc: Document = DOC_WITH_LISTS.into();

    let single_selection_count = doc.select("div").select_single(".list").length();
    assert_eq!(single_selection_count, 1);

    let multiple_selection_count = doc.select("div").select(".list").length();
    assert_eq!(multiple_selection_count, 2);
}

#[test]
fn test_handle_selection() {
    let doc: Document = DOC_WITH_LISTS.into();

    let all_matched: String = doc
        .select(".list")
        .iter()
        .map(|s| s.inner_html().trim().to_string())
        .collect();

    assert_eq!(
        all_matched,
        "<li>1</li><li>2</li><li>3</li><li>4</li><li>5</li><li>6</li>"
    );
}
