//!
//! The tests directory entity.
//!

use std::path::PathBuf;

///
/// The tests directory entity.
///
pub struct TestsDirectory {
    /// The tests directory path.
    pub path: PathBuf,
    /// The tests extension.
    pub extension: String,
    /// The flag if flatten enabled.
    pub flatten: bool,
}
