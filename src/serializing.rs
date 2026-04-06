//! This module provides serialization functions. Currently contains only markdown serialization.
//! 

#[cfg(feature = "markdown")]
mod md;

#[cfg(feature = "markdown")]
pub(crate) use md::serialize_md;
