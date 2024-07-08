//!
//! The test.
//!

pub mod case;
pub mod instance;

use solidity_adapter::EVMVersion;
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
    /// The test cases.
    cases: Vec<Case>,
    /// The test mode.
    mode: Mode,
    /// The test group.
    group: Option<String>,
    /// The EraVM contract builds.
    eravm_builds: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
    /// The EVM contract builds.
    evm_builds: HashMap<String, EVMBuild>,
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
        eravm_builds: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
        evm_builds: HashMap<String, EVMBuild>,
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
        for case in self.cases {
            let vm = EraVM::clone_with_contracts(vm.clone(), self.eravm_builds.clone(), None);
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
    /// Runs the test on REVM.
    ///
    pub fn run_revm(self, summary: Arc<Mutex<Summary>>) {
        for case in self.cases {
            /*let config = evm::standard::Config::shanghai();
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
            );*/

            let vm = revm::Evm::builder().build();
            case.run_revm(summary.clone(),vm,&self.mode, self.name.clone(), self.group.clone(), self.evm_builds.clone());
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
            case.run_evm_interpreter::<D, M>(
                summary.clone(),
                vm,
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
        }
    }
}
