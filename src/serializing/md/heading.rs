use html5ever::{QualName, local_name};

pub(super) const fn is_heading(name: &QualName) -> bool {
    matches!(
        name.local,
        local_name!("h1")
            | local_name!("h2")
            | local_name!("h3")
            | local_name!("h4")
            | local_name!("h5")
            | local_name!("h6")
    )
}

pub(super) const fn md_prefix(name: &QualName) -> Option<&'static str> {
    let prefix = match name.local {
        local_name!("h1") => "# ",
        local_name!("h2") => "## ",
        local_name!("h3") => "### ",
        local_name!("h4") => "#### ",
        local_name!("h5") => "##### ",
        local_name!("h6") => "###### ",
        local_name!("hr") => "---",
        _ => "",
    };

    if prefix.is_empty() {
        None
    } else {
        Some(prefix)
    }
}

/// A run of `#` that ends a heading after a space would be dropped as the
/// ATX closing sequence.
pub(super) fn escape_closing_hashes(text: &mut String) {
    let content = text.trim_end();
    let stem = content.trim_end_matches('#');
    if stem.len() < content.len() && stem.ends_with(' ') {
        text.insert(stem.len(), '\\');
    }
}
