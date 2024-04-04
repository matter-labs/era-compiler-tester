//!
//! The Ethereum compiler test.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::directories::Buildable;
use crate::filters::Filters;
use crate::summary::Summary;
use crate::test::case::Case;
use crate::test::eravm::Test as EraVMTest;
use crate::test::evm::Test as EVMTest;
use crate::test::instance::Instance;
use crate::vm::eravm::deployers::address_predictor::AddressPredictor as EraVMAddressPredictor;
use crate::vm::evm::address_predictor::AddressPredictor as EVMAddressPredictor;
use crate::vm::AddressPredictorIterator;

///
/// The Ethereum compiler test.
///
pub struct EthereumTest {
    /// The index test entity.
    pub index_entity: solidity_adapter::EnabledTest,
    /// The test data.
    pub test: solidity_adapter::Test,
}

impl EthereumTest {
    ///
    /// Try to create new test.
    ///
    pub fn new(
        index_entity: solidity_adapter::EnabledTest,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> Option<Self> {
        let test_path = index_entity.path.to_string_lossy().to_string();

        if !filters.check_case_path(&test_path) {
            return None;
        }

        if !filters.check_group(&index_entity.group) {
            return None;
        }

        let test = match solidity_adapter::Test::try_from(index_entity.path.as_path()) {
            Ok(test) => test,
            Err(error) => {
                Summary::invalid(summary, None, test_path, error);
                return None;
            }
        };

        Some(Self { index_entity, test })
    }
}

impl Buildable for EthereumTest {
    fn build_for_eravm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<EraVMTest> {
        let test_path = self.index_entity.path.to_string_lossy().to_string();

        if !filters.check_mode(&mode) {
            return None;
        }

        if let Some(filters) = self.index_entity.modes.as_ref() {
            if !mode.check_extended_filters(filters.as_slice()) {
                return None;
            }
        }

        if let Some(versions) = self.index_entity.version.as_ref() {
            if !mode.check_version(versions) {
                return None;
            }
        }

        if !mode.check_ethereum_tests_params(&self.test.params) {
            return None;
        }

        let mut calls = self.test.calls.clone();
        if !calls
            .iter()
            .any(|call| matches!(call, solidity_adapter::FunctionCall::Constructor { .. }))
        {
            let constructor = solidity_adapter::FunctionCall::Constructor {
                calldata: vec![],
                value: None,
                events: vec![],
                gas_options: vec![],
            };
            let constructor_insert_index = calls
                .iter()
                .position(|call| !matches!(call, solidity_adapter::FunctionCall::Library { .. }))
                .unwrap_or(calls.len());
            calls.insert(constructor_insert_index, constructor);
        }

        let last_source = match self.test.sources.last() {
            Some(last_source) => last_source.0.clone(),
            None => {
                Summary::invalid(
                    summary,
                    Some(mode),
                    test_path,
                    anyhow::anyhow!("Sources is empty"),
                );
                return None;
            }
        };

        let mut address_predictor = EraVMAddressPredictor::new();

        let mut contract_address = None;
        let mut caller = solidity_adapter::account_address(solidity_adapter::DEFAULT_ACCOUNT_INDEX);

        let mut libraries_addresses = HashMap::new();
        let mut libraries = BTreeMap::new();

        for call in calls.iter() {
            match call {
                solidity_adapter::FunctionCall::Constructor { .. } => {
                    if contract_address.is_some() {
                        Summary::invalid(
                            summary,
                            Some(mode),
                            test_path,
                            anyhow::anyhow!("Two constructors in test"),
                        );
                        return None;
                    }
                    contract_address = Some(address_predictor.next(&caller, true));
                }
                solidity_adapter::FunctionCall::Library { name, source } => {
                    let source = source.clone().unwrap_or_else(|| last_source.clone());
                    let address = address_predictor.next(&caller, true);
                    libraries
                        .entry(source.clone())
                        .or_insert_with(BTreeMap::new)
                        .insert(
                            name.clone(),
                            format!("0x{}", crate::utils::address_as_string(&address)),
                        );
                    libraries_addresses.insert(format!("{source}:{name}"), address);
                }
                solidity_adapter::FunctionCall::Account { input, expected } => {
                    let address = solidity_adapter::account_address(*input);
                    if !expected.eq(&address) {
                        Summary::invalid(
                            summary,
                            Some(mode),
                            test_path,
                            anyhow::anyhow!(
                                "Expected address: {}, but found {}",
                                expected,
                                address
                            ),
                        );
                        return None;
                    }
                    caller = address;
                }
                _ => {}
            }
        }

        let contract_address = contract_address.expect("Always valid");

        let compiler_output = match compiler
            .compile_for_eravm(
                test_path.clone(),
                self.test.sources.clone(),
                libraries,
                &mode,
                false,
                false,
                debug_config,
            )
            .map_err(|error| anyhow::anyhow!("Failed to compile sources: {}", error))
        {
            Ok(output) => output,
            Err(error) => {
                Summary::invalid(summary, Some(mode), test_path, error);
                return None;
            }
        };

        let main_contract = compiler_output.last_contract;

        let main_contract_build = match compiler_output
            .builds
            .get(main_contract.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Main contract not found in the compiler build artifacts")
            }) {
            Ok(build) => build,
            Err(error) => {
                Summary::invalid(summary, Some(mode), test_path, error);
                return None;
            }
        };
        let main_contract_instance = Instance::new(
            main_contract,
            Some(contract_address),
            main_contract_build.bytecode_hash,
        );

        let mut libraries_instances = HashMap::with_capacity(libraries_addresses.len());

