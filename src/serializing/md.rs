mod constants;
mod ext;
mod serializer;
mod text_utils;

use tendril::StrTendril;

use crate::node::NodeRef;

pub fn serialize_md(
    root_node: &NodeRef,
    include_node: bool,
    skip_tags: Option<&[&str]>,
) -> StrTendril {
    serializer::MDSerializer::new(root_node, skip_tags).serialize(include_node)
}

#[cfg(test)]
mod tests {

    use crate::Document;

    use super::*;

    #[track_caller]
    fn html_2md_compare(html_contents: &str, expected: &str) {
        let doc = Document::from(html_contents);
        let root_node = &doc.root();
        let md_text = serialize_md(root_node, false, None);
        assert_eq!(md_text.as_ref(), expected);
    }

    /// Renders the serialized Markdown with pulldown-cmark and returns the
    /// printable event stream (tag starts/ends and text/code content), so
    /// tests can assert what a Markdown consumer actually sees.
    fn pulldown_events(markdown: &str) -> Vec<String> {
        use pulldown_cmark::{Event, Options, Parser};
        Parser::new_ext(markdown, Options::all())
            .map(|event| match event {
                Event::Start(tag) => format!("<{tag:?}>"),
                Event::End(_) => "</>".to_string(),
                Event::Text(t) | Event::Code(t) | Event::Html(t) | Event::InlineHtml(t) => {
                    t.to_string()
                }
                Event::InlineMath(_) | Event::DisplayMath(_) => "<math>".to_string(),
                Event::SoftBreak => "\\n".to_string(),
                Event::HardBreak => "<br>".to_string(),
                Event::Rule => "<rule>".to_string(),
                Event::FootnoteReference(t) => format!("[^{t}]"),
                Event::TaskListMarker(b) => format!("[{}]", if b { "x" } else { " " }),
            })
            .collect()
    }

    #[track_caller]
    fn assert_events(markdown: &str, expected: &[&str]) {
        let events = pulldown_events(markdown);
        let events: Vec<&str> = events.iter().map(String::as_str).collect();
        assert_eq!(events, expected, "event stream mismatch for {markdown:?}");
    }

    #[test]
    fn test_headings() {
        // when passing include_node: true, leading and trailing whitespaces will be kept.
        let contents = r"<h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
        <h4>Heading 4</h4>
        <h5>Heading 5</h5>
        <h6>Heading 6</h6>
        <h3><span>III.</span> Heading With Span</h3>
        <h3><span></span>Early years (2006–2009)</h3>
        <h3><span> </span> Early years (2006–2009)</h3>
        <hr>";

        let expected = "\n\n# Heading 1\n\n\
        ## Heading 2\n\n\
        ### Heading 3\n\n\
        #### Heading 4\n\n\
        ##### Heading 5\n\n\
        ###### Heading 6\n\n\
        ### III. Heading With Span\n\n\
        ### Early years (2006–2009)\n\n\
        ### Early years (2006–2009)\n\n\
        ---\n\n";

        let doc = Document::from(contents);
        let body_sel = &doc.select("body");
        let body_node = body_sel.nodes().first().unwrap();
        let md_text = serialize_md(body_node, true, None);
        assert_eq!(md_text.as_ref(), expected);
    }

    #[test]
    fn test_emphasis_boundary_whitespace() {
        // CommonMark requires an opening delimiter to be followed, and a closing
        // delimiter to be preceded, by a non-whitespace character. Whitespace
        // staying inside the delimiter run renders as literal `**`/`*`.
        html_2md_compare(
            "<p><strong>The Rundown: </strong>Body text</p>",
            "**The Rundown:** Body text",
        );
        html_2md_compare(
            "<p><em> Leading</em> and <strong>trailing </strong></p>",
            "*Leading* and **trailing**",
        );
        // nested emphasis keeps working
        html_2md_compare(
            "<p><strong><em>both </em>bold</strong></p>",
            "***both* bold**",
        );
        // whitespace-only emphasis drops its delimiters instead of emitting an
        // unbalanced run
        html_2md_compare("<p>a<strong>  </strong>b</p>", "a b");
    }

    #[test]
    fn test_emphasis_wrapping_list() {
        // A list inside an emphasis element is written into the same buffer
        // after the opening delimiter. Trimming that nested output must not
        // shift the text before it: the delimiter's recorded byte offset
        // then pointed into the middle of `é` (a panic) or at the wrong
        // character (`**ab cd` became `** abcd`).
        html_2md_compare(
            "<h1>Hi</h1><em>résumé tips<ul><li>a</li></ul></em>",
            "# Hi\n\n*résumé tips\n\n- a\n*",
        );
        html_2md_compare(
            "<div><i>中文字<ol><li>一</li></ol>后</i></div>",
            "*中文字\n\n1. 一\n后*",
        );
        html_2md_compare(
            "<div><b>ab cd<ul><li>y</li></ul></b></div>",
            "**ab cd\n\n- y\n**",
        );
        html_2md_compare(
            "<ul><li><b>ab cd<ol><li>y</li></ol></b></li></ul>",
            "- **ab cd\n\n    1. y\n**",
        );
    }

