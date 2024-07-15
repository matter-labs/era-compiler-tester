//!
//! The balance check input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use revm::db::State;
use web3::types::U256;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;

use super::revm_type_conversions::web3_address_to_revm_address;

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
        _summary: Arc<Mutex<Summary>>,
        _vm: &EVM,
        _mode: Mode,
        _test_group: Option<String>,
        _name_prefix: String,
        _index: usize,
    ) {
        todo!()
    }

    ///
    /// Runs the balance check on REVM.
    ///
    pub fn run_revm<'a, EXT, DB: revm::db::Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut revm::Evm<'a, EXT, State<DB>>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{name_prefix}[#balance_check:{index}]");
        let found = vm
            .context
            .evm
            .balance(web3_address_to_revm_address(&self.address));
        match found {
            Ok(found) => {
                let u256_found = U256::from(found.0.to_be_bytes());
                if u256_found == self.balance {
                    Summary::passed_special(summary, mode, name, test_group);
                } else {
                    Summary::failed(
                        summary,
                        mode,
                        name,
                        self.balance.into(),
                        u256_found.into(),
                        self.address.to_fixed_bytes().to_vec(),
                    );
                }
            }
            Err(_) => {
                Summary::failed(
                    summary,
                    mode,
                    name,
                    self.balance.into(),
                    U256::zero().into(),
                    self.address.to_fixed_bytes().to_vec(),
                );
            }
        }
    }

    ///
    /// Runs the balance check on EVM interpreter.
    ///
    pub fn run_evm_interpreter(
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
}
