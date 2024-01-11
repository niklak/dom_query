use readability::extractor::extract;
use std::time::Instant;

use std::env;
use std::io::Cursor;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let url = env::args().skip(1).next().unwrap();

    let html: String = ureq::get(&url)
        .call()?
        .into_string()?;
    let url = &url.parse()?;
    let mut c = Cursor::new(html.as_bytes());

    let article = extract(&mut c, &url)?;

    println!("title   ====> {}", article.title);
    println!("article ====> {}", article.content);
    println!("{:?}", start.elapsed());
    Ok(())
}
