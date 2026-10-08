//! Round trip: each spec example's Markdown is rendered with pulldown-cmark,
//! the html goes through `Document::md`, and that is rendered again. Both
//! renders must show the same thing.
//! 
//! The inputs are the official spec examples (CC-BY-SA 4.0), which are fetched
//! on demand and cached locally in `FIXTURE_DIR`:
//! - CommonMark spec: `CMARK_SPEC_FILE` from `CMARK_SPEC_URL`
//! - GFM spec: `GFM_SPEC_FILE` from `GFM_SPEC_URL`
#![cfg(feature = "markdown")]
#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashSet;

use pulldown_cmark::{Event, Options, Parser, html};
use ureq;
use dom_query::Document;

const FIXTURE_DIR : &str = "test-md-fixtures";
const CMARK_SPEC_FILE: &str = "commonmark-spec.txt";
const GFM_SPEC_FILE: &str = "gfm-spec.txt";
const CMARK_SPEC_URL: &str = "https://raw.githubusercontent.com/commonmark/commonmark-spec/0.31.2/spec.txt";
const GFM_SPEC_URL: &str = "https://raw.githubusercontent.com/github/cmark-gfm/27d942c8b0a62d192f616e5bf3578f4b6a89e180/test/spec.txt";


/// Extension examples to run from the GFM spec; task list examples are tagged `disabled`.
const GFM_EXTENSIONS: &[&str] = &["table", "strikethrough", "disabled"];

/// `md` writes nested emphasis of one kind as a single run, which renders the same.
const NESTED_EMPHASIS: &str = "em em, strong strong";

const FENCE: &str = "````````````````````````````````";
const EXAMPLE: &str = "```````````````````````````````` example";

fn parser(md: &str) -> Parser<'_> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    Parser::new_ext(md, opts)
}

fn render(md: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, parser(md));
    out
}

/// Raw html passes through the renderer as written, so `md` can't be expected
/// to reproduce it.
fn has_raw_html(md: &str) -> bool {
    parser(md).any(|e| matches!(e, Event::Html(_) | Event::InlineHtml(_)))
}

/// Whitespace as a browser shows it: outside `<pre>` a run of it is one
/// space, and none shows at the edges of a paragraph.
fn as_shown(html: &str) -> String {
    let collapse = |s: &str| s.split_ascii_whitespace().collect::<Vec<_>>().join(" ");
    let out: String = html
        .split("<pre")
        .enumerate()
        .map(|(i, part)| match part.split_once("</pre>") {
            Some((code, rest)) if i > 0 => ["<pre", code, "</pre>", &collapse(rest)].concat(),
            _ => collapse(part),
        })
        .collect();
    out.replace("<p> ", "<p>").replace(" </p>", "</p>")
}

/// The examples of a spec.txt as (number, extension tag, markdown).
fn spec_examples(spec: &str) -> Vec<(usize, &str, String)> {
    let mut examples = Vec::new();
    let mut lines = spec.lines();
    while let Some(line) = lines.next() {
        if let Some(tag) = line.strip_prefix(EXAMPLE) {
            let md: String = lines
                .by_ref()
                .take_while(|l| *l != ".")
                .map(|l| l.to_owned() + "\n")
                .collect();
            lines.by_ref().take_while(|l| *l != FENCE).for_each(drop);
            examples.push((examples.len() + 1, tag.trim(), md.replace('→', "\t")));
        }
    }
    examples
}

/// Round-trips the examples of `spec` whose extension tag `keep` accepts.
fn check_spec(spec: &str, keep: impl Fn(&str) -> bool) {
    let mut seen = HashSet::new();
    let mut failures = Vec::new();
    for (n, tag, spec_md) in spec_examples(spec) {
        if !keep(tag) || has_raw_html(&spec_md) {
            continue;
        }
        let html = render(&spec_md);
        // Many examples render the same html; run each distinct one once.
        if html.is_empty() || !seen.insert(html.clone()) {
            continue;
        }
        let doc = Document::from(html.as_str());
        if doc.select(NESTED_EMPHASIS).exists() {
            continue;
        }
        let md = doc.md(None);
        let (want, got) = (as_shown(&html), as_shown(&render(&md)));
        if want != got {
            eprintln!("=== {n}\n{spec_md}--- md\n{md}\n--- want\n{want}\n--- got\n{got}\n");
            failures.push(n);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures: {failures:?}",
        failures.len()
    );
}

use std::error::Error;

fn load_or_fetch_spec(filename: &str, url: &str) -> Result<String, Box<dyn Error>> {
    let dir = std::path::Path::new(FIXTURE_DIR);
    let path = dir.join(filename);

    // 1. file exists already -- read from file
    if path.exists() {
        return Ok(std::fs::read_to_string(&path)?);
    }

    // 2. create directory if it doesn't exist
    std::fs::create_dir_all(dir)?;

    println!("Downloading {filename} from {url}...");

    // 3. downloading file
    let content = ureq::get(url).call()?.body_mut().read_to_string()?;

    // 4. caching it for further use
    std::fs::write(&path, &content)?;

    Ok(content)
}

#[test]
fn commonmark_spec() {
    match load_or_fetch_spec(CMARK_SPEC_FILE, CMARK_SPEC_URL) {
        Ok(spec) => check_spec(&spec, |_| true),
        Err(e) => eprintln!("Failed to load commonmark specs: {}", e),
    }
}



#[test]
fn gfm_spec_extensions() {
    match load_or_fetch_spec(GFM_SPEC_FILE, GFM_SPEC_URL) {
        Ok(spec) => check_spec(&spec, |tag| {
            GFM_EXTENSIONS.contains(&tag)
        }),
        Err(e) => eprintln!("Failed to load gfm specs: {}", e),
    }
}

/// Html that the spec round trip can't reach, because its input is always
/// html that pulldown-cmark rendered from Markdown. Each case is an input
/// and the html it should show after `md` and a render.
const HTML_INPUTS: &[(&str, &str)] = &[
    // emphasis whose `*` run wouldn't be left- or right-flanking
    ("<p><em>a.</em>b</p>", "<p><em>a.</em>b</p>"),
    (
        "<p>a<strong>-b-</strong>c</p>",
        "<p>a<strong>-b-</strong>c</p>",
    ),
    // emphasis around a block
    ("<em><p>a</p></em>", "<p><em>a</em></p>"),
    // a link's leading space
    (
        "<p>x<a href=\"/u\"> a</a></p>",
        "<p>x<a href=\"/u\"> a</a></p>",
    ),
    // a literal tilde run
    ("<p>~a~</p>", "<p>~a~</p>"),
    // a literal backtick before a code span
    ("<p>a`<code>b</code></p>", "<p>a`<code>b</code></p>"),
    // a backslash before a pipe in a table cell's code
    (
        "<table><thead><tr><th>h</th></tr></thead><tbody>\n<tr><td><code>a\\|b</code></td></tr>\n</tbody></table>",
        "<table><thead><tr><th>h</th></tr></thead><tbody>\n<tr><td><code>a\\|b</code></td></tr>\n</tbody></table>",
    ),
];

#[test]
fn html_inputs() {
    let mut failures = Vec::new();
    for &(input, want) in HTML_INPUTS {
        let md = Document::from(input).md(None);
        let got = as_shown(&render(&md));
        if got != as_shown(want) {
            eprintln!("=== {input}\n--- md\n{md}\n--- want\n{want}\n--- got\n{got}\n");
            failures.push(input);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures: {failures:?}",
        failures.len()
    );
}