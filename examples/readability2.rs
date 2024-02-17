use readability::extractor::extract;
use std::time::Instant;

use std::env;
use std::error::Error;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let url = env::args().nth(1).unwrap();

    let html: String = ureq::get(&url).call()?.into_string()?;
    let url = &url.parse()?;
    let mut c = Cursor::new(html.as_bytes());

    let article = extract(&mut c, url)?;

    println!("title   ====> {}", article.title);
    println!("article ====> {}", article.content);
    println!("{:?}", start.elapsed());
    Ok(())
}