    #[test]
    fn test_ordered_list_start_and_value() {
        html_2md_compare("<ol start=\"5\"><li>a</li><li>b</li></ol>", "5. a\n6. b");
        html_2md_compare(
            "<ol><li>a</li><li value=\"9\">b</li><li>c</li></ol>",
            "1. a\n9. b\n10. c",
        );
        // a non-numeric start falls back to the default numbering
        html_2md_compare("<ol start=\"x\"><li>a</li></ol>", "1. a");
        // CommonMark markers hold at most nine digits, and the counter must
        // not overflow on huge values
        html_2md_compare(
            "<ol start=\"1234567890\"><li>a</li><li>b</li></ol>",
            "999999999. a\n999999999. b",
        );
        html_2md_compare(
            "<ol start=\"18446744073709551615\"><li>a</li></ol>",
            "999999999. a",
        );
        html_2md_compare(
            "<ol><li>a</li><li value=\"1234567890\">b</li></ol>",
            "1. a\n999999999. b",
        );
    }

    #[test]
    fn test_nested_list_under_wide_marker() {
        // a child list must start at the parent item's content column; at a
        // fixed four columns it sits left of `100. ` and parses as an
        // indented code block
        let md = "100. a\n\n     1. x";
        html_2md_compare("<ol start=\"100\"><li>a<ol><li>x</li></ol></li></ol>", md);
        assert_events(
            md,
            &[
                "<List(Some(100))>",
                "<Item>",
                "<Paragraph>",
                "a",
                "</>",
                "<List(Some(1))>",
                "<Item>",
                "x",
                "</>",
                "</>",
                "</>",
                "</>",
            ],
        );
        html_2md_compare(
            "<ol start=\"98\"><li>a</li><li>b</li><li>c<ul><li>n</li></ul></li></ol>",
            "98. a\n99. b\n100. c\n\n     - n",
        );
    }

    #[test]
    fn test_list_item_block_syntax_text() {
        // text inside a list item that begins with block syntax must be
        // escaped, or it turns the line into a nested construct
        html_2md_compare("<ul><li>1. Preheat oven</li></ul>", "- 1\\. Preheat oven");
        html_2md_compare("<ul><li># tag</li></ul>", "- \\# tag");
        html_2md_compare("<ul><li>- x</li></ul>", "- \\- x");
        html_2md_compare("<ul><li>+ x</li></ul>", "- \\+ x");
        html_2md_compare("<ul><li>&gt; x</li></ul>", "- \\> x");
        html_2md_compare("<ol><li># tag</li></ol>", "1. \\# tag");
        // continuation blocks inside an item are indented, not at a raw line
        // start, and need the same protection
        html_2md_compare("<ul><li><p>a</p><p># b</p></li></ul>", "- a\n\n  \\# b");
        // plain inline content in the same positions stays unescaped
        html_2md_compare("<ul><li>a #b</li></ul>", "- a #b");
    }

    #[test]
    fn test_img_src_fallbacks() {
        // lazy-load pages carry the image URL in srcset or data-src
        html_2md_compare(
            "<p><img srcset=\"https://i.e.com/p.png 1x, https://i.e.com/p2x.png 2x\" alt=\"pic\"></p>",
            "![pic](https://i.e.com/p.png)",
        );
        html_2md_compare(
            "<p><img data-src=\"https://i.e.com/p.png\" alt=\"pic\"></p>",
            "![pic](https://i.e.com/p.png)",
        );
        // the first candidate may omit its descriptor and carry a trailing
        // comma; leading commas and whitespace are skipped too
        html_2md_compare(
            "<p><img srcset=\"a.png, b.png 2x\" alt=\"pic\"></p>",
            "![pic](a.png)",
        );
        html_2md_compare(
            "<p><img srcset=\", a.png 2x\" alt=\"pic\"></p>",
            "![pic](a.png)",
        );
        // a `!` before an image must not swallow the image's own `!`
        html_2md_compare(
            "<p>Wow!<img src=\"i.png\" alt=\"a\"></p>",
            "Wow\\!![a](i.png)",
        );
        // a real src still wins
        html_2md_compare(
            "<p><img src=\"https://i.e.com/s.png\" data-src=\"https://i.e.com/d.png\" alt=\"pic\"></p>",
            "![pic](https://i.e.com/s.png)",
        );
        // an empty placeholder src counts as missing
        html_2md_compare(
            "<p><img src=\"\" data-src=\"https://i.e.com/p.png\" alt=\"pic\"></p>",
            "![pic](https://i.e.com/p.png)",
        );
        // no URL anywhere: still dropped
        html_2md_compare("<p><img alt=\"pic\"></p>", "");
        html_2md_compare("<p><img src=\"\" alt=\"pic\"></p>", "");
    }

