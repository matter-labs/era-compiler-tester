//!
//! The EVM wrapper.
//!

pub mod address_predictor;
pub mod input;
pub mod invoker;
pub mod output;
pub mod runtime;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use colored::Colorize;

use crate::compilers::downloader::Downloader as CompilerDownloader;
use crate::vm::execution_result::ExecutionResult;

use self::input::build::Build as EVMBuild;
use self::invoker::Invoker as EVMInvoker;
use self::output::Output as EVMOutput;
use self::runtime::Runtime as EVMRuntime;

///
/// The EVM wrapper.
///
#[allow(non_camel_case_types)]
pub struct EVM<'evm> {
    /// The EVM runtime.
    runtime: EVMRuntime,
    /// The known contracts.
    known_contracts: HashMap<web3::types::Address, EVMBuild>,
    /// The EVM invoker.
    invoker: EVMInvoker<'evm>,
}

impl<'evm> EVM<'evm> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        known_contracts: HashMap<web3::types::Address, EVMBuild>,
        invoker: EVMInvoker<'evm>,
    ) -> Self {
        let runtime = EVMRuntime::default();

        Self {
            runtime,
            known_contracts,
            invoker,
        }
    }

    ///
    /// Downloads the necessary compiler binaries.
    ///
    pub fn download(binary_download_config_paths: Vec<PathBuf>) -> anyhow::Result<()> {
        let mut http_client_builder = reqwest::blocking::ClientBuilder::new();
        http_client_builder = http_client_builder.connect_timeout(Duration::from_secs(60));
        http_client_builder = http_client_builder.pool_idle_timeout(Duration::from_secs(60));
        http_client_builder = http_client_builder.timeout(Duration::from_secs(60));
        let http_client = http_client_builder.build()?;

        let download_time_start = Instant::now();
        println!(" {} compiler binaries", "Downloading".bright_green().bold());
        for config_path in binary_download_config_paths.into_iter() {
            CompilerDownloader::new(http_client.clone()).download(config_path.as_path())?;
        }
        println!(
            "    {} downloading compiler binaries in {}m{:02}s",
            "Finished".bright_green().bold(),
            download_time_start.elapsed().as_secs() / 60,
            download_time_start.elapsed().as_secs() % 60,
        );

        Ok(())
    }

    ///
    /// Runs a deploy code test transaction.
    ///
    pub fn execute_deploy_code(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        value: Option<u128>,
        constructor_args: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        let bytecode = self.known_contracts.values().next().unwrap();
        let mut deploy_code = bytecode.deploy_build.bytecode.to_owned();
        deploy_code.extend(constructor_args);
        let runtime_code = bytecode.runtime_build.bytecode.to_owned();

        self.runtime
            .balances
            .insert(caller, web3::types::U256::max_value());

        let (address, exception) = match evm::transact(
            evm::standard::TransactArgs::Create {
                caller,
                value: value.unwrap_or_default().into(),
                init_code: deploy_code,
                salt: None,
                gas_limit: web3::types::U256::from_str_radix(
                    "ffffffff",
                    era_compiler_common::BASE_HEXADECIMAL,
                )
                .expect("Always valid"),
                gas_price: web3::types::U256::from_str_radix(
                    "b2d05e00",
                    era_compiler_common::BASE_HEXADECIMAL,
                )
                .expect("Always valid"),
                access_list: vec![],
            },
            None,
            &mut self.runtime,
            &self.invoker,
        ) {
            Ok(evm::standard::TransactValue::Create { succeed, address }) => match succeed {
                evm::ExitSucceed::Returned => {
                    self.runtime.codes.insert(address, runtime_code.clone());
                    (address, false)
                }
                _ => (web3::types::Address::zero(), true),
            },
            Ok(evm::standard::TransactValue::Call { .. }) => {
                panic!("Unreachable due to the `Create` transaction sent above")
            }
            Err(error) => (web3::types::Address::zero(), true),
        };

        let mut return_data = vec![
            0u8;
            era_compiler_common::BYTE_LENGTH_FIELD
                - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS
        ];
        return_data.extend(address.as_fixed_bytes());
        let events = self.runtime.logs.drain(..).collect();
        let output = EVMOutput::new(return_data, exception, events);

        let execution_result = ExecutionResult::from(output);
        Ok(execution_result)
    }

    ///
    /// Runs a runtime code transaction.
    ///
    pub fn execute_runtime_code(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        value: Option<u128>,
        calldata: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        self.runtime
            .balances
            .insert(caller, web3::types::U256::max_value());

        let address = self.runtime.codes.iter().next().unwrap().0.to_owned();

        let (return_data, exception) = match evm::transact(
            evm::standard::TransactArgs::Call {
                caller,
                address,
                value: value.unwrap_or_default().into(),
                data: calldata,
                gas_limit: web3::types::U256::from_str_radix(
                    "ffffffff",
                    era_compiler_common::BASE_HEXADECIMAL,
                )
                .expect("Always valid"),
                gas_price: web3::types::U256::from_str_radix(
                    "b2d05e00",
                    era_compiler_common::BASE_HEXADECIMAL,
                )
                .expect("Always valid"),
                access_list: vec![],
            },
            None,
            &mut self.runtime,
            &self.invoker,
        ) {
            Ok(evm::standard::TransactValue::Call { succeed, retval }) => {
                (retval, succeed != evm::ExitSucceed::Returned)
            }
            Ok(evm::standard::TransactValue::Create { .. }) => {
                panic!("Unreachable due to the `Call` transaction sent above")
            }
            Err(_error) => (vec![], true),
        };

        let events = self.runtime.logs.drain(..).collect();
        let output = EVMOutput::new(return_data, exception, events);

        let execution_result = ExecutionResult::from(output);
        Ok(execution_result)
    }

    ///
    /// Adds values to storage.
    ///
    pub fn populate_storage(
        &mut self,
        values: HashMap<(web3::types::Address, web3::types::U256), web3::types::H256>,
    ) {
        for ((address, key), value) in values.into_iter() {
            self.runtime
                .storages
                .entry(address)
                .or_default()
                .insert(crate::utils::u256_to_h256(&key), value);
        }
    }
}
