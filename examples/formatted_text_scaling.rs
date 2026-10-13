use std::time::Instant;

use dom_query::Document;

fn main() {
    let unit = "<div><p>Some <b>bold</b> text and a <a href=\"/x\">link</a> here </p>\
                <ul><li>one </li><li>two</li></ul></div>\n";
    for k in [1, 2, 4, 8, 16] {
        let html = format!("<html><body>{}</body></html>", unit.repeat(2000 * k));
        let doc = Document::from(html.as_str());

        let t = Instant::now();
        let formatted = doc.formatted_text();
        let formatted_time = t.elapsed();

        let t = Instant::now();
        let plain = doc.text();
        let plain_time = t.elapsed();

        println!(
            "{:>5} KB html: formatted_text {:>10.1?} ({} bytes out), text {:>8.1?} ({} bytes out)",
            html.len() / 1024,
            formatted_time,
            formatted.len(),
            plain_time,
            plain.len()
        );
    }
}
