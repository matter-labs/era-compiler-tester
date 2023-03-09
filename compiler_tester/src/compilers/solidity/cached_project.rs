//!
//! The Solidity compiler cached projects.
//!

use std::collections::BTreeMap;

///
/// The Solidity compiler cached projects.
///
pub struct CachedProject {
    /// The Solidity project.
    pub project: compiler_solidity::Project,
    /// The method identifiers.
    pub method_identifiers: BTreeMap<String, BTreeMap<String, String>>,
    /// The last contract name.
    pub last_contract: String,
}

impl CachedProject {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        project: compiler_solidity::Project,
        method_identifiers: BTreeMap<String, BTreeMap<String, String>>,
        last_contract: String,
    ) -> Self {
        Self {
            project,
            method_identifiers,
            last_contract,
        }
    }
}
