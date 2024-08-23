use std::{convert::Infallible, str::FromStr};

use revm::{
    db::{states::plain_account::PlainStorage, EmptyDBTyped},
    primitives::{Address, FixedBytes, TxKind, B256, U256},
    Evm,
};
use solidity_adapter::EVMVersion;

use crate::{test::case::input::calldata::Calldata, vm::eravm::system_context::SystemContext};

use super::revm_type_conversions::{
    web3_address_to_revm_address, web3_u256_to_revm_address, web3_u256_to_revm_u256,
};

#[derive(Debug)]
pub struct Revm<'a> {
    pub state: Evm<'a, (), revm::State<EmptyDBTyped<Infallible>>>,
}

impl<'a> Default for Revm<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Revm<'a> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        let mut cache = revm::CacheState::new(false);
        // Precompile 0x01 needs to have its code hash
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::from(1_u64),
            code_hash: FixedBytes::from_str(
                "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
            )
            .expect("Always valid"),
            code: None,
            nonce: 1,
        };

        cache.insert_account_with_storage(
            Address::from_word(FixedBytes::from(U256::from(1_u64))),
            acc_info,
            PlainStorage::default(),
        );

        // Account 0x00 needs to have its code hash on 0
        let acc_info_zero = revm::primitives::AccountInfo {
            balance: U256::from(0_u64),
            code_hash: FixedBytes::from(U256::ZERO),
            code: None,
            nonce: 1,
        };

        cache.insert_account_with_storage(
            Address::from_word(FixedBytes::from(U256::ZERO)),
            acc_info_zero,
            PlainStorage::default(),
        );

        let mut state = revm::db::State::builder()
            .with_cached_prestate(cache)
            .with_bundle_update()
            .build();

        // Blocks 0 and 1 need to have their hashes set (revm by default just uses the keccak of the number)
        state.block_hashes.insert(
            1,
            B256::from_str("0x3737373737373737373737373737373737373737373737373737373737373737")
                .unwrap(),
        );
        state.block_hashes.insert(
            0,
            B256::from_str("0x3737373737373737373737373737373737373737373737373737373737373737")
                .unwrap(),
        );

        Self {
            state: revm::Evm::builder().with_db(state).build(),
        }
    }

    pub fn fill_runtime_new_transaction(
        self,
        address: web3::types::Address,
        caller: web3::types::Address,
        calldata: Calldata,
        value: Option<u128>,
        evm_version: Option<EVMVersion>,
    ) -> Self {
        let vm = self
            .state
            .modify()
            .modify_env(|env| {
                let evm_context = SystemContext::get_constants_evm(evm_version);
                env.tx.caller = web3_address_to_revm_address(&caller);
                env.tx.data = revm::primitives::Bytes::from(calldata.inner.clone());
                env.tx.value = revm::primitives::U256::from(value.unwrap_or_default());
                env.tx.transact_to = TxKind::Call(web3_address_to_revm_address(&address));
                env.cfg.chain_id = evm_context.chain_id;
                env.block.number = U256::from(evm_context.block_number);
                let coinbase = web3::types::U256::from_str_radix(evm_context.coinbase, 16).unwrap();
                env.block.coinbase = web3_u256_to_revm_address(coinbase);
                env.block.timestamp = U256::from(evm_context.block_timestamp);
                env.block.gas_limit = U256::from(evm_context.block_gas_limit);
                env.block.basefee = U256::from(evm_context.base_fee);
                let block_difficulty =
                    web3::types::U256::from_str_radix(evm_context.block_difficulty, 16).unwrap();
                env.block.difficulty = web3_u256_to_revm_u256(block_difficulty);
                env.block.prevrandao = Some(B256::from(env.block.difficulty));
                env.tx.gas_price = U256::from(0xb2d05e00_u32);
                env.tx.gas_limit = evm_context.block_gas_limit;
                env.tx.access_list = vec![];
            })
            .build();
        Self { state: vm }
    }

    pub fn fill_deploy_new_transaction(
        self,
        caller: web3::types::Address,
        value: Option<u128>,
        evm_version: Option<EVMVersion>,
        deploy_code: Vec<u8>,
    ) -> Self {
        let vm = self
            .state
            .modify()
            .modify_env(|env| {
                let evm_context = SystemContext::get_constants_evm(evm_version);
                env.cfg.chain_id = evm_context.chain_id;
                env.block.number = U256::from(evm_context.block_number);
                let coinbase = web3::types::U256::from_str_radix(evm_context.coinbase, 16).unwrap();
                env.block.coinbase = web3_u256_to_revm_address(coinbase);
                env.block.timestamp = U256::from(evm_context.block_timestamp);
                env.block.gas_limit = U256::from(evm_context.block_gas_limit);
                env.block.basefee = U256::from(evm_context.base_fee);
                let block_difficulty =
                    web3::types::U256::from_str_radix(evm_context.block_difficulty, 16).unwrap();
                env.block.difficulty = web3_u256_to_revm_u256(block_difficulty);
                env.block.prevrandao = Some(B256::from(env.block.difficulty));
                env.tx.gas_price = U256::from(0xb2d05e00_u32);
                env.tx.gas_limit = evm_context.block_gas_limit;
                env.tx.access_list = vec![];
                env.tx.caller = web3_address_to_revm_address(&caller);
                env.tx.data = revm::primitives::Bytes::from(deploy_code);
                env.tx.value = revm::primitives::U256::from(value.unwrap_or_default());
                env.tx.transact_to = TxKind::Create;
            })
            .build();
        Self { state: vm }
    }
}
