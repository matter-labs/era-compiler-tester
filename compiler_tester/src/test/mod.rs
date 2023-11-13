//!
//! The test.
//!

pub mod case;
pub mod instance;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::deployers::Deployer;
use crate::eravm::EraVM;
use crate::Summary;

use self::case::Case;

///
/// The test.
///
pub struct Test {
    /// The test name.
    name: String,
    /// The test group.
    group: Option<String>,
    /// The test mode.
    mode: Mode,
    /// The contract builds.
    builds: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
    /// The test cases.
    cases: Vec<Case>,
}

impl Test {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        name: String,
        group: Option<String>,
        mode: Mode,
        builds: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
        cases: Vec<Case>,
    ) -> Self {
        Self {
            name,
            group,
            mode,
            builds,
            cases,
        }
    }

    ///
    /// Run the test.
    ///
    pub fn run<D, const M: bool>(self, summary: Arc<Mutex<Summary>>, initial_vm: Arc<EraVM>)
    where
        D: Deployer,
    {
        for case in self.cases {
            let vm = EraVM::clone_with_contracts(initial_vm.clone(), self.builds.clone());
            case.run::<D, M>(
                summary.clone(),
                vm,
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
        }
    }
}
