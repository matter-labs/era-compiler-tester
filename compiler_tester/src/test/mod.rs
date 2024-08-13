//!
//! The test.
//!

pub mod case;
pub mod instance;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::test::case::Case;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::invoker::Invoker as EVMInvoker;
use crate::vm::evm::runtime::Runtime as EVMRuntime;
use crate::vm::evm::EVM;

///
/// The test.
///
#[derive(Debug)]
pub struct Test {
    /// The test name.
    name: String,
    /// The test group.
    group: Option<String>,
    /// The test mode.
    mode: Mode,
    /// The EraVM contract builds.
    eravm_builds: HashMap<web3::types::U256, Vec<u8>>,
    /// The EVM contract builds.
    evm_builds: HashMap<String, EVMBuild>,
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
        eravm_builds: HashMap<web3::types::U256, Vec<u8>>,
        evm_builds: HashMap<String, EVMBuild>,
        cases: Vec<Case>,
    ) -> Self {
        Self {
            name,
            group,
            mode,
            eravm_builds,
            evm_builds,
            cases,
        }
    }

    ///
    /// Runs the test on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(self, summary: Arc<Mutex<Summary>>, vm: Arc<EraVM>)
    where
        D: EraVMDeployer,
    {
        for case in self.cases {
            let vm = EraVM::clone_with_contracts(vm.clone(), self.eravm_builds.clone());
            case.run_eravm::<D, M>(
                summary.clone(),
                vm.clone(),
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
        }
    }

    ///
    /// Runs the test on EVM.
    ///
    pub fn run_evm(self, summary: Arc<Mutex<Summary>>) {
        for case in self.cases {
            let config = evm::standard::Config::shanghai();
            let etable =
                evm::Etable::<evm::standard::State, EVMRuntime, evm::trap::CallCreateTrap>::runtime(
                );
            let resolver = evm::standard::EtableResolver::new(&config, &(), &etable);
            let invoker = EVMInvoker::new(&config, &resolver);

            let vm = EVM::new(self.evm_builds.clone(), invoker);
            case.run_evm(
                summary.clone(),
                vm,
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
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
            let vm = EraVM::clone_with_contracts(vm.clone(), self.eravm_builds.clone());
            case.run_evm_interpreter::<D, M>(
                summary.clone(),
                vm.clone(),
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
        }
    }
}
