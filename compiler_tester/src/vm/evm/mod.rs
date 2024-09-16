//!
//! The EVM emulator.
//!

pub mod address_iterator;
pub mod input;
pub mod invoker;
pub mod output;
pub mod runtime;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use colored::Colorize;

use crate::vm::execution_result::ExecutionResult;

use self::input::build::Build as EVMBuild;
use self::invoker::Invoker as EVMInvoker;
use self::output::Output as EVMOutput;
use self::runtime::Runtime as EVMRuntime;

///
/// The EVM emulator.
///
pub struct EVM<'evm> {
    /// The EVM runtime.
    runtime: EVMRuntime,
    /// The builds to deploy.
    builds: HashMap<String, EVMBuild>,
    /// The EVM invoker.
    invoker: EVMInvoker<'evm>,
}

impl<'evm> EVM<'evm> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(builds: HashMap<String, EVMBuild>, invoker: EVMInvoker<'evm>) -> Self {
        let runtime = EVMRuntime::default();

        Self {
            runtime,
            builds,
            invoker,
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
        println!(" {} compiler executables", "Downloading".bright_green().bold());
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
    /// Runs a deploy code test transaction.
    ///
    pub fn execute_deploy_code(
        &mut self,
        _test_name: String,
        path: &str,
        caller: web3::types::Address,
        value: Option<u128>,
        constructor_args: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        let build = self.builds.get(path).expect("Always valid");
        let mut deploy_code = build.deploy_build.bytecode.to_owned();
        deploy_code.extend(constructor_args);

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
                evm::ExitSucceed::Returned => (address, false),
                _ => (web3::types::Address::zero(), true),
            },
            Ok(evm::standard::TransactValue::Call { .. }) => {
                unreachable!("The `Create` transaction must be executed above")
            }
            Err(_error) => (web3::types::Address::zero(), true),
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
        _test_name: String,
        address: web3::types::Address,
        caller: web3::types::Address,
        value: Option<u128>,
        calldata: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        self.runtime
            .balances
            .insert(caller, web3::types::U256::max_value()); // TODO

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
                unreachable!("The `Call` transaction must be executed above")
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
