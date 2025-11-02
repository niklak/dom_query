use dom_query::{Document, Matcher};

#[test]
fn test_selection() {
    let contents = include_str!("../test-pages/hacker_news.html");
    let doc = Document::from(contents);

    let links = doc.select("body td.title a[href]");
    let base_link_count = links.nodes().len();
    let mut count: usize;

    let matcher = Matcher::new(r"body td.title a[href]").unwrap();

    let links = doc.select_matcher(&matcher);
    count = links.nodes().len();
    assert_eq!(base_link_count, count);

    let body = dom_query::Selection::from(doc.body().unwrap());
    count = body.select_matcher_iter(&matcher).count();
    assert_eq!(base_link_count, count);

    let links = doc.select_single("body").select("td.title").select("a[href]");
    count = links.nodes().len();
    assert_eq!(base_link_count, count);

    let root = doc.root();
    let links = root.find(&["body", "td", "a"]);
    count = links.len();
    assert_eq!(base_link_count, count);

    let root = doc.root();
    let links = root.find_descendants(r#"body td.title a[href]"#);
    count = links.len();
    assert_eq!(base_link_count, count);
}