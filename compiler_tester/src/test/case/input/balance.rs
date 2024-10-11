//!
//! The balance check input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::vm::eravm::system_context::SystemContext;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;
use crate::vm::revm::revm_type_conversions::web3_address_to_revm_address;
use crate::vm::revm::Revm;

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
        vm: &mut EraVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{name_prefix}[#balance_check:{index}]");

        let rich_addresses = SystemContext::get_rich_addresses();
        if rich_addresses.contains(&self.address) {
            vm.mint_ether(
                self.address,
                web3::types::U256::from_str_radix(
                    "10000000000000000000000000",
                    era_compiler_common::BASE_HEXADECIMAL,
                )
                .expect("Always valid"),
            );
        }

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
    /// Runs the balance check on EVM emulator.
    ///
    pub fn run_evm_emulator(
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
    pub fn run_revm(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut Revm,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{name_prefix}[#balance_check:{index}]");
        let found = vm
            .state
            .context
            .evm
            .balance(web3_address_to_revm_address(&self.address));
        match found {
            Ok(found) => {
                let u256_found = web3::types::U256::from(found.data.to_be_bytes());
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
                    web3::types::U256::zero().into(),
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
