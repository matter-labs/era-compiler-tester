//!
//! The test input.
//!

pub mod balance;
pub mod calldata;
pub mod deploy;
pub mod output;
pub mod runtime;
pub mod storage;
pub mod storage_empty;
pub mod value;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::deployers::Deployer;
use crate::directories::matter_labs::test::metadata::case::input::Input as MatterLabsTestInput;
use crate::summary::Summary;
use crate::test::instance::Instance;
use crate::zkevm::zkEVM;

use self::balance::Balance;
use self::calldata::Calldata;
use self::deploy::Deploy;
use self::output::Output;
use self::runtime::Runtime;
use self::storage::Storage;
use self::storage_empty::StorageEmpty;

///
/// The test input.
///
#[derive(Debug, Clone)]
pub enum Input {
    /// The contract call.
    Runtime(Runtime),
    /// The contract deploy.
    Deploy(Deploy),
    /// The storage empty check.
    StorageEmpty(StorageEmpty),
    /// Check account balance.
    Balance(Balance),
}

impl Input {
    ///
    /// Try convert from Matter Labs compiler test metadata input.
    ///
    pub fn try_from_matter_labs(
        input: &MatterLabsTestInput,
        mode: &Mode,
        instances: &HashMap<String, Instance>,
        method_identifiers: &Option<BTreeMap<String, BTreeMap<String, u32>>>,
    ) -> anyhow::Result<Self> {
        let caller = web3::types::Address::from_str(input.caller.as_str())
            .map_err(|error| anyhow::anyhow!("Invalid caller: {}", error))?;

        let value = match input.value.as_ref() {
            Some(value) => Some(if let Some(value) = value.strip_suffix(" ETH") {
                u128::from_str(value)
                    .map_err(|error| anyhow::anyhow!("Invalid value literal: {}", error))?
                    .checked_mul(10u128.pow(18))
                    .ok_or_else(|| anyhow::anyhow!("Overflow: value too big"))?
            } else if let Some(value) = value.strip_suffix(" wei") {
                u128::from_str(value)
                    .map_err(|error| anyhow::anyhow!("Invalid value literal: {}", error))?
            } else {
                anyhow::bail!("Invalid value");
            }),
            None => None,
        };

        let mut calldata = Calldata::try_from_matter_labs(&input.calldata, instances)
            .map_err(|error| anyhow::anyhow!("Invalid calldata: {}", error))?;

        let expected = match input.expected.as_ref() {
            Some(expected) => Output::try_from_matter_labs_expected(expected, mode, instances)
                .map_err(|error| anyhow::anyhow!("Invalid expected: {}", error))?,
            None => Output::default(),
        };

        let storage = Storage::try_from_matter_labs(&input.storage, instances)
            .map_err(|error| anyhow::anyhow!("Invalid storage: {}", error))?;

        let instance = instances
            .get(&input.instance)
            .ok_or_else(|| anyhow::anyhow!("Instance `{}` not found", input.instance))?;

        let input = match input.method.as_str() {
            "#deployer" => Input::Deploy(Deploy::new(
                instance.path.to_owned(),
                instance.code_hash,
                calldata,
                caller,
                value,
                storage,
                expected,
            )),
            "#fallback" => {
                let address = instance.address.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Instance `{}` was not successfully deployed",
                        input.instance
                    )
                })?;

