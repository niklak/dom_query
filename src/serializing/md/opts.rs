use std::borrow::Cow;

use super::text_utils::linebreak;

use super::constants::LIST_OFFSET_BASE;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct EmphasisScope(u8);

impl EmphasisScope {
    pub const BOLD: Self = Self(1 << 0);
    pub const ITALIC: Self = Self(1 << 1);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[must_use]
    pub const fn insert(mut self, other: Self) -> Self {
        self.0 |= other.0;
        self
    }
}

impl From<&str> for EmphasisScope {
    fn from(marker: &str) -> Self {
        match marker {
            // TODO: support "_"
            "*" => Self::ITALIC,
            _ => Self::BOLD,
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy)]
pub struct FormatOpts {
    pub include_node: bool,
    pub skip_escape: bool,
    pub inline: bool,
    /// Whether to skip Markdown formatting syntax and render plain text.
    pub skip_md: bool,
    /// Enables formatting rules specific to table cells:
    /// 1. Prevents standard line breaks.
    /// 2. Replaces line breaks with `<br>` tags.
    pub table_cell: bool,
    pub offset: usize,
    pub emphasis_scope: EmphasisScope,
}

impl FormatOpts {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn include_node(mut self) -> Self {
        self.include_node = true;
        self
    }

    pub const fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub const fn skip_escape(mut self) -> Self {
        self.skip_escape = true;
        self
    }

    pub const fn table_cell(mut self) -> Self {
        self.table_cell = true;
        self
    }

    pub const fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
    pub const fn skip_md(mut self) -> Self {
        self.skip_md = true;
        self
    }

    pub const fn emphasis_scope(mut self, scope: EmphasisScope) -> Self {
        self.emphasis_scope = self.emphasis_scope.insert(scope);
        self
    }
}

/// Represents the kind of list being serialized.
pub enum ListKind {
    Ul,
    Ol(u64),
}

/// Context for a list being serialized.
pub struct ListContext {
    pub opts: FormatOpts,
    pub kind: ListKind,
    list_indent: usize,
}

impl ListContext {
    pub fn new(opts: FormatOpts, kind: ListKind) -> Self {
        let list_opts = opts.offset(opts.offset + 1);
        Self {
            opts: list_opts,
            kind,
            list_indent: opts.offset * LIST_OFFSET_BASE,
        }
    }

    pub fn list_indent(&self) -> String {
        " ".repeat(self.list_indent)
    }

    pub const fn linebreak(&self) -> &str {
        linebreak(self.opts.table_cell)
    }

    pub const fn ul_prefix(&self) -> &'static str {
        if self.opts.table_cell { "+ " } else { "- " }
    }

    pub fn prefix(&self) -> Cow<'static, str> {
        match self.kind {
            ListKind::Ul => Cow::Borrowed(self.ul_prefix()),
            ListKind::Ol(n) => Cow::Owned(format!("{n}. ")),
        }
    }
}
