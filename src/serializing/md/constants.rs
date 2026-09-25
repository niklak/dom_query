pub(super) const LIST_OFFSET_BASE: usize = 4;

/// Characters with Markdown meaning anywhere in a line, escaped unconditionally.
pub(super) const ALWAYS_ESCAPED: &[char] = &['\\', '`', '*', '_', '[', ']', '<', '|', '"'];
/// Characters that can only start a Markdown block at the beginning of a line
pub(super) const LINE_START_ESCAPED: &[char] = &['#', '>', '-', '+'];

pub(super) const DEFAULT_SKIP_TAGS: [&str; 4] = ["script", "style", "meta", "head"];
pub(super) const CODE_LANGUAGE_ATTRIBUTES: [&str; 2] = ["data-lang", "data-language"];
pub(super) const CODE_LANGUAGE_PREFIX: &str = "language-";