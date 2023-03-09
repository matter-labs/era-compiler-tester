//!
//! The Ethereum compiler test.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::deployers::address_predictor::AddressPredictor;
use crate::directories::Buildable;
use crate::filters::Filters;
use crate::summary::Summary;
use crate::test::case::Case;
use crate::test::instance::Instance;
use crate::test::Test;

///
/// The Ethereum compiler test.
///
pub struct EthereumTest<C>
where
    C: Compiler,
{
    /// The test name.
    test_name: String,
    /// The index test entity.
    index_entity: solidity_adapter::EnabledTest,
    /// The test compiler.
    compiler: C,
    /// The test function calls.
    calls: Vec<solidity_adapter::FunctionCall>,
    /// The test params.
    params: solidity_adapter::Params,
    /// The main contract address.
    contract_address: web3::types::Address,
    /// The libraries addresses.
    libraries_addresses: HashMap<String, web3::types::Address>,
    /// The last source name.
    last_source: String,
}

impl<C> EthereumTest<C>
where
    C: Compiler,
{
    ///
    /// Try to create new test.
    ///
    pub fn new(
        index_entity: solidity_adapter::EnabledTest,
        summary: Arc<Mutex<Summary>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        filters: &Filters,
    ) -> Option<Self> {
        let test_name = index_entity.path.to_string_lossy().to_string();

        if !filters.check_group(&index_entity.group) {
            return None;
        }

        if !filters.check_case_path(&test_name) {
            return None;
        }

        let test = match solidity_adapter::Test::try_from(index_entity.path.as_path()) {
            Ok(test) => test,
            Err(error) => {
                Summary::invalid(summary, None, test_name, error);
                return None;
            }
        };

        let solidity_adapter::Test {
            sources,
            mut calls,
            params,
        } = test;

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

        let last_source = match sources.last() {
            Some(last_source) => last_source.0.clone(),
            None => {
                Summary::invalid(
                    summary,
                    None,
                    test_name,
                    anyhow::anyhow!("Sources is empty"),
                );
                return None;
            }
        };

        let mut address_predictor = AddressPredictor::new();

        let mut contract_address = None;
        let mut caller = solidity_adapter::account_address(solidity_adapter::DEFAULT_ACCOUNT_INDEX);

        let mut libraries_for_compiler = BTreeMap::new();
        let mut libraries_addresses = HashMap::new();

        for call in calls.iter() {
            match call {
                solidity_adapter::FunctionCall::Constructor { .. } => {
                    if contract_address.is_some() {
                        Summary::invalid(
                            summary,
                            None,
                            test_name,
                            anyhow::anyhow!("Two constructors in test"),
                        );
                        return None;
                    }
                    contract_address = Some(address_predictor.next_address(caller));
                }
                solidity_adapter::FunctionCall::Library { name, source } => {
                    let source = source.clone().unwrap_or_else(|| last_source.clone());
                    let address = address_predictor.next_address(caller);
                    libraries_for_compiler
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
                            None,
                            test_name,
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

        Some(Self {
            test_name,
            index_entity,
            compiler: C::new(sources, libraries_for_compiler, debug_config, false),
            calls,
            params,
            contract_address,
            libraries_addresses,
            last_source,
        })
    }
}

impl<C> Buildable for EthereumTest<C>
where
    C: Compiler,
{
    fn build(&self, mode: Mode, summary: Arc<Mutex<Summary>>, filters: &Filters) -> Option<Test> {
        let test_name = self.index_entity.path.to_string_lossy().to_string();

        if !filters.check_mode(&mode) {
            return None;
        }

        if let Some(filters) = self.index_entity.modes.as_ref() {
            if !Filters::check_mode_filters(&mode, filters.as_slice()) {
                return None;
            }
        }

        if let Some(versions) = self.index_entity.version.as_ref() {
            if !mode.check_version(versions) {
                return None;
            }
        }

        if !C::check_ethereum_tests_params(&mode, &self.params) {
            return None;
        }

        let main_contract = match self
            .compiler
            .last_contract(&mode, false)
            .map_err(|error| anyhow::anyhow!("Failed to get main contract: {}", error))
        {
            Ok(main_contract) => main_contract,
            Err(error) => {
                Summary::invalid(summary, Some(mode.clone()), test_name, error);
                return None;
            }
        };

        let mut main_contract_instance = None;
        let mut libraries_instances = HashMap::with_capacity(self.libraries_addresses.len());

        let builds = match self
            .compiler
            .compile(&mode, false)
            .map_err(|error| anyhow::anyhow!("Failed to compile sources: {}", error))
            .and_then(|builds| {
                let main_contract_build = builds.get(main_contract.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("Main contract not found in the compiler build artifacts")
                })?;
                main_contract_instance = Some(Instance::new(
                    main_contract.clone(),
                    Some(self.contract_address),
                    main_contract_build.bytecode_hash,
                ));

                for (library_name, library_address) in self.libraries_addresses.iter() {
                    let build = builds.get(library_name).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Library {} not found in the compiler build artifacts",
                            library_name
                        )
                    })?;
                    libraries_instances.insert(
                        library_name.clone(),
                        Instance::new(
                            library_name.clone(),
                            Some(*library_address),
                            build.bytecode_hash,
                        ),
                    );
                }
                Ok(builds
                    .into_values()
                    .map(|build| (build.bytecode_hash, build.assembly))
                    .collect())
            }) {
            Ok(builds) => builds,
            Err(error) => {
                Summary::invalid(summary, Some(mode.clone()), test_name, error);
                return None;
            }
        };

        let case = match Case::from_ethereum::<C>(
            &self.calls,
            &main_contract_instance.expect("Always valid"),
            &libraries_instances,
            &self.last_source,
        ) {
            Ok(case) => case,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.test_name.clone(), error);
                return None;
            }
        };

        Some(Test::new(
            test_name,
            self.index_entity.group.clone(),
            mode,
            builds,
            vec![case],
        ))
    }
}
