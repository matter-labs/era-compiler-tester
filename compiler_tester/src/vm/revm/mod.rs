pub mod address_iterator;
pub mod balance;
pub mod input;
pub mod revm_type_conversions;

use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

use colored::Colorize;
use revm::{
    context::Evm,
    database::{states::plain_account::PlainStorage, EmptyDBTyped},
    primitives::{Address, FixedBytes, TxKind, B256, U256},
    state::EvmState,
};

use solidity_adapter::EVMVersion;

use crate::{test::case::input::calldata::Calldata, vm::eravm::system_context::SystemContext};

use self::revm_type_conversions::{
    web3_address_to_revm_address, web3_u256_to_revm_address, web3_u256_to_revm_u256,
};

///
/// REVM instance with its internal state.
///
#[derive(Debug)]
pub struct REVM<'a> {
    /// REVM internal state.
    pub state: Evm<(), EvmState<EmptyDBTyped<Infallible>>>,
}

impl Default for REVM<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl REVM<'_> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        let mut cache = revm::database::CacheState::new(false);
        // Precompile 0x01 needs to have its code hash
        let acc_info = revm::state::AccountInfo {
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
        let acc_info_zero = revm::state::AccountInfo {
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

        let mut state = revm::database::State::builder()
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
            state: revm::context::Evm::new(state),
        }
    }

    ///
    /// Downloads the necessary compiler executables.
    ///
    pub fn download(executable_download_config_paths: Vec<PathBuf>) -> anyhow::Result<()> {
        let mut http_client_builder = reqwest::blocking::ClientBuilder::new();
        http_client_builder = http_client_builder.connect_timeout(Duration::from_secs(60));
        http_client_builder = http_client_builder.pool_idle_timeout(Duration::from_secs(60));
        http_client_builder = http_client_builder.timeout(Duration::from_secs(60));
        let http_client = http_client_builder.build()?;

        let download_time_start = Instant::now();
        println!(
            " {} compiler executables",
            "Downloading".bright_green().bold()
        );
        for config_path in executable_download_config_paths.into_iter() {
            era_compiler_downloader::Downloader::new(http_client.clone())
                .download(config_path.as_path())?;
        }
        println!(
            "    {} downloading compiler executables in {}m{:02}s",
            "Finished".bright_green().bold(),
            download_time_start.elapsed().as_secs() / 60,
            download_time_start.elapsed().as_secs() % 60,
        );

        Ok(())
    }

    ///
    /// Fills a deploy transaction with the given parameters.
    ///
    pub fn fill_deploy_new_transaction(
        self,
        caller: web3::types::Address,
        value: Option<u128>,
        evm_version: Option<EVMVersion>,
        code: Vec<u8>,
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
                env.tx.data = revm::primitives::Bytes::from(code);
                env.tx.value = revm::primitives::U256::from(value.unwrap_or_default());
                env.tx.transact_to = TxKind::Create;
            })
            .build();
        Self { state: vm }
    }

    ///
    /// Fills a runtime transaction with the given parameters.
    ///
    pub fn fill_runtime_new_transaction(
        self,
        address: web3::types::Address,
        caller: web3::types::Address,
        calldata: Calldata,
        value: Option<u128>,
        evm_version: Option<EVMVersion>,
        input_index: usize,
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
                env.block.number = U256::from(input_index);
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
}
