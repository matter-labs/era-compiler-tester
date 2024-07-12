//!
//! The storage emptiness check input variant.
//!

use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::cache_account;
use revm::db::states::plain_account;
use revm::db::State;
use revm::Database;
use revm::DatabaseCommit;

use crate::compilers::mode::Mode;
use crate::summary::Summary;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;

///
/// The storage emptiness check input variant.
///
#[derive(Debug, Clone)]
pub struct StorageEmpty {
    /// Whether storage is empty.
    is_empty: bool,
}

impl StorageEmpty {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(is_empty: bool) -> Self {
        Self { is_empty }
    }
}

impl StorageEmpty {
    ///
    /// Runs the storage empty check on EraVM.
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
        let name = format!("{name_prefix}[#storage_empty_check:{index}]");

        let found = vm.is_storage_empty();
        if found == self.is_empty {
            Summary::passed_special(summary, mode, name, test_group);
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.is_empty.into(),
                found.into(),
                vec![],
            );
        }
    }

    ///
    /// Runs the storage empty check on EVM.
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
    /// Runs the storage empty check on REVM.
    ///
    pub fn run_revm<EXT, DB: Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut revm::Evm<EXT, State<DB>>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        let name = format!("{name_prefix}[#storage_empty_check:{index}]");

        let mut is_empty = true;
        for (address, cache_account) in &vm.db().cache.accounts {
            let plain_account = cache_account.clone().account;
            if plain_account.is_some() {
                for (key, value) in plain_account.unwrap().storage.iter() {
                    if !value.is_zero() {
                        is_empty = false;
                    }
                }
            }
        }
        if is_empty == self.is_empty {
            Summary::passed_special(summary, mode, name, test_group);
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.is_empty.into(),
                is_empty.into(),
                vec![],
            );
        }
    }

    ///
    /// Runs the storage empty check on EVM interpreter.
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
        let name = format!("{name_prefix}[#storage_empty_check:{index}]");

        let found = vm.is_storage_empty();
        if found == self.is_empty {
            Summary::passed_special(summary, mode, name, test_group);
        } else {
            Summary::failed(
                summary,
                mode,
                name,
                self.is_empty.into(),
                found.into(),
                vec![],
            );
        }
    }
}
