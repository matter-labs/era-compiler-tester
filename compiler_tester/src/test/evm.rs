//!
//! The EVM test.
//!

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::test::case::Case;
use crate::vm::evm::input::build::Build as EVMBuild;
use crate::vm::evm::invoker::Invoker as EVMInvoker;
use crate::vm::evm::runtime::Runtime as EVMRuntime;
use crate::vm::evm::EVM;
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
    builds: HashMap<web3::types::Address, EVMBuild>,
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
        builds: HashMap<web3::types::Address, EVMBuild>,
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
    pub fn run(self, summary: Arc<Mutex<Summary>>) {
        for case in self.cases {
            let config = evm::standard::Config::shanghai();
            let etable =
                evm::Etable::<evm::standard::State, EVMRuntime, evm::trap::CallCreateTrap>::runtime(
                );
            let resolver = evm::standard::EtableResolver::new(&config, &(), &etable);
            let invoker = EVMInvoker::new(&config, &resolver);

            case.run_evm(
                summary.clone(),
                EVM::new(self.builds.clone(), invoker),
                &self.mode,
                self.name.clone(),
                self.group.clone(),
            );
        }
    }
}
