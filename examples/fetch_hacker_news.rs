use std::error::Error;

use dom_query::Document;

fn main() -> Result<(), Box<dyn Error>> {
    let html = ureq::get("https://news.ycombinator.com/news")
        .call()?
        .into_string()?;

    let document = Document::from(html.as_str());

    for news in document.select("tr.athing:has(a[href][id])").iter() {
        let link = news.select(".title span.titleline > a");
        let source = news.select(".sitebit a");
        println!("{:<6} => {}", "title", link.text());
        println!("{:<6} => {}", "link", link.attr("href").unwrap_or_default());
        println!("{:<6} => {}\n", "source", source.text());
    }
    Ok(())
}
