mod md;

#[cfg(feature = "markdown")]
pub(crate) use md::serialize_md;
