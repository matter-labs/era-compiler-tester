//!
//! The compiler output.
//!

pub mod build;

use std::collections::BTreeMap;
use std::collections::HashMap;

use self::build::Build;

///
/// The compiler output.
///
#[derive(Debug, Clone)]
pub struct Output {
    /// The contract builds.
    pub builds: HashMap<String, Build>,
    /// The contracts method identifiers.
    pub method_identifiers: Option<BTreeMap<String, BTreeMap<String, u32>>>,
    /// The last contract name.
    pub last_contract: String,
}

impl Output {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        builds: HashMap<String, Build>,
        method_identifiers: Option<BTreeMap<String, BTreeMap<String, u32>>>,
        last_contract: String,
    ) -> Self {
        Self {
            builds,
            method_identifiers,
            last_contract,
        }
    }
}
