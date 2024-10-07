use dom_query::Document;

fn main() {
    let html_contents = r#"<!DOCTYPE html>
    <html>
        <head><title>Test</title></head>
        <body>
            <div class="content">
                <p>9,8,7</p>
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

    // Add a new html block to the selection
    let mut content_selection = doc.select("body .content");
    content_selection.append_html(r#"<div class="inner">inner block</div>"#);
    
    println!("{}", doc.html());
    println!("{:-<50}", "");

    // Delete all child nodes of a selection and replace with a new html block
    content_selection.set_html(r#"<div class="inner">1,2,3</div>"#);
    println!("{}", doc.html());
    println!("{:-<50}", "");

    // Remove selection from the document
    doc.select(".remove-it").remove();

    println!("{}", doc.html());
    println!("{:-<50}", "");
    
    // Replacing inner block content with new content, current selection remains the same
    let mut replace_selection = doc.select(".inner");
    replace_selection.replace_with_html(r#"<div class="replaced">Replaced</div>"#);

    assert_eq!(replace_selection.text(), "1,2,3".into());

    println!("{}", doc.html());
    println!("{:-<50}", "");


}
