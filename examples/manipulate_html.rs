use dom_query::Document;

fn main() {
    let html_contents = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="content">
                <ul class="list-a">
                    <li>1</li>
                    <li>2</li>
                    <li>3</li>
                    <li>4</li>
                </ul>
                <div class="other"></div>
            </div
        </body>
    </html>"#;

    let doc = Document::from(html_contents);

    // Initial state
    println!("{}", doc.html());
    println!("{:-<50}", "");

    let mut content_selection = doc.select(".content");
    content_selection.append_html(r#"<div class="inner">inner block</div>"#);

    // After adding a new block to the content block
    println!("{}", doc.html());
    println!("{:-<50}", "");

    // Replacing inner block content with new content, current selection remains the same
    let mut inner_selection = doc.select(".other");
    inner_selection.replace_with_html(r#"<div class="inner">INNER BLOCK</div>"#);

   // assert_eq!(inner_selection.text(), "inner block".into());

    println!("{}", doc.html());
    println!("{:-<50}", "");


}
