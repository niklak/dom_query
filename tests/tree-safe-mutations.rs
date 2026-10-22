#![cfg(feature = "checked-node-mutations")]

mod data;

use data::ANCESTORS_CONTENTS;
use dom_query::{Document, NodeRef};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[track_caller]
fn assert_move_is_ignored(contents: &str, f: impl FnOnce(&Document)) {
    let doc = Document::from(contents);
    let html = doc.select("body").html();

    f(&doc);

    doc.tree.validate().unwrap();
    assert_eq!(doc.select("body").html(), html);
}

#[track_caller]
fn assert_ancestor_move_is_ignored(ancestor_sel: &str, f: impl Fn(&NodeRef, &NodeRef)) {
    assert_move_is_ignored(ANCESTORS_CONTENTS, |doc| {
        let ancestor = doc.select_single(ancestor_sel).nodes()[0];
        let node = doc.select_single("#first-child").nodes()[0];
        f(&node, &ancestor);
    });
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_ancestor_as_child_moves_are_ignored() {
    for ancestor_sel in ["#parent", "#grand-parent"] {
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.append_child(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.prepend_child(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.append_children(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.prepend_children(ancestor);
        });
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_ancestor_as_sibling_moves_are_ignored() {
    for ancestor_sel in ["#parent", "#grand-parent"] {
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.insert_before(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.insert_after(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.replace_with(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.wrap_node(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.insert_siblings_before(ancestor);
        });
        assert_ancestor_move_is_ignored(ancestor_sel, |node, ancestor| {
            node.insert_siblings_after(ancestor);
        });
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn test_ancestor_in_moved_run_is_ignored() {
    let contents =
        r#"<div id="outer"><p id="run"></p><section><div id="inner"></div></section></div>"#;

    let move_node = |doc: &Document| {
        let run = doc.select_single("#run").nodes()[0];
        let node = doc.select_single("#inner").nodes()[0];
        node.append_children(&run);
    };
    assert_move_is_ignored(contents, move_node);

    let move_node = |doc: &Document| {
        let run = doc.select_single("#run").nodes()[0];
        let node = doc.select_single("#inner").nodes()[0];
        node.prepend_children(&run);
    };
    assert_move_is_ignored(contents, move_node);

    let move_node = |doc: &Document| {
        let run = doc.select_single("#run").nodes()[0];
        let node = doc.select_single("#inner").nodes()[0];
        node.insert_siblings_before(&run);
    };
    assert_move_is_ignored(contents, move_node);

    let move_node = |doc: &Document| {
        let run = doc.select_single("#run").nodes()[0];
        let node = doc.select_single("#inner").nodes()[0];
        node.insert_siblings_after(&run);
    };
    assert_move_is_ignored(contents, move_node);
}