                Input::Runtime(Runtime::new(
                    "#fallback".to_string(),
                    address,
                    calldata,
                    caller,
                    value,
                    storage,
                    expected,
                ))
            }
            entry => {
                let address = instance.address.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Instance `{}` was not successfully deployed",
                        input.instance
                    )
                })?;
                let path = instance.path.as_str();
                let selector = match method_identifiers {
                    Some(method_identifiers) => method_identifiers
                        .get(path)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Contract {} not found in the method identifiers", path)
                        })?
                        .iter()
                        .find_map(|(name, selector)| {
                            if name.starts_with(entry) {
                                Some(*selector)
                            } else {
                                None
                            }
                        })
                        .ok_or_else(|| {
                            anyhow::anyhow!("Selector of the method `{}` not found", entry)
                        })?,
                    None => u32::from_str_radix(entry, compiler_common::BASE_HEXADECIMAL)
                        .map_err(|err| anyhow::anyhow!("Invalid entry value: {}", err))?,
                };

                calldata.add_selector(selector);

                Input::Runtime(Runtime::new(
                    entry.to_string(),
                    address,
                    calldata,
                    caller,
                    value,
                    storage,
                    expected,
                ))
            }
        };

        Ok(input)
    }

    ///
    /// Try convert from Ethereum compiler test metadata input.
    ///
    pub fn try_from_ethereum(
        input: &solidity_adapter::FunctionCall,
        main_contract_instance: &Instance,
        libraries_instances: &HashMap<String, Instance>,
        last_source: &str,
        caller: &web3::types::Address,
    ) -> anyhow::Result<Option<Self>> {
        let main_contract_address = main_contract_instance
            .address
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Internal error: main contract address is none"))?;
        let input = match input {
            solidity_adapter::FunctionCall::Constructor {
                calldata,
                value,
                events,
                ..
            } => {
                let value = match value {
                    Some(value) => Some(
                        (*value)
                            .try_into()
                            .map_err(|error| anyhow::anyhow!("Value is too big: {}", error))?,
                    ),
                    None => None,
                };

                let expected = Output::from_ethereum_expected(
                    &[web3::types::U256::from_big_endian(
                        main_contract_address.as_bytes(),
                    )],
                    false,
                    events,
                    main_contract_address,
                );

                Some(Input::Deploy(Deploy::new(
                    main_contract_instance.path.to_owned(),
                    main_contract_instance.code_hash,
                    calldata.clone().into(),
                    *caller,
                    value,
                    Storage::default(),
                    expected,
                )))
            }
            solidity_adapter::FunctionCall::Library { name, source } => {
                let library = format!(
                    "{}:{}",
                    source.clone().unwrap_or_else(|| last_source.to_string()),
                    name
                );
                let instance = libraries_instances.get(library.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("Internal error: Library {} not found", library)
                })?;
                let hash = instance.code_hash;
                let address = instance
                    .address
                    .ok_or_else(|| anyhow::anyhow!("Internal error: library address is none"))?;

                let expected = Output::from_ethereum_expected(
                    &[web3::types::U256::from_big_endian(address.as_bytes())],
                    false,
                    &[],
                    main_contract_address,
                );

                Some(Input::Deploy(Deploy::new(
                    instance.path.to_owned(),
                    hash,
                    Calldata::default(),
                    *caller,
                    None,
                    Storage::default(),
                    expected,
                )))
            }
            solidity_adapter::FunctionCall::Balance {
                input, expected, ..
            } => {
                let address = input.unwrap_or(*main_contract_address);
                Some(Input::Balance(Balance::new(address, *expected)))
            }
            solidity_adapter::FunctionCall::StorageEmpty { expected } => {
                Some(Input::StorageEmpty(StorageEmpty::new(*expected)))
            }
            solidity_adapter::FunctionCall::Call {
                method,
                calldata,
                value,
                expected,
                failure,
                events,
                ..
            } => {
                let value = match value {
                    Some(value) => Some(
                        (*value)
                            .try_into()
                            .map_err(|error| anyhow::anyhow!("Value is too big: {}", error))?,
                    ),
                    None => None,
                };

                let expected = Output::from_ethereum_expected(
                    expected,
                    *failure,
                    events,
                    main_contract_address,
                );

                Some(Input::Runtime(Runtime::new(
                    method.clone(),
                    *main_contract_address,
                    calldata.clone().into(),
                    *caller,
                    value,
                    Storage::default(),
                    expected,
                )))
            }
            _ => None,
        };

        Ok(input)
    }

    ///
    /// Run the input.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn run<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut zkEVM,
        mode: Mode,
        deployer: &mut D,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) where
        D: Deployer,
    {
        match self {
            Self::Runtime(runtime) => {
                runtime.run::<M>(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::Deploy(deploy) => {
                deploy.run::<_, M>(summary, vm, mode, deployer, test_group, name_prefix)
            }
            Self::StorageEmpty(storage_empty) => {
                storage_empty.run(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::Balance(balance_check) => {
                balance_check.run(summary, vm, mode, test_group, name_prefix, index)
            }
        };
    }
}
