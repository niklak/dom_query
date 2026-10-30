use crate::{Element, NodeId, NodeRef};

use super::super::ctx::FormatOpts;
use super::super::host::MDHost;
use super::super::text_utils::trim_trailing_space;

pub(in crate::serializing::md) fn write<H: MDHost>(
    h: &H,
    text: &mut String,
    _el: &Element,
    id: NodeId,
    _opts: FormatOpts,
) {
    let table_ref = NodeRef::new(id, h.tree());
    let row_nodes = table_ref.find(&["tr"]);

    let header_id = table_header_row(&row_nodes).map(|row| row.id);
    if !is_table_node_writable(&table_ref, &row_nodes, header_id) {
        h.write(text, id, FormatOpts::default());
        return;
    }

    let opts = FormatOpts::new().table_cell();
    let mut headings = vec![];
    let mut rows = vec![];
    // the first row with cells (the header row, when there is one) aligns the columns
    let aligns = row_nodes
        .iter()
        .map(|tr| {
            tr.children_it(false)
                .filter(is_table_cell)
                .map(|cell| cell_alignment(&cell))
                .collect::<Vec<_>>()
        })
        .find(|row| !row.is_empty())
        .unwrap_or_default();
    for tr_ref in &row_nodes {
        let mut row = vec![];
        for cell_ref in tr_ref.children_it(false).filter(is_table_cell) {
            let mut cell_text = String::new();
            h.write(&mut cell_text, cell_ref.id, opts);
            trim_trailing_cell_break(&mut cell_text);
            escape_table_cell_inplace(&mut cell_text);
            row.push(cell_text);
        }
        if header_id == Some(tr_ref.id) {
            headings = row;
        } else if !row.is_empty() {
            rows.push(row);
        }
    }

    while headings.len() < rows.first().map_or(0, Vec::len) {
        headings.push(" ".into());
    }

    text.push_str("\n| ");

    let heading = headings.join(" | ");
    text.push_str(&heading);
    text.push_str(" |\n");
    text.push_str("| ");

    text.push_str(
        headings
            .iter()
            .enumerate()
            .map(|(i, s)| delimiter_cell(s.len(), aligns.get(i).copied().flatten()))
            .collect::<Vec<_>>()
            .join(" | ")
            .as_str(),
    );
    text.push_str(" |\n");

    for row in rows {
        text.push_str("| ");
        text.push_str(&row.join(" | "));
        text.push_str(" |\n");
    }

    text.push('\n');
}

/// Block children of a table cell are joined with a single cell linebreak,
/// as a cell cannot hold a blank line.
pub(in crate::serializing::md) fn add_cell_block_break(text: &mut String, linebreak: &str) {
    trim_trailing_space(text);
    if !text.is_empty() && !ends_with_cell_break(text) {
        text.push_str(linebreak);
    }
}

/// Drops trailing cell linebreaks left by a final `<br>` or block child.
fn trim_trailing_cell_break(text: &mut String) {
    while ends_with_cell_break(text) {
        text.truncate(text.len() - "<br>".len());
        trim_trailing_space(text);
    }
}

/// Whether the text ends with a cell linebreak rather than escaped cell text:
/// `\<br>` is text, `\\<br>` is an escaped backslash and then a linebreak.
fn ends_with_cell_break(text: &str) -> bool {
    text.strip_suffix("<br>")
        .is_some_and(|rest| (rest.len() - rest.trim_end_matches('\\').len()) % 2 == 0)
}

fn is_table_node_writable(
    table_node: &NodeRef,
    rows: &[NodeRef],
    header_id: Option<NodeId>,
) -> bool {
    if table_node.descendants_it().any(|n| n.has_name("table")) {
        // if table has inline table then ignore this table
        return false;
    }
    let mut common_cell_count: usize = 0;
    for row in rows {
        if header_id == Some(row.id) {
            continue;
        }
        let curr_cell_count = row.children_it(false).filter(is_table_cell).count();
        if common_cell_count == 0 {
            common_cell_count = curr_cell_count;
        } else if common_cell_count != curr_cell_count {
            return false;
        }
    }
    header_id.is_some() || common_cell_count > 0
}

/// A cell's `left`, `center` or `right` alignment; its `text-align` style
/// overrides the `align` attribute, as in CSS.
fn cell_alignment(cell: &NodeRef) -> Option<&'static str> {
    let style = cell.attr("style").unwrap_or_default();
    let align = cell.attr("align");
    // the last declaration wins, unless an earlier one is `!important`
    let declarations = || {
        style
            .rsplit(';')
            .filter_map(|decl| decl.split_once(':'))
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case("text-align"))
            .map(|(_, value)| value)
    };
    let value = declarations()
        .find(|value| strip_important(value) != value.trim())
        .or_else(|| declarations().next())
        .map(strip_important)
        .or(align.as_deref())?
        .trim();
    ["left", "center", "right"]
        .into_iter()
        .find(|a| value.eq_ignore_ascii_case(a))
}

fn delimiter_cell(width: usize, align: Option<&str>) -> String {
    let (left, right) = match align {
        Some("left") => (":", ""),
        Some("center") => (":", ":"),
        Some("right") => ("", ":"),
        _ => ("", ""),
    };
    let dashes = width.saturating_sub(left.len() + right.len()).max(1);
    [left, &"-".repeat(dashes), right].concat()
}

/// Drops a trailing `!important`, with or without whitespace, in any case.
fn strip_important(value: &str) -> &str {
    let value = value.trim();
    match value.rfind('!') {
        Some(i) if value[i + 1..].trim().eq_ignore_ascii_case("important") => value[..i].trim_end(),
        _ => value,
    }
}

fn is_table_cell(node: &NodeRef) -> bool {
    node.has_name("th") || node.has_name("td")
}

/// The first row with cells, when they are all `th`; a `th` next to a `td`
/// labels its row instead.
fn table_header_row<'a>(rows: &'a [NodeRef<'a>]) -> Option<&'a NodeRef<'a>> {
    let row = rows
        .iter()
        .find(|row| row.children_it(false).any(|c| is_table_cell(&c)))?;
    let all_th = row
        .children_it(false)
        .filter(is_table_cell)
        .all(|c| c.has_name("th"));
    all_th.then_some(row)
}

fn escape_table_cell_inplace(content: &mut String) {
    if content.is_empty() || !content.contains('|') {
        return;
    }

    let bytes = content.as_bytes();
    let mut result = String::with_capacity(content.len() + 4);
    let mut last = 0;

    for i in 0..bytes.len() {
        if bytes[i] == b'|' {
            // Check if '|' is preceded by an unescaped backslash
            let is_escaped = i > 0 && bytes[i - 1] == b'\\';
            if !is_escaped {
                // Append preceding slice and insert escaping backslash
                result.push_str(&content[last..i]);
                result.push('\\');
                // Set 'last' to 'i' so '|' itself will be included in the next chunk slice
                last = i;
            }
        }
    }
    // If all pipes were already escaped, avoid extra string assignment
    if result.is_empty() {
        return;
    }
    // Append the remaining tail of the string
    result.push_str(&content[last..]);
    *content = result;
}
