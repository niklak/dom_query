use html5ever::{QualName, local_name};

// Limited set of elements treated as block-level in Markdown output.
pub(super) const fn is_md_block(name: &QualName) -> bool {
    if is_block_wrapper(name) {
        return true;
    }
    matches!(
        name.local,
        local_name!("blockquote")
            | local_name!("p")
            | local_name!("h1")
            | local_name!("h2")
            | local_name!("h3")
            | local_name!("h4")
            | local_name!("h5")
            | local_name!("h6")
            | local_name!("ul")
            | local_name!("ol")
            | local_name!("dl")
            | local_name!("table")
            | local_name!("hr")
            | local_name!("figure")
            | local_name!("figcaption")
            | local_name!("dt")
            | local_name!("dd")
    )
}

/// An element that is written as its own line, or table cell, outside link text.
pub(super) fn is_link_block(name: &QualName) -> bool {
    // TODO: add the other HTML5 block elements missing from `is_md_block`.
    is_md_block(name)
        || matches!(
            name.local,
            local_name!("li") | local_name!("tr") | local_name!("th") | local_name!("td")
        )
}

/// A `<div>`-like container with no Markdown syntax of its own.
pub(super) const fn is_block_wrapper(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("div")
            | local_name!("section")
            | local_name!("article")
            | local_name!("header")
            | local_name!("footer")
            | local_name!("main")
            | local_name!("nav")
            | local_name!("aside")
            | local_name!("address")
            | local_name!("details")
            | local_name!("summary")
            | local_name!("hgroup")
    )
}

pub(super) const fn is_list(name: &QualName) -> bool {
    matches!(name.local, local_name!("ul") | local_name!("ol"))
}
