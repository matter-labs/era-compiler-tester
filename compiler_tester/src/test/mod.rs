//!
//! The test.
//!

pub mod case;
pub mod context;
pub mod description;
pub mod instance;
pub mod selector;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use solidity_adapter::EVMVersion;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::Case;
use crate::test::context::case::CaseContext;
use crate::test::context::input::InputContext;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;

///
/// The test.
///
#[derive(Debug)]
pub struct Test {
    /// The test name.
    name: String,
    /// The test cases.
    cases: Vec<Case>,
    /// The test mode.
    mode: Mode,
    /// The test group.
    group: Option<String>,
    /// The EraVM contract builds.
    eravm_builds: HashMap<web3::types::U256, Vec<u8>>,
    /// The EVM contract builds.
    evm_builds: HashMap<String, Vec<u8>>,
    /// The EVM version.
    evm_version: Option<EVMVersion>,
}

impl Test {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        name: String,
        cases: Vec<Case>,
        mode: Mode,
        group: Option<String>,
        eravm_builds: HashMap<web3::types::U256, Vec<u8>>,
        evm_builds: HashMap<String, Vec<u8>>,
        evm_version: Option<EVMVersion>,
    ) -> Self {
        Self {
            name,
            cases,
            mode,
            group,
            eravm_builds,
            evm_builds,
            evm_version,
        }
    }

    ///
    /// Runs the test on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(self, summary: Arc<Mutex<Summary>>, vm: Arc<EraVM>)
    where
        D: EraVMDeployer,
    {
        let context = CaseContext {
            name: &self.name,
            mode: &self.mode,
            group: &self.group,
        };
        for case in self.cases {
            let vm = EraVM::clone_with_contracts(vm.clone(), self.eravm_builds.clone(), None);
            case.run_eravm::<D, M>(summary.clone(), vm.clone(), &context);
        }
    }

    ///
    /// Runs the test on REVM.
    ///
    pub fn run_revm(self, summary: Arc<Mutex<Summary>>) {
        for case in self.cases {
            let context = CaseContext {
                name: &self.name,
                mode: &self.mode,
                group: &self.group,
            };
            case.run_revm(summary.clone(), self.evm_version, &context);
        }
    }

    ///
    /// Runs the test on EVM interpreter.
    ///
    pub fn run_evm_interpreter<D, const M: bool>(self, summary: Arc<Mutex<Summary>>, vm: Arc<EraVM>)
    where
        D: EraVMDeployer,
    {
        for case in self.cases {
            let vm = EraVM::clone_with_contracts(
                vm.clone(),
                self.eravm_builds.clone(),
                self.evm_version,
            );
            let context = CaseContext {
                name: &self.name,
                mode: &self.mode,
                group: &self.group,
            };
            case.run_evm_interpreter::<D, M>(summary.clone(), vm, &context);
        }
    }
}
