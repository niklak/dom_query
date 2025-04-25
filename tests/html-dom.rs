use dom_query::{Document, SerializableNodeRef};
use html5ever::parse_document;
use html5ever::serialize;
use html5ever::serialize::{SerializeOpts, TraversalScope};
use tendril::SliceExt;
use tendril::StrTendril;
use tendril::TendrilSink;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod alloc;

fn parse_and_serialize(input: StrTendril) -> StrTendril {
    let dom = Document::fragment(input);

    let validity_check = dom.tree.validate();
    assert!(validity_check.is_ok(), "Tree is not valid: {}", validity_check.unwrap_err());

    let root = dom.root();
    let inner: SerializableNodeRef = root.first_child().unwrap().into();

    let mut result = vec![];
    serialize(&mut result, &inner, Default::default()).unwrap();
    StrTendril::try_from_byte_slice(&result).unwrap()
}

macro_rules! test_fn {
    ($f:ident, $name:ident, $input:expr, $output:expr) => {
        #[cfg_attr(not(target_arch = "wasm32"), test)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn $name() {
            assert_eq!($output, &*$f($input.to_tendril()));
        }
    };

    // Shorthand for $output = $input
    ($f:ident, $name:ident, $input:expr) => {
        test_fn!($f, $name, $input, $input);
    };
}

macro_rules! test {
    ($($t:tt)*) => {
        test_fn!(parse_and_serialize, $($t)*);
    };
}

