use std::convert::AsRef;
use std::fmt;
use std::ops::Deref;

use cssparser::{self, ToCss};
use html5ever::LocalName;
use tendril::StrTendril;

/// CssString wraps StrTendril, to implement <ToCss> trait. Currently it is used in the <InnerSelector>.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct CssString(StrTendril);

impl Deref for CssString {
    type Target = StrTendril;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for CssString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CssString {
    fn from(value: &str) -> Self {
        CssString(StrTendril::from(value))
    }
}

impl ToCss for CssString {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        //dest.write_str(&self.0)
        cssparser::serialize_string(&self.0, dest)
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct CssLocalName(LocalName);

impl ToCss for CssLocalName {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl From<&str> for CssLocalName {
    fn from(value: &str) -> Self {
        CssLocalName(value.into())
    }
}

impl Deref for CssLocalName {
    type Target = LocalName;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
