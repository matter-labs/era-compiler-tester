//!
//! The Vyper compiler cached projects.
//!

///
/// The Vyper compiler cached projects.
///
pub struct CachedProject {
    /// The Vyper project.
    pub project: compiler_vyper::Project,
}

impl CachedProject {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(project: compiler_vyper::Project) -> Self {
        Self { project }
    }
}