test!(empty, r#""#);
test!(fuzz, "<a a=\r\n", "");
test!(smoke_test, r#"<p><i>Hello</i>, World!</p>"#);

test!(
    misnest,
    r#"<p><i>Hello!</p>, World!</i>"#,
    r#"<p><i>Hello!</i></p><i>, World!</i>"#
);

test!(attr_literal, r#"<base foo="<'>">"#);
test!(attr_escape_amp, r#"<base foo="&amp;">"#);
test!(
    attr_escape_amp_2,
    r#"<base foo=&amp>"#,
    r#"<base foo="&amp;">"#
);
test!(
    attr_escape_nbsp,
    "<base foo=x\u{a0}y>",
    r#"<base foo="x&nbsp;y">"#
);
test!(
    attr_escape_quot,
    r#"<base foo='"'>"#,
    r#"<base foo="&quot;">"#
);
test!(
    attr_escape_several,
    r#"<span foo=3 title='test "with" &amp;quot;'>"#,
    r#"<span foo="3" title="test &quot;with&quot; &amp;quot;"></span>"#
);

test!(text_literal, r#"<p>"'"</p>"#);
test!(text_escape_amp, r#"<p>&amp;</p>"#);
test!(text_escape_amp_2, r#"<p>&amp</p>"#, r#"<p>&amp;</p>"#);
test!(text_escape_nbsp, "<p>x\u{a0}y</p>", r#"<p>x&nbsp;y</p>"#);
test!(text_escape_lt, r#"<p>&lt;</p>"#);
test!(text_escape_gt, r#"<p>&gt;</p>"#);
test!(text_escape_gt2, r#"<p>></p>"#, r#"<p>&gt;</p>"#);

test!(
    script_literal,
    r#"<script>(x & 1) < 2; y > "foo" + 'bar'</script>"#
);
test!(
    style_literal,
    r#"<style>(x & 1) < 2; y > "foo" + 'bar'</style>"#
);
test!(xmp_literal, r#"<xmp>(x & 1) < 2; y > "foo" + 'bar'</xmp>"#);
test!(
    iframe_literal,
    r#"<iframe>(x & 1) < 2; y > "foo" + 'bar'</iframe>"#
);
test!(
    noembed_literal,
    r#"<noembed>(x & 1) < 2; y > "foo" + 'bar'</noembed>"#
);
test!(
    noframes_literal,
    r#"<noframes>(x & 1) < 2; y > "foo" + 'bar'</noframes>"#
);

test!(pre_lf_0, "<pre>foo bar</pre>");
test!(pre_lf_1, "<pre>\nfoo bar</pre>", "<pre>foo bar</pre>");
test!(pre_lf_2, "<pre>\n\nfoo bar</pre>", "<pre>\nfoo bar</pre>");

test!(textarea_lf_0, "<textarea>foo bar</textarea>");
test!(
    textarea_lf_1,
    "<textarea>\nfoo bar</textarea>",
    "<textarea>foo bar</textarea>"
);
test!(
    textarea_lf_2,
    "<textarea>\n\nfoo bar</textarea>",
    "<textarea>\nfoo bar</textarea>"
);

test!(listing_lf_0, "<listing>foo bar</listing>");
test!(
    listing_lf_1,
    "<listing>\nfoo bar</listing>",
    "<listing>foo bar</listing>"
);
test!(
    listing_lf_2,
    "<listing>\n\nfoo bar</listing>",
    "<listing>\nfoo bar</listing>"
);

test!(comment_1, r#"<p>hi <!--world--></p>"#);
test!(comment_2, r#"<p>hi <!-- world--></p>"#);
test!(comment_3, r#"<p>hi <!--world --></p>"#);
test!(comment_4, r#"<p>hi <!-- world --></p>"#);

// FIXME: test serialization of qualified tag/attribute names that can't be
// parsed from HTML

test!(attr_ns_1, r#"<svg xmlns="bleh"></svg>"#);
test!(attr_ns_2, r#"<svg xmlns:foo="bleh"></svg>"#);
test!(attr_ns_3, r#"<svg xmlns:xlink="bleh"></svg>"#);
test!(attr_ns_4, r#"<svg xlink:href="bleh"></svg>"#);

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn doctype() {
    let dom = parse_document(Document::default(), Default::default()).one("<!doctype html>");
    let mut result = vec![];
    let root = dom.root();
    let document: SerializableNodeRef = root.first_child().unwrap().into();
    serialize(
        &mut result,
        &document,
        SerializeOpts {
            scripting_enabled: true,
            traversal_scope: TraversalScope::IncludeNode,
            create_missing_parent: false,
        },
    )
    .unwrap();
    assert_eq!(String::from_utf8(result).unwrap(), "<!DOCTYPE html>");

    let validity_check = dom.tree.validate();
    assert!(validity_check.is_ok(), "Tree is not valid: {}", validity_check.unwrap_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[should_panic]
fn test_issue_node_append_the_next_sibling() {
    const HTML: &str = r#"<!DOCTYPE html><html><head></head><body>
           <div id="parent">
               <div id="child" class="child">Child</div>
            </div>
        </body></html>"#;

    let doc = Document::from(HTML);

    let sel = doc.select("#child");
    let child = sel.nodes().first().unwrap();

    // - Create a wrapper directly in the same tree area.
    // This element is detached from the DOM tree -- it has no parent.
    let wrapper = doc.tree.new_element("div");
    wrapper.set_attr("id", "wrapper");

    // - Insert wrapper before existing child in the parent. 
    // Under the hood `insert_before_of` will call `remove_from_parent` for the new children,
    // because we don`t know if it is a new node without bound to the tree or not.
    // At this point `child` becomes a next sibling of the `wrapper`.
    doc.tree.insert_before_of(&child.id, &wrapper.id);

    // If we don't call `remove_from_parent` for the `child` node,
    // it will still be the next sibling of the `wrapper` node
    // and at the same time also a child of the `wrapper` node.
    // In this case, it will be both the first and the last child of the `wrapper`.
    // - Move child into wrapper as the only child
    //doc.tree.remove_from_parent(&child.id);
    doc.tree.append_child_of(&wrapper.id, &child.id);
    // This may seem inconvenient, but the `Tree` methods are intended more for internal use,
    // while `NodeRef` is designed for external usage.

    // So yes, without "detaching" the child node from the tree, the `Tree` instance becomes invalid.

    // **But I cannot consider this a bug**.

    // Because there are ways to avoid this situation:
    // 1. Using `Node::insert_before` and `Node::append_child` methods, which will do the dirty work for you:
    // child.insert_before(&wrapper);
    // wrapper.append_child(child);

    // 2. Using `Node::replace_with` and `Node::append_child` methods, which also handle cleanup internally.
    // But note that the child node will be removed from the tree twice in this case.
    // child.replace_with(&wrapper);
    // wrapper.append_child(child);

    // - Normalize the document for printing
    doc.normalize();

    // Check to see if the structure is valid
    let validity_check = doc.tree.validate();
    assert!(validity_check.is_ok(), "Tree is not valid: {}", validity_check.unwrap_err());
}
