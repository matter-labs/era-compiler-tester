//!
//! The EraVM test.
//!

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::test::case::Case;
use crate::vm::eravm::deployers::Deployer as EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::Summary;

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
    /// Runs the test.
    ///
    pub fn run<D, const M: bool>(self, summary: Arc<Mutex<Summary>>, vm: Arc<EraVM>)
    where
        D: EraVMDeployer,
    {
        for case in self.cases {
            let vm = EraVM::clone_with_contracts(vm.clone(), self.builds.clone());
            case.run_eravm::<D, M>(
                summary.clone(),
                vm.clone(),
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
        }
    }
}
