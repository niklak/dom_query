use dom_query::Document;

fn main() {
    let html = include_str!("../test-pages/hacker_news.html");
    let document = Document::from(html);

    document.select("tr.athing:has(a[href][id])").iter().for_each(|athing| {
        let title = athing.select(".title a");
        let href = athing.select(".storylink");
        println!("{}", title.text());
        println!("{}", href.attr("href").unwrap());
        println!();
    });
}
