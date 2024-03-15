//!
//! The balance check input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;
use crate::Summary;

///
/// The balance check input variant.
///
#[derive(Debug, Clone)]
pub struct Balance {
    /// The account address.
    address: web3::types::Address,
    /// The expected balance.
    balance: web3::types::U256,
}

impl Balance {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(address: web3::types::Address, balance: web3::types::U256) -> Self {
        Self { address, balance }
    }
}

impl Balance {
    ///
    /// Runs the balance check on EraVM.
    ///
    pub fn run_eravm(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &EraVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{name_prefix}[#balance_check:{index}]");
        let found = vm.get_balance(self.address);
        if found == self.balance {
            Summary::passed_special(summary, mode, name, test_group);
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.balance.into(),
                found.into(),
                self.address.to_fixed_bytes().to_vec(),
            );
        }
    }

    ///
    /// Runs the balance check on EVM.
    ///
    pub fn run_evm(
        self,
        summary: Arc<Mutex<Summary>>,
        _vm: &EVM,
        mode: Mode,
        _test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        // TODO: get balance from EVM
        let name = format!("{name_prefix}[#balance_check:{index}]");
        Summary::failed(
            summary,
            mode,
            name,
            self.balance.into(),
            self.balance.into(),
            self.address.to_fixed_bytes().to_vec(),
        );
    }
}