    #[test]
    fn test_link_destination_and_alt_escaping() {
        // balanced parens need no escaping
        html_2md_compare(
            "<p><a href=\"https://e.com/x(y)z\">x</a></p>",
            "[x](https://e.com/x(y)z)",
        );
        // unbalanced parens truncate the destination at the stray `)`
        html_2md_compare(
            "<p><a href=\"https://e.com/x)y\">x</a></p>",
            "[x](<https://e.com/x)y>)",
        );
        // a space turns the whole destination into plain text
        html_2md_compare(
            "<p><a href=\"https://e.com/a b.png\">x</a></p>",
            "[x](<https://e.com/a b.png>)",
        );
        // `>` alone is legal in a bare destination
        html_2md_compare(
            "<p><a href=\"https://e.com/a>b\">x</a></p>",
            "[x](https://e.com/a>b)",
        );
        // brackets in alt text break the image
        html_2md_compare(
            "<p><img src=\"https://i.e.com/p.png\" alt=\"a [b] c\"></p>",
            "![a \\[b\\] c](https://i.e.com/p.png)",
        );

        // the destinations survive rendering as actual links
        for (md, dest) in [
            ("[x](<https://e.com/x)y>)", "https://e.com/x)y"),
            ("[x](<https://e.com/a b.png>)", "https://e.com/a b.png"),
            ("[x](https://e.com/a>b)", "https://e.com/a>b"),
        ] {
            let events = pulldown_events(md).join("\n");
            assert!(
                events.contains(&format!("dest_url: Borrowed(\"{dest}\")")),
                "{dest:?} not parsed as destination in {md}: {events}"
            );
        }
    }

    #[test]
    fn test_linked_image() {
        // an image wrapped in a link must become the link body, not be dropped
        html_2md_compare(
            "<p><a href=\"https://e.com\"><img src=\"https://i.e.com/p.png\" alt=\"pic\"></a></p>",
            "[![pic](https://i.e.com/p.png)](https://e.com)",
        );
        // the rendered stream must contain an Image nested inside a Link
        let events = pulldown_events("[![pic](https://i.e.com/p.png)](https://e.com)").join("\n");
        let link = events.find("Link {").expect("link event");
        assert!(
            events[link..].contains("Image {"),
            "image not inside link: {events}"
        );
        // an image with a text sibling keeps both
        html_2md_compare(
            "<p><a href=\"https://e.com\">see <img src=\"https://i.e.com/p.png\" alt=\"pic\"></a></p>",
            "[see ![pic](https://i.e.com/p.png)](https://e.com)",
        );
        // a genuinely empty link is still skipped
        html_2md_compare("<p><a href=\"https://e.com\"></a>x</p>", "x");
    }

    #[test]
    fn test_card_link_with_image_and_blocks() {
        // a card-style link wraps block children: a blank line inside `[...]`
        // ends the paragraph, leaving the brackets as literal text
        let md = "[![](t.png) Caption text](/card)";
        html_2md_compare(
            "<div><a href=\"/card\"><div><img src=\"t.png\"></div><p>Caption text</p></a></div>",
            md,
        );
        assert_events(
            md,
            &[
                "<Paragraph>",
                "<Link { link_type: Inline, dest_url: Borrowed(\"/card\"), title: Borrowed(\"\"), id: Borrowed(\"\") }>",
                "<Image { link_type: Inline, dest_url: Borrowed(\"t.png\"), title: Borrowed(\"\"), id: Borrowed(\"\") }>",
                "</>",
                " Caption text",
                "</>",
                "</>",
            ],
        );
        // an image without any source writes nothing, so the link keeps its
        // plain-text body
        html_2md_compare(
            "<div><a href=\"/p/1\"><img class=\"avatar\"><div>Name</div><div>Follow</div></a></div>",
            "[NameFollow](/p/1)",
        );
    }

    #[test]
    fn test_adjacent_emphasis_elements() {
        // A closing delimiter run immediately followed by an opening one of the
        // same type merges into an unparseable sequence (`**a****b**`), so
        // adjacent elements of the same type are merged into one.
        html_2md_compare("<p><strong>a</strong><strong>b</strong> c</p>", "**ab** c");
        html_2md_compare("<p><em>a</em><em>b</em>c</p>", "*ab*c");
        // an empty element in between writes nothing and does not interfere
        html_2md_compare("<p><b>a</b><i></i><b>b</b>c</p>", "**ab**c");
        // nested runs merge with the element that closed last
        html_2md_compare("<p><b><i>a</i></b><b>b</b>c</p>", "***a*b**c");
        // different types keep their delimiters: `**a***b*` and `*a***b**`
        // parse back into two elements, followed by a word character or not
        html_2md_compare("<p><strong>a</strong><em>b</em>c</p>", "**a***b*c");
        assert_events(
            "**a***b*c",
            &[
                "<Paragraph>",
                "<Strong>",
                "a",
                "</>",
                "<Emphasis>",
                "b",
                "</>",
                "c",
                "</>",
            ],
        );
        html_2md_compare("<p>中<em>a</em><b>b</b>文</p>", "中*a***b**文");
        assert_events(
            "中*a***b**文",
            &[
                "<Paragraph>",
                "中",
                "<Emphasis>",
                "a",
                "</>",
                "<Strong>",
                "b",
                "</>",
                "文",
                "</>",
            ],
        );
        // an escaped asterisk before an element is text, not a delimiter run
        html_2md_compare("<p>a*<em>b</em>c</p>", "a\\**b*c");
        assert_events(
            "a\\**b*c",
            &[
                "<Paragraph>",
                "a",
                "*",
                "<Emphasis>",
                "b",
                "</>",
                "c",
                "</>",
            ],
        );
        // a space between the elements means no collision
        html_2md_compare(
            "<p><strong>a</strong> <strong>b</strong></p>",
            "**a** **b**",
        );
    }

