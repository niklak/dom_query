use dom_query::Document;
use std::error::Error;
use std::time::Instant;
use ureq;
fn main() -> Result<(), Box<dyn Error>> {
    let html: String = ureq::get("https://news.ycombinator.com/news")
        .call()?
        .into_string()?;
    let start = Instant::now();
    let document = Document::from(&html);

    for news in document.select("tr.athing").iter() {
        let link = news.select(".title span.titleline > a");
        let source = news.select(".sitebit a");
        println!("title => {}", link.text());
        println!("link => {}", link.attr("href").unwrap_or_default());
        println!("source => {}", source.text())
    }

    println!("{:?}", start.elapsed());
    Ok(())
}
