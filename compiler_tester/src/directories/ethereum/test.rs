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
use crate::target::Target;
use crate::test::case::Case;
use crate::test::Test;
use crate::vm::address_iterator::AddressIterator;
use crate::vm::eravm::address_iterator::EraVMAddressIterator;
use crate::vm::evm::address_iterator::EVMAddressIterator;

///
/// The Ethereum compiler test.
///
#[derive(Debug)]
pub struct EthereumTest {
    /// The test identifier.
    pub identifier: String,
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
        let identifier = index_entity.path.to_string_lossy().to_string();

        if !filters.check_case_path(&identifier) {
            return None;
        }

        if !filters.check_group(&index_entity.group) {
            return None;
        }

        let test = match solidity_adapter::Test::try_from(index_entity.path.as_path()) {
            Ok(test) => test,
            Err(error) => {
                Summary::invalid(summary, None, identifier, error);
                return None;
            }
        };

        Some(Self {
            identifier,
            index_entity,
            test,
        })
    }

    ///
    /// Checks if the test is not filtered out.
    ///
    fn check_filters(&self, filters: &Filters, mode: &Mode) -> Option<()> {
        if !filters.check_mode(mode) {
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
        Some(())
    }

    ///
    /// Inserts necessary deploy transactions into the list of calls.
    ///
    fn insert_deploy_calls(&self, calls: &mut Vec<solidity_adapter::FunctionCall>) {
        if calls
            .iter()
            .any(|call| matches!(call, solidity_adapter::FunctionCall::Constructor { .. }))
        {
            return;
        }

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

    ///
    /// Returns all addresses.
    ///
    fn get_addresses(
        &self,
        mut address_iterator: impl AddressIterator,
        calls: &[solidity_adapter::FunctionCall],
        last_source: &str,
    ) -> anyhow::Result<(
        web3::types::Address,
        BTreeMap<String, web3::types::Address>,
        BTreeMap<String, BTreeMap<String, String>>,
    )> {
        let mut caller = solidity_adapter::account_address(solidity_adapter::DEFAULT_ACCOUNT_INDEX);

        let mut contract_address = None;
        let mut libraries_addresses = BTreeMap::new();
        let mut libraries = BTreeMap::new();
        for call in calls.iter() {
            match call {
                solidity_adapter::FunctionCall::Constructor { .. } => {
                    if contract_address.is_some() {
                        anyhow::bail!("Two constructors are not allowed for a single instance");
                    }
                    contract_address = Some(address_iterator.next(&caller, true));
                }
                solidity_adapter::FunctionCall::Library { name, source } => {
                    let source = source.clone().unwrap_or_else(|| last_source.to_owned());
                    let address = address_iterator.next(&caller, true);
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
                        anyhow::bail!("Expected address: `{}`, found `{}`", expected, address);
                    }
                    caller = address;
                }
                _ => {}
            }
        }
        let contract_address = contract_address.expect("Always valid");

        Ok((contract_address, libraries_addresses, libraries))
    }

    ///
    /// Returns the last source defined in the test.
    ///
    /// If the test has no sources, reports an `INVALID` and returns `None`.
    ///
    fn last_source(&self, summary: Arc<Mutex<Summary>>, mode: &Mode) -> Option<String> {
        match self.test.sources.last() {
            Some(last_source) => Some(last_source.0.to_owned()),
            None => {
                Summary::invalid(
                    summary,
                    Some(mode.to_owned()),
                    self.identifier.to_owned(),
                    anyhow::anyhow!("The Ethereum test `{}` sources are empty", self.identifier),
                );
                None
            }
        }
    }
}

impl Buildable for EthereumTest {
    fn build_for_eravm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        _target: Target,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<Test> {
        self.check_filters(filters, &mode)?;

        let mut calls = self.test.calls.clone();
        self.insert_deploy_calls(&mut calls);

        let last_source = self.last_source(summary.clone(), &mode)?;

        let (contract_address, libraries_addresses, libraries) = match self.get_addresses(
            EraVMAddressIterator::new(),
            calls.as_slice(),
            last_source.as_str(),
        ) {
            Ok((contract_address, libraries_addresses, libraries)) => {
                (contract_address, libraries_addresses, libraries)
            }
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let eravm_input = match compiler
            .compile_for_eravm(
                self.identifier.to_owned(),
                self.test.sources.clone(),
                libraries,
                &mode,
                vec![],
                debug_config,
            )
            .map_err(|error| anyhow::anyhow!("Failed to compile sources:\n{error}"))
        {
            Ok(output) => output,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let instances = match eravm_input.get_instances(
            &BTreeMap::new(),
            libraries_addresses,
            contract_address,
        ) {
            Ok(instance) => instance,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let case = match Case::try_from_ethereum(&calls, instances, &last_source) {
            Ok(case) => case,
            Err(error) => {
                Summary::invalid(
                    summary.clone(),
                    Some(mode),
                    self.identifier.to_owned(),
                    error,
                );
                return None;
            }
        };

        let builds = eravm_input
            .builds
            .into_values()
            .map(|build| {
                (
                    web3::types::U256::from_big_endian(build.bytecode_hash.as_slice()),
                    build.bytecode,
                )
            })
            .collect();

        Some(Test::new(
            self.identifier.to_owned(),
            self.index_entity.group.clone(),
            mode,
            builds,
            HashMap::new(),
            vec![case],
        ))
    }

    fn build_for_evm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        _target: Target,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<Test> {
        self.check_filters(filters, &mode)?;

        let mut calls = self.test.calls.clone();
        self.insert_deploy_calls(&mut calls);

        let last_source = self.last_source(summary.clone(), &mode)?;

        let (contract_address, libraries_addresses, libraries) = match self.get_addresses(
            EVMAddressIterator::new(false),
            calls.as_slice(),
            last_source.as_str(),
        ) {
            Ok((contract_address, libraries_addresses, libraries)) => {
                (contract_address, libraries_addresses, libraries)
            }
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let evm_input = match compiler
            .compile_for_evm(
                self.identifier.to_owned(),
                self.test.sources.clone(),
                libraries,
                &mode,
                vec![],
                debug_config,
            )
            .map_err(|error| anyhow::anyhow!("Failed to compile sources:\n{error}"))
        {
            Ok(output) => output,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let instances = match evm_input.get_instances(
            &BTreeMap::new(),
            libraries_addresses,
            Some(contract_address),
        ) {
            Ok(instance) => instance,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let case = match Case::try_from_ethereum(&calls, instances, &last_source) {
            Ok(case) => case,
            Err(error) => {
                Summary::invalid(
                    summary.clone(),
                    Some(mode),
                    self.identifier.to_owned(),
                    error,
                );
                return None;
            }
        };

        Some(Test::new(
            self.identifier.to_owned(),
            self.index_entity.group.clone(),
            mode,
            HashMap::new(),
            evm_input.builds,
            vec![case],
        ))
    }
}
