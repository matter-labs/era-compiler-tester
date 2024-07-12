//!
//! The test input.
//!

pub mod balance;
pub mod calldata;
pub mod deploy_eravm;
pub mod deploy_evm;
pub mod output;
pub mod revm_type_conversions;
pub mod runtime;
pub mod storage;
pub mod storage_empty;
pub mod value;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::RandomState;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::State;
use revm::Database;

use solidity_adapter::EVMVersion;

use crate::compilers::mode::Mode;
use crate::directories::matter_labs::test::metadata::case::input::Input as MatterLabsTestInput;
use crate::summary::Summary;
use crate::test::instance::Instance;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build;
use crate::vm::evm::EVM;

use self::balance::Balance;
use self::calldata::Calldata;
use self::deploy_eravm::DeployEraVM;
use self::deploy_evm::DeployEVM;
use self::output::Output;
use self::runtime::Runtime;
use self::storage::Storage;
use self::storage_empty::StorageEmpty;

///
/// The test input.
///
#[derive(Debug, Clone)]
pub enum Input {
    /// The EraVM contract deploy.
    DeployEraVM(DeployEraVM),
    /// The EVM contract deploy.
    DeployEVM(DeployEVM),
    /// The contract call.
    Runtime(Runtime),
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
        input: MatterLabsTestInput,
        mode: &Mode,
        instances: &BTreeMap<String, Instance>,
        method_identifiers: &Option<BTreeMap<String, BTreeMap<String, u32>>>,
    ) -> anyhow::Result<Self> {
        let caller = web3::types::Address::from_str(input.caller.as_str())
            .map_err(|error| anyhow::anyhow!("Invalid caller `{}`: {}", input.caller, error))?;

        let value = match input.value {
            Some(value) => Some(if let Some(value) = value.strip_suffix(" ETH") {
                u128::from_str(value)
                    .map_err(|error| anyhow::anyhow!("Invalid value literal `{value}`: {}", error))?
                    .checked_mul(10u128.pow(18))
                    .ok_or_else(|| {
                        anyhow::anyhow!("Invalid value literal `{value}`: u128 overflow")
                    })?
            } else if let Some(value) = value.strip_suffix(" wei") {
                u128::from_str(value).map_err(|error| {
                    anyhow::anyhow!("Invalid value literal `{value}`: {}", error)
                })?
            } else {
                anyhow::bail!("Invalid value `{value}`");
            }),
            None => None,
        };

        let mut calldata = Calldata::try_from_matter_labs(input.calldata, instances)
            .map_err(|error| anyhow::anyhow!("Invalid calldata: {}", error))?;

        let expected = match input.expected {
            Some(expected) => Output::try_from_matter_labs_expected(expected, mode, instances)
                .map_err(|error| anyhow::anyhow!("Invalid expected metadata: {}", error))?,
            None => Output::default(),
        };

        let storage = Storage::try_from_matter_labs(input.storage, instances)
            .map_err(|error| anyhow::anyhow!("Invalid storage: {}", error))?;

        let instance = instances
            .get(&input.instance)
            .ok_or_else(|| anyhow::anyhow!("Instance `{}` not found", input.instance))?;

        let input = match input.method.as_str() {
            "#deployer" => match instance {
                Instance::EraVM(instance) => Input::DeployEraVM(DeployEraVM::new(
                    instance.path.to_owned(),
                    instance.code_hash,
                    calldata,
                    caller,
                    value,
                    storage,
                    expected,
                )),
                Instance::EVM(instance) => Input::DeployEVM(DeployEVM::new(
                    instance.path.to_owned(),
                    instance.init_code.to_owned(),
                    calldata,
                    caller,
                    value,
                    storage,
                    expected,
                )),
            },
            "#fallback" => {
                let address = instance.address().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Instance `{}` was not successfully deployed",
                        input.instance
                    )
                })?;

                Input::Runtime(Runtime::new(
                    "#fallback".to_string(),
                    *address,
                    calldata,
                    caller,
                    value,
                    storage,
                    expected,
                ))
            }
            entry => {
                let address = instance.address().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Instance `{}` was not successfully deployed",
                        input.instance
                    )
                })?;

                let path = instance.path();
                let selector = match method_identifiers {
                    Some(method_identifiers) => method_identifiers
                        .get(path)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Contract `{}` not found in the method identifiers",
                                path
                            )
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
                            anyhow::anyhow!(
                                "In the contract `{}`, selector of the method `{}` not found",
                                path,
                                entry
                            )
                        })?,
                    None => u32::from_str_radix(entry, era_compiler_common::BASE_HEXADECIMAL)
                        .map_err(|error| {
                            anyhow::anyhow!(
                                "Invalid entry value for contract `{}`: {}",
                                path,
                                error
                            )
                        })?,
                };

                calldata.push_selector(selector);

                Input::Runtime(Runtime::new(
                    entry.to_string(),
                    *address,
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
        instances: &BTreeMap<String, Instance>,
        last_source: &str,
        caller: &web3::types::Address,
    ) -> anyhow::Result<Option<Self>> {
        let main_contract_instance = instances
            .values()
            .find(|instance| instance.is_main())
            .ok_or_else(|| anyhow::anyhow!("Could not identify the Ethereum test main contract"))?
            .to_owned();
        let main_contract_address = main_contract_instance.address().expect("Always exists");

        let input = match input {
            solidity_adapter::FunctionCall::Constructor {
                calldata,
                value,
                events,
                ..
            } => {
                let value = match value {
                    Some(value) => Some((*value).try_into().map_err(|error| {
                        anyhow::anyhow!("Invalid value literal `{:X}`: {}", value, error)
                    })?),
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

                match main_contract_instance {
                    Instance::EraVM(instance) => Some(Input::DeployEraVM(DeployEraVM::new(
                        instance.path.to_owned(),
                        instance.code_hash,
                        calldata.clone().into(),
                        *caller,
                        value,
                        Storage::default(),
                        expected,
                    ))),
                    Instance::EVM(instance) => Some(Input::DeployEVM(DeployEVM::new(
                        instance.path.to_owned(),
                        instance.init_code.to_owned(),
                        calldata.clone().into(),
                        *caller,
                        value,
                        Storage::default(),
                        expected,
                    ))),
                }
            }
            solidity_adapter::FunctionCall::Library { name, source } => {
                let library = format!(
                    "{}:{}",
                    source.clone().unwrap_or_else(|| last_source.to_string()),
                    name
                );
                let instance = instances
                    .get(library.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Library `{}` not found", library))?;

                let expected = Output::from_ethereum_expected(
                    &[web3::types::U256::from_big_endian(
                        instance
                            .address()
                            .expect("Must be set by this point")
                            .as_bytes(),
                    )],
                    false,
                    &[],
                    main_contract_address,
                );

                match instance {
                    Instance::EraVM(instance) => Some(Input::DeployEraVM(DeployEraVM::new(
                        instance.path.to_owned(),
                        instance.code_hash,
                        Calldata::default(),
                        *caller,
                        None,
                        Storage::default(),
                        expected,
                    ))),
                    Instance::EVM(instance) => Some(Input::DeployEVM(DeployEVM::new(
                        instance.path.to_owned(),
                        instance.init_code.to_owned(),
                        Calldata::default(),
                        *caller,
                        None,
                        Storage::default(),
                        expected,
                    ))),
                }
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
                    Some(value) => Some((*value).try_into().map_err(|error| {
                        anyhow::anyhow!("Invalid value literal `{:X}`: {}", value, error)
                    })?),
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
    /// Runs the input on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        mode: Mode,
        deployer: &mut D,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) where
        D: EraVMDeployer,
    {
        match self {
            Self::DeployEraVM(deploy) => {
                deploy.run_eravm::<_, M>(summary, vm, mode, deployer, test_group, name_prefix)
            }
            Self::DeployEVM(deploy) => deploy.run_evm_interpreter::<_, M>(
                summary,
                vm,
                mode,
                deployer,
                test_group,
                name_prefix,
            ),
            Self::Runtime(runtime) => {
                runtime.run_eravm::<M>(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::StorageEmpty(storage_empty) => {
                storage_empty.run_eravm(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::Balance(balance_check) => {
                balance_check.run_eravm(summary, vm, mode, test_group, name_prefix, index)
            }
        };
    }

    ///
    /// Runs the input on EVM.
    ///
    pub fn run_evm(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EVM,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) {
        match self {
            Self::DeployEraVM { .. } => panic!("EraVM deploy transaction cannot be run on EVM"),
            Self::DeployEVM(deploy) => deploy.run_evm(summary, vm, mode, test_group, name_prefix),
            Self::Runtime(runtime) => {
                runtime.run_evm(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::StorageEmpty(storage_empty) => {
                storage_empty.run_evm(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::Balance(balance_check) => {
                balance_check.run_evm(summary, vm, mode, test_group, name_prefix, index)
            }
        };
    }

    ///
    /// Runs the input on REVM.
    ///
    pub fn run_revm<'a, EXT, DB: Database>(
        self,
        summary: Arc<Mutex<Summary>>,
        mut vm: revm::Evm<'a, EXT, revm::db::State<DB>>,
        mode: Mode,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
        evm_builds: &HashMap<String, Build, RandomState>,
        evm_version: Option<EVMVersion>,
    ) -> revm::Evm<'a, EXT, State<DB>> {
        match self {
            Self::DeployEraVM { .. } => panic!("EraVM deploy transaction cannot be run on REVM"),
            Self::DeployEVM(deploy) => deploy.run_revm(
                summary,
                vm,
                mode,
                test_group,
                name_prefix,
                evm_builds,
                evm_version,
            ),
            Self::Runtime(runtime) => runtime.run_revm(
                summary,
                vm,
                mode,
                test_group,
                name_prefix,
                index,
                evm_version,
            ),
            Self::StorageEmpty(storage_empty) => {
                storage_empty.run_revm(summary, &mut vm, mode, test_group, name_prefix, index);
                vm
            }
            Self::Balance(balance_check) => {
                balance_check.run_revm(summary, &mut vm, mode, test_group, name_prefix, index);
                vm
            }
        }
    }

    ///
    /// Runs the input on EVM interpreter.
    ///
    pub fn run_evm_interpreter<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        vm: &mut EraVM,
        mode: Mode,
        deployer: &mut D,
        test_group: Option<String>,
        name_prefix: String,
        index: usize,
    ) where
        D: EraVMDeployer,
    {
        match self {
            Self::DeployEraVM { .. } => {
                panic!("EraVM deploy transaction cannot be run on EVM interpreter")
            }
            Self::DeployEVM(deploy) => deploy.run_evm_interpreter::<_, M>(
                summary,
                vm,
                mode,
                deployer,
                test_group,
                name_prefix,
            ),
            Self::Runtime(runtime) => {
                runtime.run_evm_interpreter::<M>(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::StorageEmpty(storage_empty) => {
                storage_empty.run_evm_interpreter(summary, vm, mode, test_group, name_prefix, index)
            }
            Self::Balance(balance_check) => {
                balance_check.run_evm_interpreter(summary, vm, mode, test_group, name_prefix, index)
            }
        };
    }
}
