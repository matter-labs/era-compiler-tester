//!
//! The Vyper compiler cache key.
//!

///
/// The Vyper compiler cache key.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The test path.
    pub test_path: String,
    /// The Vyper compiler version.
    pub version: semver::Version,
    /// Whether to run the Vyper compiler optimizer.
    pub optimize: bool,
}

impl CacheKey {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(test_path: String, version: semver::Version, optimize: bool) -> Self {
        Self {
            test_path,
            version,
            optimize,
        }
    }
}
