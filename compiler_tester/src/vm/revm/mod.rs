pub mod address_iterator;
pub mod input;
pub mod revm_type_conversions;

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

use colored::Colorize;
use revm::context::Host;
use revm::{
    database::{states::plain_account::PlainStorage, CacheState},
    context::ContextTr, context::Evm, context_interface::JournalTr,
    primitives::{FixedBytes, U256, Address},
    state::AccountInfo,
};

use crate::{test::case::input::calldata::Calldata, vm::eravm::system_context::SystemContext};

use self::revm_type_conversions::web3_address_to_revm_address;

///
/// REVM instance with its internal state.
///
#[derive(Debug)]
pub struct REVM {
    /// REVM internal state.
    pub evm: Evm<
        revm::context::Context<
            revm::context::BlockEnv,
            revm::context::TxEnv,
            revm::context::CfgEnv,
            revm::database::State<revm::database::EmptyDB>,
            revm::context::Journal<revm::database::State<revm::database::EmptyDB>>,
            (),
            revm::context::LocalContext,
        >,
        (),
        revm::handler::instructions::EthInstructions<
            revm::interpreter::interpreter::EthInterpreter,
            revm::context::Context<
                revm::context::BlockEnv,
                revm::context::TxEnv,
                revm::context::CfgEnv,
                revm::database::State<revm::database::EmptyDB>,
                revm::context::Journal<revm::database::State<revm::database::EmptyDB>>,
                (),
                revm::context::LocalContext,
            >,
        >,
        revm::handler::EthPrecompiles,
        revm::handler::EthFrame,
    >,
}

impl Default for REVM {
    fn default() -> Self {
        Self::new()
    }
}

impl REVM {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        let mut cache = CacheState::new(false);
        // Account 0x00 needs to have its code hash on 0.
        cache.insert_account_with_storage(
            Address::from_word(FixedBytes::from(U256::ZERO)),
            AccountInfo {
            balance: U256::from(0_u64),
            code_hash: FixedBytes::from(U256::ZERO),
            code: None,
            nonce: 1,
        },
            PlainStorage::default(),
        );
        // Precompile 0x01 needs to have its code hash.
        cache.insert_account_with_storage(
            Address::from_word(FixedBytes::from(U256::from(1_u64))),
            AccountInfo {
            balance: U256::from(1_u64),
            code_hash: FixedBytes::from_str(
                "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
            )
            .expect("Always valid"),
            code: None,
            nonce: 1,
        },
            PlainStorage::default(),
        );

        let block_hashes = (0..0xffff - 0x3737)
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let hash = format!("0x373737373737373737373737373737373737373737373737373737373737{:04x}", 0x3737 + value);
                (
                    index as u64,
                    revm::primitives::B256::from_str(hash.as_str()).expect("Always valid"),
                )
            })
            .collect();

        let state = revm::database::State::builder()
            .with_cached_prestate(cache)
            .with_block_hashes(block_hashes)
            .with_bundle_update()
            .build();

        let context: revm::context::Context<
            revm::context::BlockEnv,
            revm::context::TxEnv,
            revm::context::CfgEnv,
            revm::database::State<revm::database::EmptyDB>,
            revm::context::Journal<revm::database::State<revm::database::EmptyDB>>,
            (),
            revm::context::LocalContext,
        > = revm::context::Context::new(state, revm::primitives::hardfork::PRAGUE);

        let mut evm = revm::context::Evm::new(
            context,
            revm::handler::instructions::EthInstructions::new_mainnet(),
            revm::handler::EthPrecompiles::default(),
        );
        evm.block.beneficiary = revm::primitives::Address::from_str(SystemContext::COIN_BASE_EVM)
            .expect("Always valid");
        evm.block.basefee = SystemContext::BASE_FEE;
        evm.block.prevrandao = Some(
            revm::primitives::B256::from_str(SystemContext::BLOCK_DIFFICULTY_POST_PARIS)
                .expect("Always valid"),
        );
        evm.block.gas_limit = SystemContext::BLOCK_GAS_LIMIT_EVM;
        evm.block.timestamp =
            revm::primitives::U256::from(SystemContext::CURRENT_BLOCK_TIMESTAMP_EVM);
        evm.tx.chain_id = Some(SystemContext::CHAIND_ID_EVM);
        evm.cfg.disable_nonce_check = true;
        Self { evm }
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
    pub fn new_deploy_transaction(
        caller: web3::types::Address,
        value: Option<u128>,
        code: Vec<u8>,
    ) -> revm::context::TxEnv {
        revm::context::TxEnv::builder()
            .caller(web3_address_to_revm_address(&caller))
            .data(revm::primitives::Bytes::from(code))
            .value(revm::primitives::U256::from(value.unwrap_or_default()))
            .create()
            .gas_price(SystemContext::GAS_PRICE as u128)
            .gas_limit(SystemContext::BLOCK_GAS_LIMIT_EVM)
            .build_fill()
    }

    ///
    /// Fills a runtime transaction with the given parameters.
    ///
    pub fn new_runtime_transaction(
        address: web3::types::Address,
        caller: web3::types::Address,
        calldata: Calldata,
        value: Option<u128>,
    ) -> revm::context::TxEnv {
        revm::context::TxEnv::builder()
            .caller(web3_address_to_revm_address(&caller))
            .data(revm::primitives::Bytes::from(calldata.inner))
            .value(revm::primitives::U256::from(value.unwrap_or_default()))
            .to(web3_address_to_revm_address(&address))
            .gas_price(SystemContext::GAS_PRICE as u128)
            .gas_limit(SystemContext::BLOCK_GAS_LIMIT_EVM)
            .build_fill()
    }

    ///
    /// Sets the balance at the specified address.
    ///
    pub fn update_balance(&mut self, address: web3::types::Address, balance: revm::primitives::U256) {
        let address = web3_address_to_revm_address(&address);

        let current_balance = self.evm.ctx.balance(address).unwrap_or_default().data;
        let increment_balance = balance.saturating_sub(current_balance);

        self.evm
            .ctx
            .journaled_state
            .balance_incr(address, increment_balance)
            .expect("Always valid");
    }

    ///
    /// If the caller is not a rich address, subtract the fee
    /// from the balance used only to previoulsy send the transaction.
    ///
    pub fn non_rich_update_balance(&mut self, address: web3::types::Address) {
        let address = web3_address_to_revm_address(&address);
        
        let increment_balance = revm::primitives::U256::from(self.evm.tx().gas_limit)
            * revm::primitives::U256::from(self.evm.tx().gas_price);

        self.evm
            .ctx
            .journaled_state
            .balance_incr(address, increment_balance)
            .expect("Always valid");
    }
}
