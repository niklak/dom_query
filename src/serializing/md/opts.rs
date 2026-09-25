#[allow(clippy::struct_excessive_bools)]
#[derive(Default, Clone, Copy)]
pub struct FormatOpts {
    pub include_node: bool,
    pub ignore_linebreak: bool,
    pub skip_escape: bool,
    pub offset: usize,
    pub br: bool,
    pub inline: bool,
}

impl FormatOpts {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn include_node(mut self) -> Self {
        self.include_node = true;
        self
    }

    pub const fn ignore_linebreak(mut self) -> Self {
        self.ignore_linebreak = true;
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

    pub const fn br(mut self) -> Self {
        self.br = true;
        self
    }
    pub const fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}