    #[test]
    fn test_escaping() {
        // Punctuation that has no Markdown meaning in prose is not escaped.
        html_2md_compare("<p>Foo. Bar!</p>", "Foo. Bar!");
        html_2md_compare(
            "<p>Call (555) 123-4567, see {a: 1} and https://example.com/x.</p>",
            "Call (555) 123-4567, see {a: 1} and https://example.com/x.",
        );
        // mid-line `#`, `>` and `-` keep their literal meaning
        html_2md_compare("<p>see #tags, a > b, x - y</p>", "see #tags, a > b, x - y");
        // ...except a word of only `#`, which closes an ATX heading
        html_2md_compare("<h2>Title #</h2>", "## Title \\#");
        assert_events(
            "## Title \\#",
            &[
                "<Heading { level: H2, id: None, classes: [], attrs: [] }>",
                "Title ",
                "#",
                "</>",
            ],
        );
        html_2md_compare("<h1>#</h1>", "# \\#");
        assert_events(
            "# \\#",
            &[
                "<Heading { level: H1, id: None, classes: [], attrs: [] }>",
                "#",
                "</>",
            ],
        );
        html_2md_compare("<h2>A ### </h2>", "## A \\###");
        html_2md_compare("<h1>C# and F#</h1>", "# C# and F#");

        // Characters that can start a Markdown block when a line begins are
        // escaped there.
        html_2md_compare("<p># not a heading</p>", "\\# not a heading");
        html_2md_compare("<p>> not a quote</p>", "\\> not a quote");
        html_2md_compare("<p>- not a list</p>", "\\- not a list");
        // ... and only the first character: later ones in the word are plain
        // text and have no block meaning
        html_2md_compare("<p>#tag-name</p>", "\\#tag-name");
        html_2md_compare("<p>+1-2</p>", "\\+1-2");
        html_2md_compare("<p>>foo>bar</p>", "\\>foo>bar");
        // a preceding space on the same line is not a line start
        html_2md_compare("<p><span>foo </span><span>#bar</span></p>", "foo #bar");
        html_2md_compare(
            "<p>2024. not an ordered list</p>",
            "2024\\. not an ordered list",
        );
        // ...but only there
        html_2md_compare("<p>see 2024. below</p>", "see 2024. below");

        // `!` is only escaped when it could introduce an image
        html_2md_compare("<p>Wow!</p>", "Wow!");
        html_2md_compare("<p>not ![an image]</p>", "not \\!\\[an image\\]");

        // Characters with Markdown meaning anywhere in the line are escaped.
        html_2md_compare(
            "<p>a *b* c _d_ e`f` [g]</p>",
            "a \\*b\\* c \\_d\\_ e\\`f\\` \\[g\\]",
        );
        // `|` always: it must not break generated table rows
        html_2md_compare("<p>a | b</p>", "a \\| b");
    }

