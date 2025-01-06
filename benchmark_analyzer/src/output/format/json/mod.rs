//!
//! Defines JSON-based output formats.
//!

pub mod lnt;
pub mod native;

use crate::output::file::File;

///
/// Create a new [`crate::output::File`] instance with an object serialized to JSON.
///
pub(crate) fn make_json_file<T>(filename: impl std::fmt::Display, object: T) -> File
where
    T: Sized + serde::Serialize,
{
    let path = format!("{filename}.json").into();
    let contents = serde_json::to_string_pretty(&object).expect("Always valid");
    File { path, contents }
}
