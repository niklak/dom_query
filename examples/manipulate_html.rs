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
                </ul>
            </div>
            <div class="replace-it">
                Replace me
            </div>
            <div class="remove-it">
                Remove me
            </div>
        </body>
    </html>"#;

    let doc = Document::from(html_contents);

    // Initial state
    println!("{}", doc.html());
    println!("{:-<50}", "");

    let mut content_selection = doc.select("body .content");
    content_selection.append_html(r#"<div class="inner">inner block</div>"#);

    // After adding a new block to the content block
    println!("{}", doc.html());
    println!("{:-<50}", "");


    content_selection.set_html(r#"<p>1,2,3</p>"#);
    // After setting the content the new content
    println!("{}", doc.html());
    println!("{:-<50}", "");

    doc.select(".remove-it").remove();

    // After removing the remove-it div
    println!("{}", doc.html());
    println!("{:-<50}", "");

    // Replacing inner block content with new content, current selection remains the same
    let mut replace_selection = doc.select(".replace-it");
    replace_selection.replace_with_html(r#"<div class="replaced">Replaced</div>"#);

   // assert_eq!(inner_selection.text(), "inner block".into());

    println!("{}", doc.html());
    println!("{:-<50}", "");


}