        for (library_name, library_address) in libraries_addresses {
            let build = match compiler_output.builds.get(&library_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Library {} not found in the compiler build artifacts",
                    library_name
                )
            }) {
                Ok(build) => build,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };
            libraries_instances.insert(
                library_name.clone(),
                Instance::new(library_name, Some(library_address), build.bytecode_hash),
            );
        }

        let case = match Case::try_from_ethereum(
            &calls,
            &main_contract_instance,
            &libraries_instances,
            &last_source,
        ) {
            Ok(case) => case,
            Err(error) => {
                Summary::invalid(summary, Some(mode), test_path, error);
                return None;
            }
        };

        let builds = compiler_output
            .builds
            .into_values()
            .map(|build| (build.bytecode_hash, build.assembly))
            .collect();

        Some(EraVMTest::new(
            test_path,
            self.index_entity.group.clone(),
            mode,
            builds,
            vec![case],
        ))
    }

    fn build_for_evm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<EVMTest> {
        let test_path = self.index_entity.path.to_string_lossy().to_string();

        if !filters.check_mode(&mode) {
            return None;
        }

        if let Some(filters) = self.index_entity.modes.as_ref() {
            if !mode.check_extended_filters(filters.as_slice()) {
                return None;
            }
        }

        if let Some(versions) = self.index_entity.version.as_ref() {
            if !mode.check_version(versions) {
                return None;
            }
        }

        if !mode.check_ethereum_tests_params(&self.test.params) {
            return None;
        }

        let mut calls = self.test.calls.clone();
        if !calls
            .iter()
            .any(|call| matches!(call, solidity_adapter::FunctionCall::Constructor { .. }))
        {
            let constructor = solidity_adapter::FunctionCall::Constructor {
                calldata: vec![],
                value: None,
                events: vec![],
                gas_options: vec![],
            };
            let constructor_insert_index = calls
                .iter()
                .position(|call| !matches!(call, solidity_adapter::FunctionCall::Library { .. }))
                .unwrap_or(calls.len());
            calls.insert(constructor_insert_index, constructor);
        }

        let last_source = match self.test.sources.last() {
            Some(last_source) => last_source.0.clone(),
            None => {
                Summary::invalid(
                    summary,
                    Some(mode),
                    test_path,
                    anyhow::anyhow!("Sources is empty"),
                );
                return None;
            }
        };

        let mut address_predictor = EVMAddressPredictor::new();

        let mut contract_address = None;
        let mut caller = solidity_adapter::account_address(solidity_adapter::DEFAULT_ACCOUNT_INDEX);

        let mut libraries_addresses = HashMap::new();
        let mut libraries = BTreeMap::new();

        for call in calls.iter() {
            match call {
                solidity_adapter::FunctionCall::Constructor { .. } => {
                    if contract_address.is_some() {
                        Summary::invalid(
                            summary,
                            Some(mode),
                            test_path,
                            anyhow::anyhow!("Two constructors in test"),
                        );
                        return None;
                    }
                    contract_address = Some(address_predictor.next(&caller, true));
                }
                solidity_adapter::FunctionCall::Library { name, source } => {
                    let source = source.clone().unwrap_or_else(|| last_source.clone());
                    let address = address_predictor.next(&caller, true);
                    libraries
                        .entry(source.clone())
                        .or_insert_with(BTreeMap::new)
                        .insert(
                            name.clone(),
                            format!("0x{}", crate::utils::address_as_string(&address)),
                        );
                    libraries_addresses.insert(format!("{source}:{name}"), address);
                }
                solidity_adapter::FunctionCall::Account { input, expected } => {
                    let address = solidity_adapter::account_address(*input);
                    if !expected.eq(&address) {
                        Summary::invalid(
                            summary,
                            Some(mode),
                            test_path,
                            anyhow::anyhow!(
                                "Expected address: {}, but found {}",
                                expected,
                                address
                            ),
                        );
                        return None;
                    }
                    caller = address;
                }
                _ => {}
            }
        }

        let contract_address = contract_address.expect("Always valid");

        let compiler_output = match compiler
            .compile_for_evm(
                test_path.clone(),
                self.test.sources.clone(),
                libraries,
                &mode,
                debug_config,
            )
            .map_err(|error| anyhow::anyhow!("Failed to compile sources: {}", error))
        {
            Ok(output) => output,
            Err(error) => {
                Summary::invalid(summary, Some(mode), test_path, error);
                return None;
            }
        };

        let main_contract = compiler_output.last_contract;

        let main_contract_build = match compiler_output
            .builds
            .get(main_contract.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Main contract not found in the compiler build artifacts")
            }) {
            Ok(build) => build,
            Err(error) => {
                Summary::invalid(summary, Some(mode), test_path, error);
                return None;
            }
        };
        let main_contract_instance = Instance::new(
            main_contract,
            Some(contract_address),
            web3::types::U256::zero(),
        );

        let mut libraries_instances = HashMap::with_capacity(libraries_addresses.len());

        for (library_name, library_address) in libraries_addresses {
            let build = match compiler_output.builds.get(&library_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Library {} not found in the compiler build artifacts",
                    library_name
                )
            }) {
                Ok(build) => build,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };
            libraries_instances.insert(
                library_name.clone(),
                Instance::new(
                    library_name,
                    Some(library_address),
                    web3::types::U256::zero(),
                ),
            );
        }

        let case = match Case::try_from_ethereum(
            &calls,
            &main_contract_instance,
            &libraries_instances,
            &last_source,
        ) {
            Ok(case) => case,
            Err(error) => {
                Summary::invalid(summary, Some(mode), test_path, error);
                return None;
            }
        };

        let builds = compiler_output
            .builds
            .into_values()
            .map(|build| (web3::types::Address::zero(), build))
            .collect();

        Some(EVMTest::new(
            test_path,
            self.index_entity.group.clone(),
            mode,
            builds,
            vec![case],
        ))
    }
}
