//!
//! The Vyper subprocess compiler cache key.
//!

///
/// The Vyper subprocess compiler cache key.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VyperCacheKey {
    /// The test path.
    pub test_path: String,
    /// The Vyper compiler version.
    pub version: semver::Version,
    /// Whether to run the Vyper compiler optimizer.
    pub optimize: bool,
}

impl VyperCacheKey {
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