    #[test]
    fn test_italic() {
        let contents = r"<h4><i>Italic Text</i></h4>";
        let expected = "#### *Italic Text*";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_span_italic() {
        let contents = r"<span>It`s like <i>that</i></span>";
        let expected = r"It\`s like *that*";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_bold_italic() {
        let contents = r"<span>It`s like <b><i>that</i></b></span>";
        let expected = r"It\`s like ***that***";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_simple_code() {
        let contents = r"<span>It`s like <code>that</code></span>";
        let expected = r"It\`s like `that`";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_false_multiline_code() {
        let contents = r"<span>
        It`s like 
        <code>
        that
        </code>
        </span>";
        let expected = r"It\`s like `that`";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_code_span_with_backticks() {
        // Backslash escapes are not interpreted inside code spans, so content
        // containing backticks must be wrapped in a longer delimiter run
        // (CommonMark code fence rule), with spaces padding the content.
        html_2md_compare("<p><code>a `b` c</code></p>", "`` a `b` c ``");
        html_2md_compare("<p><code>`leading</code></p>", "`` `leading ``");
        html_2md_compare("<p><code>trailing`</code></p>", "`` trailing` ``");
        html_2md_compare("<p><code>```</code></p>", "```` ``` ````");
        // content without backticks keeps the single-backtick form
        html_2md_compare("<p><code>go.sum</code></p>", "`go.sum`");
    }

    #[test]
    fn test_multiline_code() {
        let contents = r"<code>$ cargo new hello
    Created binary (application) `hello` package

$ cd hello</code>";
        let expected = r"```
$ cargo new hello
    Created binary (application) `hello` package

$ cd hello
```";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_ul() {
        let contents = "<h3>Pizza Margherita Ingredients</h3>\
        <ul>\
            <li>Pizza Dough</li>\
            <li>Mozzarella cheese</li>\
            <li>Tomatoes</li>\
            <li>Olive Oil</li>\
            <li><i>Basil</i></li>\
            <li><b>Salt</b></li>\
        </ul>";

        let expected = "### Pizza Margherita Ingredients\n\n\
        - Pizza Dough\n\
        - Mozzarella cheese\n\
        - Tomatoes\n\
        - Olive Oil\n\
        - *Basil*\n\
        - **Salt**";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_ol() {
        let contents = "<h3>Pizza Margherita Ingredients</h3>\
        <ol>\
            <li>Pizza Dough</li>\
            <li>Mozzarella cheese</li>\
            <li>Tomatoes</li>\
            <li>Olive Oil</li>\
            <li><i>Basil</i></li>\
            <li><b>Salt</b></li>\
        </ol>";

        let expected = "### Pizza Margherita Ingredients\n\n\
        1. Pizza Dough\n\
        2. Mozzarella cheese\n\
        3. Tomatoes\n\
        4. Olive Oil\n\
        5. *Basil*\n\
        6. **Salt**";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_bad_ol() {
        let contents = "<h3>Pizza Margherita Ingredients</h3>\
        <ol>\
            <li>Pizza Dough</li>\
            <li>Mozzarella cheese</li>\
            <li>Tomatoes</li>\
            <li>Olive Oil</li>\
            <div><i>Basil</i></div>\
            <li><b>Salt</b></li>\
        </ol>";

        let expected = "### Pizza Margherita Ingredients\n\n\
        1. Pizza Dough\n\
        2. Mozzarella cheese\n\
        3. Tomatoes\n\
        4. Olive Oil\n\
        \n*Basil*\n\n\
        5. **Salt**";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_list_item_with_inline_then_block() {
        // the paragraph must not merge into the marker line, nor into the
        // lead-in paragraph as a lazy continuation line
        html_2md_compare("<ul><li>a<p>more a</p></li></ul>", "- a\n\n  more a");
        let md = "10. lead\n\n    block";
        html_2md_compare("<ol start=\"10\"><li>lead<p>block</p></li></ol>", md);
        assert_events(
            md,
            &[
                "<List(Some(10))>",
                "<Item>",
                "<Paragraph>",
                "lead",
                "</>",
                "<Paragraph>",
                "block",
                "</>",
                "</>",
                "</>",
            ],
        );
        // a whitespace-only lead-in must not trigger the break
        html_2md_compare(
            "<ul><li>\n  <p>first</p>\n  <p>second</p>\n</li></ul>",
            "- first\n\n  second",
        );
        // in a nested list the continuation lines keep the outer indent too,
        // or the item would break apart
        html_2md_compare(
            "<ol><li><ol><li><p>P1</p><p>P2</p></li></ol></li></ol>",
            "1.\n\n    1. P1\n\n       P2",
        );
    }

    #[test]
    fn test_list_inline() {
        let contents = "
        <ol>\
            <li>Item 1</li>\
            <li>Item 2</li>\
            <li>Item 3\
                <div>\
                    <ol>\
                        <li>Item 3-1</li>\
                        <li>Item 3-2</li>\
                        <li>Item 3-3\
                            <ol>\
                                <li>Item 3-3-1</li>\
                                <li>Item 3-3-2</li>\
                                <li>Item 3-3-3</li>\
                            </ol>
                        </li>\
                    </ol>\
                </div>
            </li>\
        </ol>";

        let expected = "\
1. Item 1
2. Item 2
3. Item 3

    1. Item 3-1
    2. Item 3-2
    3. Item 3-3

        1. Item 3-3-1
        2. Item 3-3-2
        3. Item 3-3-3";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_list_with_paragraphs() {
        let contents = "<ol>
            <li>
                <p>Paragraph 1-1</p>
                <p>Paragraph 1-2</p>
            </li>
            <li><p>Paragraph 2-1</p><p>Paragraph 2-2</p></li>
            <li><p>Paragraph 3-1</p></li>
        </ol>
        <p>Another Paragraph</p>";

        let expected = "\
1. Paragraph 1-1

   Paragraph 1-2

2. Paragraph 2-1

   Paragraph 2-2

3. Paragraph 3-1

Another Paragraph";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_paragraphs() {
        let contents =
            "<p>To create paragraphs, use a blank line to separate one or more lines of text.</p>
        <p>I really like using <span>Markdown</span><span>  text</span>.</p>

        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected = "To create paragraphs, use a blank line to separate one or more lines of text.\n\n\
        I really like using Markdown text.\n\n\
        I think I'll use it to format all of my documents from now on.";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_links() {
        let simple_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com">Duck Duck Go</a>.</p>"#;
        let simple_expected =
            r"My favorite search engine is [Duck Duck Go](https://duckduckgo.com).";
        html_2md_compare(simple_contents, simple_expected);

        // link with title attribute
        let title_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com" title="Duck Duck Go">Duck Duck Go</a>.</p>"#;
        let title_expected = r#"My favorite search engine is [Duck Duck Go](https://duckduckgo.com "Duck Duck Go")."#;
        html_2md_compare(title_contents, title_expected);

        let bold_contents = r#"<p>My favorite search engine is <b><a href="https://duckduckgo.com">Duck Duck Go</a></b>.</p>"#;
        let bold_expected =
            r"My favorite search engine is **[Duck Duck Go](https://duckduckgo.com)**.";
        html_2md_compare(bold_contents, bold_expected);

        // bold inside of link is not supported.
        let bold_ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><b>Duck Duck Go</b></a>.</p>"#;
        let bold_ignored_expected =
            r"My favorite search engine is [Duck Duck Go](https://duckduckgo.com).";
        html_2md_compare(bold_ignored_contents, bold_ignored_expected);

        // any elements inside `a` elements are also ignored,
        // html5ever transforms a > div to div > a, and there is no way to determine how it was.
        // This is an open question.
        let ignored_contents = r#"<p>My favorite search engine is <a href="https://duckduckgo.com"><div>Duck Duck Go</div></a>.</p>"#;
        let ignored_expected =
            "My favorite search engine is\n\n[Duck Duck Go](https://duckduckgo.com)

.";
        html_2md_compare(ignored_contents, ignored_expected);

        let no_href_contents = r"<p>My favorite search engine is <a>Duck Duck Go</a>.</p>";
        let no_href_expected = "My favorite search engine is Duck Duck Go.";
        html_2md_compare(no_href_contents, no_href_expected);

        let complex_contents =
            r#"<a href="https://duckduckgo.com" title="My &quot;Search&quot;">Duck Duck Go</a>"#;
        let comptex_expected = r#"[Duck Duck Go](https://duckduckgo.com "My \"Search\"")"#;
        html_2md_compare(complex_contents, comptex_expected);

        // link text is escaped exactly once, in the link-body context
        let escaped_contents = r#"<p><a href="u">my_file "q"</a></p>"#;
        let escaped_expected = r#"[my\_file \"q\"](u)"#;
        html_2md_compare(escaped_contents, escaped_expected);

        // a trailing `!` must not turn the link into image syntax
        html_2md_compare(
            r#"<p>Wow!<a href="https://e.com">x</a></p>"#,
            r"Wow\![x](https://e.com)",
        );
        // an already escaped `!` keeps its (now doubled) backslash
        html_2md_compare(
            r#"<p>Wow\!<a href="https://e.com">x</a></p>"#,
            r"Wow\\![x](https://e.com)",
        );
        // same hazard when a linked image follows the text
        html_2md_compare(
            r#"<p>Wow!<a href="u"><img src="i.png" alt="a"></a></p>"#,
            r"Wow\![![a](i.png)](u)",
        );
    }

    #[test]
    fn test_images() {
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text"></p>"#;
        let simple_expected = "Image: ![Alt text](/path/to/img.jpg)";
        html_2md_compare(simple_contents, simple_expected);

        // with title
        let simple_contents =
            r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text" title="Title"></p>"#;
        let simple_expected = r#"Image: ![Alt text](/path/to/img.jpg "Title")"#;
        html_2md_compare(simple_contents, simple_expected);

        // a title with quotes must not terminate the destination early
        let simple_contents =
            r#"<p>Image: <img src="/path/to/img.jpg" alt="Alt text" title='A "Great" Photo'></p>"#;
        let simple_expected = r#"Image: ![Alt text](/path/to/img.jpg "A \"Great\" Photo")"#;
        html_2md_compare(simple_contents, simple_expected);

        // without alt
        let simple_contents = r#"<p>Image: <img src="/path/to/img.jpg"></p>"#;
        let simple_expected = r"Image: ![](/path/to/img.jpg)";
        html_2md_compare(simple_contents, simple_expected);

        // no img
        let simple_contents = r#"<p>Image:  <img alt="Alt text" title="Title"></p>"#;
        let simple_expected = "Image:";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_with_interior_backticks() {
        // A fenced block whose content contains a fence-length backtick line
        // would terminate at that line; the fence must be longer than any
        // interior backtick run (CommonMark §fenced-code-blocks).
        html_2md_compare(
            "<pre><code>```bash\nls\n```</code></pre>",
            "````\n```bash\nls\n```\n````",
        );
        // a short interior run does not force a longer fence
        html_2md_compare("<pre><code>a`b</code></pre>", "```\na`b\n```");
    }

    #[test]
    fn test_pre_code() {
        let simple_contents = "<pre>\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\
</pre>";
        let simple_expected = "```
fn main() {
    println!(\"Hello, World!\");
}
```";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code_with_data_lang_attribute() {
        let simple_contents = "<pre data-lang=\"rust\">\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\n\
</pre>";
        let simple_expected = "```rust
fn main() {
    println!(\"Hello, World!\");
}

```";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code_with_data_lang_attribute_in_parent_tag() {
        let simple_contents = "<div data-lang=\"rust\"><pre>\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\n\
</pre></div>";
        let simple_expected = "```rust
fn main() {
    println!(\"Hello, World!\");
}

```";
        html_2md_compare(simple_contents, simple_expected);
    }

    #[test]
    fn test_pre_code_with_language_css_class_in_child_code_tag() {
        let contents = "<pre><code class=\"language-rust something else\">\
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>\n\
<span>    </span><span>println!</span><span>(</span><span>\"Hello, World!\"</span><span>);</span>\n\
<span>}</span>\n\
</code></pre>";
        let expected = "```rust
fn main() {
    println!(\"Hello, World!\");
}

```";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_blockquote() {
        let simple_contents = "<blockquote><p>Quoted text</p></blockquote>";
        let simple_expected = "> Quoted text";
        html_2md_compare(simple_contents, simple_expected);

        let complex_contents = "<blockquote>
<p>
Who has seen the wind?<br>
Neither I nor you:<br>
But when the leaves hang trembling,<br>
The wind is passing through.
</p>
<p>
Who has seen the wind?<br>
Neither you nor I:<br>
But when the trees bow down their heads,<br>
The wind is passing by.
</p>
</blockquote>
<p><i>Christina Rossetti</i></p>";
        let complex_expected = r"> Who has seen the wind?  
> Neither I nor you:  
> But when the leaves hang trembling,  
> The wind is passing through.
> 
> Who has seen the wind?  
> Neither you nor I:  
> But when the trees bow down their heads,  
> The wind is passing by.

*Christina Rossetti*";
        html_2md_compare(complex_contents, complex_expected);

        let empty_contents = "<blockquote></blockquote>";
        let empty_expected = "";
        html_2md_compare(empty_contents, empty_expected);
    }

    #[test]
    fn test_inline_blockquote() {
        let contents = "<blockquote>
<p>
Who has seen the wind?<br>
Neither I nor you:<br>
But when the leaves hang trembling,<br>
The wind is passing through.
</p>
<blockquote>
<p>
Who has seen the wind?<br>
Neither you nor I:<br>
But when the trees bow down their heads,<br>
The wind is passing by.
</p>
</blockquote>
</blockquote>";
        let expected = r"> Who has seen the wind?  
> Neither I nor you:  
> But when the leaves hang trembling,  
> The wind is passing through.
> 
> > Who has seen the wind?  
> > Neither you nor I:  
> > But when the trees bow down their heads,  
> > The wind is passing by.";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table() {
        let contents = "<table>
    <tr>
        <th>Column 1</th>
        <th>Column 2</th>
        <th>Column 3</th>
    </tr>
    <tr>
        <td>R 1, <i>C 1</i></td>
        <td>R 1, <i>C 2</i></td>
        <td>R 1, <i>C 3</i></td>
    </tr>
    <tr>
        <td>R 2, <i>C 1</i></td>
        <td>R 2, <i>C 2</i></td>
        <td>R 2, <i>C 3</i></td>
    </tr>
</table>";
        let expected = "| Column 1 | Column 2 | Column 3 |
| -------- | -------- | -------- |
| R 1, *C 1* | R 1, *C 2* | R 1, *C 3* |
| R 2, *C 1* | R 2, *C 2* | R 2, *C 3* |";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_inside_table() {
        let contents = "<table>
    <tr>
        <td>
            <table>
                <tr>
                    <th>Column 1</th>
                    <th>Column 2</th>
                    <th>Column 3</th>
                </tr>
                <tr>
                    <td>R 1, <i>C 1</i></td>
                    <td>R 1, <i>C 2</i></td>
                    <td>R 1, <i>C 3</i></td>
                </tr>
                <tr>
                    <td>R 2, <i>C 1</i></td>
                    <td>R 2, <i>C 2</i></td>
                    <td>R 2, <i>C 3</i></td>
                </tr>
            </table>
        </td>
    </tr>
</table>";
        let expected = "| Column 1 | Column 2 | Column 3 |
| -------- | -------- | -------- |
| R 1, *C 1* | R 1, *C 2* | R 1, *C 3* |
| R 2, *C 1* | R 2, *C 2* | R 2, *C 3* |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_without_headings() {
        let contents = "<table>
    <tr>
        <td>R 1, <i>C 1</i></td>
        <td>R 1, <i>C 2</i></td>
        <td>R 1, <i>C 3</i></td>
    </tr>
    <tr>
        <td>R 2, <i>C 1</i></td>
        <td>R 2, <i>C 2</i></td>
        <td>R 2, <i>C 3</i></td>
    </tr>
</table>";
        let expected = "|   |   |   |
| - | - | - |
| R 1, *C 1* | R 1, *C 2* | R 1, *C 3* |
| R 2, *C 1* | R 2, *C 2* | R 2, *C 3* |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_with_row_header_cells() {
        // Row-header `<th>` cells must stay in their own row as first-class
        // cells; previously they were collected into the header row instead.
        // Only a first row made of `th` cells is the header.
        html_2md_compare(
            "<table>
    <tr><th></th><th>C1</th><th>C2</th></tr>
    <tr><th>R1</th><td>a</td><td>b</td></tr>
</table>",
            "|  | C1 | C2 |
| - | -- | -- |
| R1 | a | b |",
        );
        // cells within a row keep document order even when mixed th/td
        html_2md_compare(
            "<table>
    <tr><th>H</th><th>H2</th></tr>
    <tr><th>R1</th><td>x</td></tr>
</table>",
            "| H | H2 |
| - | -- |
| R1 | x |",
        );
        // without a `th` header row, the header is blank and every row,
        // including one that starts with a row-header `th`, is a body row
        html_2md_compare(
            "<table>
    <tr><th>R1</th><td>a</td></tr>
    <tr><th>R2</th><td>b</td></tr>
</table>",
            "|   |   |
| - | - |
| R1 | a |
| R2 | b |",
        );
        // a header-only table (main panicked indexing the missing body row)
        html_2md_compare(
            "<table><tr><th>a</th><th>b</th></tr></table>",
            "| a | b |\n| - | - |",
        );
    }

    #[test]
    fn test_adjacent_inline_blocks_separated() {
        // figcaption always follows a block-level image, the caption must not
        // glue to the image markdown
        html_2md_compare(
            "<figure><img src=\"https://i.e.com/p.png\" alt=\"f\"><figcaption>Photo Credit: Reddit</figcaption></figure>",
            "![f](https://i.e.com/p.png)\n\nPhoto Credit: Reddit",
        );
        // dt and dd are separate blocks in HTML but neither is a md block
        html_2md_compare("<dl><dt>Term</dt><dd>Def</dd></dl>", "Term\n\nDef");
        // block children inside a table cell must be joined with the cell
        // linebreak (<br>), not concatenated
        html_2md_compare(
            "<table><tr><th>H</th></tr><tr><td><p>para1</p><p>para2</p></td></tr></table>",
            "| H |\n| - |\n| para1<br>para2 |",
        );
    }

    #[test]
    fn test_table_with_empty_header_cells() {
        // An empty `<th>` must still get a separator cell, otherwise the
        // separator row is not a valid delimiter row and the table is not
        // recognized by Markdown renderers.
        let contents = "<table>
    <tr>
        <th></th>
        <th>x</th>
    </tr>
    <tr>
        <td>a</td>
        <td>b</td>
    </tr>
</table>";
        let expected = "|  | x |
| - | - |
| a | b |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_skip() {
        let contents = "<table>
    <tr>
        <td>R 1, <i>C 1</i></td>
        <td>R 1, <i>C 2</i></td>
        <td>R 1, <i>C 3</i></td>
    </tr>
    <tr>
        <td>R 2, <i>C 1</i></td>
        <td>R 2, <i>C 2</i></td>
    </tr>
</table>";
        let expected = "R 1, *C 1* R 1, *C 2* R 1, *C 3*  
R 2, *C 1* R 2, *C 2*";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_empty() {
        let contents = "<table>
    <tr></tr>
    <tr></tr>
</table>";
        let expected = "";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_table_with_list() {
        let contents = "<table>
    <tr>
        <td>1</td>
        <td>
            <ul><li>Lemon</li></ul>
            <ul><li>Lime</li></ul>
            <ul><li>Grapefruit</li></ul>
            <ul><li>Orange</li></ul>
        </td>
    </tr>
</table>";
        let expected =
            "|   |   |\n| - | - |\n| 1 | + Lemon<br>+ Lime<br>+ Grapefruit<br>+ Orange |";
        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_skip_tags_default() {
        // By default, formatter will skip ["script", "style", "meta", "head"]
        let contents = "
        <style>p {color: blue;}</style>
        <p>I really like using <b>Markdown</b>.</p>

        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected = "I really like using **Markdown**.\n\n\
        I think I'll use it to format all of my documents from now on.";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_skip_tags() {
        // If you need all text content of the elements, you need to pass Some(&vec![]) to `md`.
        // If you pass a structure like this into `Document::from`, the html5ever will create html > head > style.
        // If you want to preserve order use `Document::fragment`.
        let contents = "<style>p {color: blue;}</style>\
        <div><h1>Content Heading<h1></div>\
        <p>I really like using Markdown.</p>\
        <p>I think I'll use it to format all of my documents from now on.</p>";

        let expected = "p {color: blue;}\n\n\
        I really like using Markdown.\n\n\
        I think I'll use it to format all of my documents from now on.";

        let doc = Document::fragment(contents);
        let html_node = &doc.root();
        let md_text = serialize_md(html_node, false, Some(&["div"]));
        assert_eq!(md_text.as_ref(), expected);
    }
    #[test]
    fn test_linebreak_after_lists() {
        let contents = r"Influenced
        <ul>
         <li>Idris (programming language)</li>
         <li>Project Verona</li>
         <li>Spark</li>
         <li>Swift</li>
         <li>V</li>
         <li>Zig</li>
        </ul>
        <p><b>Rust</b> is a general-purpose programming language</p>";
        let expected = "Influenced\n\n\
- Idris (programming language)
- Project Verona
- Spark
- Swift
- V
- Zig

**Rust** is a general-purpose programming language";

        html_2md_compare(contents, expected);
    }

    #[test]
    fn test_pre_code_without_new_line() {
        let simple_contents = r#"<pre>
<span>fn</span> <span>main</span><span>()</span><span> </span><span>{</span>
<span>    </span><span>println!</span><span>(</span><span>"Hello, World!"</span><span>);</span>
<span>}</span></pre>"#;
        let simple_expected = "```
fn main() {
    println!(\"Hello, World!\");
}
```";
        html_2md_compare(simple_contents, simple_expected);
    }
}
