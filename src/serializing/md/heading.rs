use html5ever::{QualName, local_name};

pub(super) const fn heading_prefix(name: &QualName) -> &'static str {
    match name.local {
        local_name!("h1") => "# ",
        local_name!("h2") => "## ",
        local_name!("h3") => "### ",
        local_name!("h4") => "#### ",
        local_name!("h5") => "##### ",
        // h6, the last heading `write_element` dispatches here
        _ => "###### ",
    }
}

/// A run of `#` that ends a heading after a space would be dropped as the
/// ATX closing sequence; the content from `start` follows the prefix's space.
pub(super) fn escape_closing_hashes(text: &mut String, start: usize) {
    let content = text.get(start..).unwrap_or_default().trim_end();
    let stem = content.trim_end_matches('#');
    if stem.len() < content.len() && (stem.is_empty() || stem.ends_with(' ')) {
        text.insert(start + stem.len(), '\\');
    }
}
