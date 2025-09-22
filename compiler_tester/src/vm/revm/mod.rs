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
    context::ContextTr, context::Evm, context_interface::JournalTr, state::AccountInfo, Database,
};

use crate::vm::revm::revm_type_conversions::web3_u256_to_revm_u256;
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
        let block_hashes = vec![
            SystemContext::ZERO_BLOCK_HASH,
            SystemContext::FIRST_BLOCK_HASH,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, hash)| {
            (
                index as u64,
                revm::primitives::B256::from_str(hash).expect("Always valid"),
            )
        })
        .collect();

        let state = revm::database::State::builder()
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
        evm.block.number = revm::primitives::U256::from(SystemContext::CURRENT_BLOCK_NUMBER);
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
            .gas_price(0xb2d05e00_u128)
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
            .gas_price(0xb2d05e00_u128)
            .gas_limit(SystemContext::BLOCK_GAS_LIMIT_EVM)
            .build_fill()
    }

    ///
    /// Sets the balance at the specified address.
    ///
    pub fn update_balance(&mut self, address: web3::types::Address, balance: web3::types::U256) {
        let address = web3_address_to_revm_address(&address);
        let balance = web3_u256_to_revm_u256(balance);

        let current_balance = self.evm.ctx.balance(address).unwrap_or_default().data;
        let increment_balance = balance.saturating_sub(current_balance);

        self.evm
            .ctx
            .journaled_state
            .balance_incr(address, increment_balance)
            .expect("Always valid");
    }

    ///
    /// REVM needs to send a transaction to execute a contract call,
    /// the balance of the caller is updated to have enough funds to send the transaction.
    ///
    pub fn update_balance_if_lack_of_funds(
        &mut self,
        caller: web3::types::Address,
        fee: revm::primitives::ruint::Uint<256, 4>,
    ) {
        let acc_info = AccountInfo {
            balance: fee,
            code_hash: revm::primitives::KECCAK_EMPTY,
            code: None,
            nonce: 1,
        };
        // self.evm.insert_account_with_storage(
        //     web3_address_to_revm_address(&caller),
        //     acc_info,
        //     PlainStorage::default(),
        // )
    }

    ///
    /// If the caller is not a rich address, subtract the fee
    /// from the balance used only to previoulsy send the transaction.
    ///
    pub fn non_rich_update_balance(&mut self, caller: web3::types::Address) {
        let post_balance = self
            .evm
            .db_mut()
            .basic(web3_address_to_revm_address(&caller))
            .map(|account_info| account_info.map(|info| info.balance).unwrap_or_default())
            .expect("Always valid");
        let balance = (revm::primitives::U256::from(self.evm.tx().gas_limit)
            * revm::primitives::U256::from(self.evm.tx().gas_price))
            - (post_balance + revm::primitives::U256::from(63615000000000u128));
        self.evm
            .ctx
            .journaled_state
            .balance_incr(web3_address_to_revm_address(&caller), balance)
            .expect("Always valid");
    }
}
