//!
//! The contract call input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::deployers::Deployer;
use crate::zkevm::zkEVM;
use crate::Summary;

use super::calldata::Calldata;
use super::output::Output;
use super::storage::Storage;

///
/// The contract call input variant.
///
#[derive(Debug, Clone)]
pub struct Deploy {
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

impl Deploy {
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

impl Deploy {
    ///
    /// Run the deploy input.
    ///
    pub fn run<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut zkEVM,
        mode: Mode,
        deployer: &mut D,
        test_group: Option<String>,
        name_prefix: String,
    ) where
        D: Deployer,
    {
        let name = format!("{}[#deployer:{}]", name_prefix, self.path,);

        vm.populate_storage(self.storage.inner);
        let result = match deployer.deploy::<M>(
            name.clone(),
            self.caller,
            self.hash,
            self.calldata.inner.clone(),
            self.value,
            vm,
        ) {
            Ok(result) => result,
            Err(error) => {
                Summary::invalid(summary, Some(mode), name, error);
                return;
            }
        };
        if result.output == self.expected {
            let build_size = match vm.get_contract_size(self.hash) {
                Ok(size) => size,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), name, error);
                    return;
                }
            };
            Summary::passed_deploy(summary, mode, name, test_group, build_size, result.cycles);
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.expected,
                result.output,
                self.calldata.inner,
            );
        }
    }
}
