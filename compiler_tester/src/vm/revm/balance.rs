use std::str::FromStr;

use revm::{
    db::states::plain_account::PlainStorage,
    primitives::{EVMError, Env, InvalidTransaction, TxKind, B256, KECCAK_EMPTY, U256},
};
use solidity_adapter::EVMVersion;

use crate::{test::case::input::calldata::Calldata, vm::eravm::system_context::SystemContext};

use super::{
    revm_type_conversions::{
        web3_address_to_revm_address, web3_u256_to_revm_address, web3_u256_to_revm_u256,
    },
    Revm,
};
use revm::Database;

impl<'a> Revm<'a> {
    /// All accounts used to deploy the test contracts should have a balance of U256::MAX
    pub fn update_deploy_balance(mut self, account: &web3::types::Address) -> Revm<'a> {
        let address = web3_address_to_revm_address(&account);
        let nonce = match self.state.db_mut().basic(address) {
            Ok(Some(acc)) => acc.nonce,
            _ => 1,
        };
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::MAX,
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
            nonce,
        };
        let mut new_state = self
            .state
            .modify()
            .modify_db(|db| {
                db.insert_account(address, acc_info.clone());
            })
            .modify_env(|env| env.clone_from(&Box::new(Env::default())))
            .build();
        new_state.transact_commit().ok(); // Even if TX fails, the balance update will be committed
        Revm { state: new_state }
    }

    pub fn update_runtime_balance(self, caller: web3::types::Address) -> Self {
        let address = web3_address_to_revm_address(&caller);
        let acc_info = revm::primitives::AccountInfo {
            balance: (U256::from(1) << 100) + U256::from_str("63615000000000").unwrap(),
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
            nonce: 1,
        };
        let mut new_state = self
            .state
            .modify()
            .modify_db(|db| {
                db.insert_account(address, acc_info);
            })
            .modify_env(|env| {
                env.clone_from(&Box::new(Env::default()));
            })
            .build();
        new_state.transact_commit().ok();
        Revm { state: new_state }
    }

    ///
    /// REVM needs to send a transaction to execute a contract call,
    /// the balance of the caller is updated to have enough funds to send the transaction.
    ///
    pub fn update_balance_if_lack_of_funds(mut self, caller: web3::types::Address) -> Revm<'a> {
        if let Err(EVMError::Transaction(InvalidTransaction::LackOfFundForMaxFee {
            fee,
            balance: _balance,
        })) = self.state.transact()
        {
            let acc_info = revm::primitives::AccountInfo {
                balance: *fee,
                code_hash: KECCAK_EMPTY,
                code: None,
                nonce: 1,
            };
            let new_state = self
                .state
                .modify()
                .modify_db(|db| {
                    db.insert_account_with_storage(
                        web3_address_to_revm_address(&caller),
                        acc_info,
                        PlainStorage::default(),
                    );
                })
                .build();
            Revm { state: new_state }
        } else {
            Self { state: self.state }
        }
    }

    ///
    /// If the caller is not a rich address, subtract the fee
    /// from the balance used only to previoulsy send the transaction.
    ///
    pub fn non_rich_update_balance(mut self, caller: web3::types::Address) -> Revm<'a> {
        let post_balance = self
            .state
            .context
            .evm
            .balance(web3_address_to_revm_address(&caller))
            .map_err(|e| self.state.context.evm.error = Err(e))
            .ok()
            .unwrap()
            .0;
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::from(self.state.tx().gas_limit) * self.state.tx().gas_price
                - (post_balance + U256::from_str("63615000000000").unwrap()),
            code_hash: KECCAK_EMPTY,
            code: None,
            nonce: 1,
        };
        let mut new_state = self
            .state
            .modify()
            .modify_db(|db| {
                db.insert_account_with_storage(
                    web3_address_to_revm_address(&caller),
                    acc_info,
                    PlainStorage::default(),
                );
            })
            .build();
        let _ = new_state.transact_commit();
        Revm { state: new_state }
    }
}
