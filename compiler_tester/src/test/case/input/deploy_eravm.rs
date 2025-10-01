//!
//! The EraVM deploy contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use crate::summary::Summary;
use crate::test::case::input::calldata::Calldata;
use crate::test::case::input::identifier::InputIdentifier;
use crate::test::case::input::output::Output;
use crate::test::case::input::storage::Storage;
use crate::test::description::TestDescription;
use crate::test::InputContext;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;

///
/// The EraVM deploy contract call input variant.
///
#[derive(Debug, Clone)]
pub struct DeployEraVM {
    /// The contract path.
    path: String,
    /// The contract hash.
    hash: web3::types::U256,
    /// The calldata.
    calldata: Calldata,
    /// The caller.
    caller: web3::types::Address,
    /// The value in wei.
    value: Option<u128>,
    /// The contracts storage to set before running.
    storage: Storage,
    /// The expected output.
    expected: Output,
}

impl DeployEraVM {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        path: String,
        hash: web3::types::U256,
        calldata: Calldata,
        caller: web3::types::Address,
        value: Option<u128>,
        storage: Storage,
        expected: Output,
    ) -> Self {
        Self {
            path,
            hash,
            calldata,
            caller,
            value,
            storage,
            expected,
        }
    }
}

impl DeployEraVM {
    ///
    /// Runs the deploy on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        deployer: &mut D,
        context: InputContext<'_>,
    ) where
        D: EraVMDeployer,
    {
        let test = TestDescription::from_context(
            context,
            InputIdentifier::Deployer {
                contract_identifier: self.path,
            },
        );
        let name = test.selector.path.to_string();
        vm.populate_storage(self.storage.inner);
        let result = match deployer.deploy_eravm::<M>(
            name,
            self.caller,
            self.hash,
            self.calldata.inner.clone(),
            self.value,
            vm,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, test, error);
                return;
            }
        };

        if result.output == self.expected {
            let size = vm.get_contract_size(self.hash) as u64;
            Summary::passed_deploy(
                summary,
                test,
                0,
                size,
                result.cycles,
                result.ergs,
                result.gas,
            );
        } else {
            Summary::failed(
                summary,
                test,
                self.expected,
                result.output,
                self.calldata.inner,
            );
        }
    }
}
