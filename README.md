
# DOM_QUERY

> A crate for manipulating HTML with CSS selectors.

[![Crates.io version](https://img.shields.io/crates/v/dom_query.svg?style=flat)](https://crates.io/crates/dom_query)
[![Download](https://img.shields.io/crates/d/dom_query.svg?style=flat)](https://crates.io/crates/dom_query)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat)](https://docs.rs/dom_query)
[![Build Status](https://github.com/niklak/dom_query/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/niklak/dom_query/actions/workflows/rust.yml)
[![CircleCI](https://circleci.com/gh/niklak/dom_query.svg?style=shield)](https://app.circleci.com/pipelines/github/niklak/dom_query)

DOM_QUERY is based on HTML crate html5ever and the CSS selector crate selectors. You can use the jQuery-like syntax to query and manipulate an HTML document quickly. **With its help you can query dom and modify it.**.

It is a fork of [nipper](https://crates.io/crates/nipper), with some updates. Also this fork supports `:has`, `:has-text`, `:contains` pseudo-classes, and some others.

## Example

#### Extract the hacker news.

```rust
use dom_query::Document;

fn main() {
    let html = include_str!("../test-pages/hacker_news.html");
    let document = Document::from(html);

    document.select("tr.athing:has(a[href][id])").iter().for_each(|athing| {
        let title = athing.select(".title a");
        let href = athing.select(".storylink");
        println!("{}", title.text());
        println!("{}", href.attr("href").unwrap());
    });
}
```

## Related projects

* [html5ever](https://crates.io/crates/html5ever)
* [selectors](https://crates.io/crates/selectors)
* [select.rs](https://crates.io/crates/select)
* [goquery](https://godoc.org/github.com/PuerkitoBio/goquery)


## Features

- `hashbrown` -- optional, standard hashmaps and hashsets will be replaced `hashbrown` hashmaps and hashsets;

## Changelog
[Changelog](./CHANGELOG.md)

## License

Licensed under MIT ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)


## Contribution

Any contribution intentionally submitted for inclusion in the work by you, shall be
licensed with MIT license, without any additional terms or conditions.
