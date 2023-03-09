//!
//! The Vyper subprocess compiler mode.
//!

///
/// The Vyper subprocess compiler mode.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubprocessMode {
    /// The Vyper compiler version.
    pub version: semver::Version,
    /// Whether to run the Vyper compiler optimizer.
    pub optimize: bool,
}

impl SubprocessMode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(version: semver::Version, optimize: bool) -> Self {
        Self { version, optimize }
    }
}
