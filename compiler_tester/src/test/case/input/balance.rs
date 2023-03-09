//!
//! The balance check input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::zkevm::zkEVM;
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
    /// Run the balance check input.
    ///
    pub fn run(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &zkEVM,
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
}
