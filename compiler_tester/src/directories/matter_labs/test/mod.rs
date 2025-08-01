//!
//! The Matter Labs compiler test.
//!

pub mod metadata;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::directories::Buildable;
use crate::environment::Environment;
use crate::filters::Filters;
use crate::summary::Summary;
use crate::test::case::Case;
use crate::test::description::TestDescription;
use crate::test::instance::Instance;
use crate::test::selector::TestSelector;
use crate::test::Test;
use crate::vm::address_iterator::AddressIterator;
use crate::vm::eravm::address_iterator::EraVMAddressIterator;
use crate::vm::revm::address_iterator::EVMAddressIterator;

use self::metadata::case::input::calldata::Calldata as MatterLabsCaseInputCalldata;
use self::metadata::case::input::expected::Expected as MatterLabsCaseInputExpected;
use self::metadata::case::input::Input as MatterLabsCaseInput;
use self::metadata::case::Case as MatterLabsCase;
use self::metadata::Metadata;

/// The default simple contract name.
pub const SIMPLE_TESTS_CONTRACT_NAME: &str = "Test";

/// The default simple contract instance name.
pub const SIMPLE_TESTS_INSTANCE: &str = "Test";

/// The default address of the caller.
pub const DEFAULT_CALLER_ADDRESS: &str = "deadbeef01000000000000000000000000000000";

///
/// Used for default initialization.
///
pub fn simple_tests_instance() -> String {
    SIMPLE_TESTS_INSTANCE.to_string()
}

///
/// Used for default initialization.
///
pub fn default_caller_address() -> String {
    DEFAULT_CALLER_ADDRESS.to_string()
}

///
/// The Matter Labs compiler test.
///
#[derive(Debug)]
pub struct MatterLabsTest {
    /// The test path.
    path: PathBuf,
    /// The test selector.
    selector: TestSelector,
    /// The test metadata.
    metadata: Metadata,
    /// The test sources.
    sources: Vec<(String, String)>,
}

impl MatterLabsTest {
    ///
    /// Try to create new test.
    ///
    pub fn new(path: PathBuf, summary: Arc<Mutex<Summary>>, filters: &Filters) -> Option<Self> {
        let selector = TestSelector {
            path: crate::utils::path_to_string_normalized(path.as_path()),
            case: None,
            input: None,
        };

        if !filters.check_test_path(selector.path.to_string().as_str()) {
            return None;
        }

        let test_description = TestDescription::default_for(selector.clone());

        let main_file_string = match std::fs::read_to_string(path.as_path()) {
            Ok(data) => data,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };

        let mut metadata = match Metadata::from_str(main_file_string.as_str())
            .map_err(|error| anyhow::anyhow!("Invalid metadata JSON: {}", error))
        {
            Ok(metadata) => metadata,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };

        if metadata.ignore {
            Summary::ignored(summary, test_description);
            return None;
        }

        if !filters.check_group(&metadata.group) {
            return None;
        }

        let sources = if metadata.contracts.is_empty() {
            if path.ends_with("test.json") {
                vec![]
            } else {
                vec![(
                    crate::utils::path_to_string_normalized(path.as_path()),
                    main_file_string,
                )]
            }
        } else {
            let mut sources = HashMap::new();
            let mut paths = HashSet::with_capacity(metadata.contracts.len());
            for (_, path_string) in metadata.contracts.iter_mut() {
                let mut file_path = path.clone();
                file_path.pop();
                let mut path_string_split = path_string.split(':');
                let file_relative_path = path_string_split.next().expect("Always exists");
                let contract_name = path_string_split.next();
                file_path.push(file_relative_path);

                let file_path_unified =
                    crate::utils::path_to_string_normalized(file_path.as_path());
                *path_string = if let Some(contract_name) = contract_name {
                    format!("{file_path_unified}:{contract_name}")
                } else {
                    file_path_unified.clone()
                };
                paths.insert(file_path_unified);
            }

            let mut test_directory_path = path.clone();
            test_directory_path.pop();
            for entry in
                glob::glob(format!("{}/**/*.sol", test_directory_path.to_string_lossy()).as_str())
                    .expect("Always valid")
                    .filter_map(Result::ok)
            {
                paths.insert(crate::utils::path_to_string_normalized(entry.as_path()));
            }

            for path in paths.into_iter() {
                let source_code = match std::fs::read_to_string(path.as_str())
                    .map_err(|err| anyhow::anyhow!("Reading source file error: {}", err))
                {
                    Ok(source) => source,
                    Err(error) => {
                        Summary::invalid(summary, test_description, error);
                        return None;
                    }
                };
                sources.insert(path, source_code);
            }
            sources.into_iter().collect()
        };

        metadata.cases.retain(|case| {
            let selector_with_case = TestSelector {
                path: selector.path.clone(),
                case: Some(case.name.clone()),
                input: selector.input.clone(),
            };
            if case.ignore {
                Summary::ignored(
                    summary.clone(),
                    TestDescription::default_for(selector_with_case),
                );
                return false;
            }
            let case_name = selector_with_case.to_string();
            if !filters.check_case_path(&case_name) {
                return false;
            }
            true
        });

        Some(Self {
            path,
            selector,
            metadata,
            sources,
        })
    }

    ///
    /// Checks if the test is not filtered out.
    ///
    fn check_filters(
        &self,
        filters: &Filters,
        mode: &Mode,
        target: era_compiler_common::Target,
    ) -> Option<()> {
        if let Some(targets) = self.metadata.targets.as_ref() {
            if !targets.contains(&target) {
                return None;
            }
        }
        if !filters.check_mode(mode) {
            return None;
        }
        if let Some(filters) = self.metadata.modes.as_ref() {
            if !mode.check_extended_filters(filters.as_slice()) {
                return None;
            }
        }
        if !mode.check_pragmas(&self.sources) {
            return None;
        }
        Some(())
    }

    ///
    /// Adds the default contract to the list of contracts if it is empty.
    ///
    fn push_default_contract(
        &self,
        contracts: &mut BTreeMap<String, String>,
        is_multi_contract: bool,
    ) {
        if contracts.is_empty() {
            let contract_name = if is_multi_contract {
                format!("{}:{}", self.selector.path, SIMPLE_TESTS_CONTRACT_NAME)
            } else {
                self.selector.path.to_string()
            };
            contracts.insert(SIMPLE_TESTS_INSTANCE.to_owned(), contract_name);
        }
    }

    ///
    /// Returns library information.
    ///
    fn get_libraries<API>(
        &self,
        address_iterator: &mut API,
    ) -> (
        era_compiler_common::Libraries,
        BTreeMap<String, web3::types::Address>,
    )
    where
        API: AddressIterator,
    {
        let mut libraries = BTreeMap::new();
        let mut library_addresses = BTreeMap::new();

        for (file, metadata_file_libraries) in self.metadata.libraries.iter() {
            let mut file_path = self.path.clone();
            file_path.pop();
            file_path.push(file);

            let file_path_string = crate::utils::path_to_string_normalized(file_path.as_path());

            let mut file_libraries = BTreeMap::new();
            for name in metadata_file_libraries.keys() {
                let address = address_iterator.next(
                    &web3::types::Address::from_str(DEFAULT_CALLER_ADDRESS).expect("Always valid"),
                    true,
                );
                file_libraries.insert(
                    name.to_owned(),
                    format!("0x{}", crate::utils::address_as_string(&address)),
                );
                library_addresses.insert(format!("{file_path_string}:{name}"), address);
            }
            libraries.insert(file_path_string, file_libraries);
        }

        (libraries.into(), library_addresses)
    }

    ///
    /// Returns precompiled EVM contract instances.
    ///
    fn get_evm_instances(&self) -> anyhow::Result<BTreeMap<String, Instance>> {
        let mut instances = BTreeMap::new();

        for (instance, evm_contract) in self.metadata.evm_contracts.iter() {
            let instruction_name = instance.split('_').next().expect("Always exists");
            let runtime_code = evm_contract.runtime_code(instruction_name);
            let mut bytecode = evm_contract.deploy_code(runtime_code.len());
            bytecode.push_str(runtime_code.as_str());

            let bytecode = hex::decode(bytecode.as_str()).map_err(|error| {
                anyhow::anyhow!("Invalid bytecode of EVM instance `{}`: {}", instance, error)
            })?;
            instances.insert(
                instance.to_owned(),
                Instance::evm(instance.to_owned(), None, false, false, bytecode.to_owned()),
            );
        }

        Ok(instances)
    }

    fn is_evm_interpreter_test(&self) -> bool {
        matches!(
            self.metadata.group.as_deref(),
            Some(benchmark_analyzer::TEST_GROUP_EVM_INTERPRETER)
        )
    }
    ///
    /// Returns cases needed for running benchmarks on the EVM interpreter.
    ///
    fn evm_interpreter_benchmark_cases(&self) -> Option<Vec<MatterLabsCase>> {
        if !self.is_evm_interpreter_test() {
            return None;
        }

        let mut evm_contracts: Vec<String> = self
            .metadata
            .evm_contracts
            .keys()
            .filter(|name| {
                name.contains("Template") || name.contains("Full") || name.contains("Before")
            })
            .cloned()
            .collect();
        evm_contracts.sort();

        let mut metadata_cases = Vec::with_capacity(evm_contracts.len() / 3);
        for pair_of_bytecodes in evm_contracts.chunks(3) {
            let before = &pair_of_bytecodes[0];
            let full = &pair_of_bytecodes[1];
            let template = &pair_of_bytecodes[2];
            let exception = full.contains("REVERT");

            metadata_cases.push(MatterLabsCase {
                comment: None,
                name: template
                    .strip_suffix("_Template")
                    .expect("Always exists")
                    .to_owned(),
                modes: None,
                inputs: vec![
                    MatterLabsCaseInput {
                        comment: None,
                        instance: before.to_owned(),
                        caller: default_caller_address(),
                        method: "#fallback".to_owned(),
                        calldata: MatterLabsCaseInputCalldata::List(vec![]),
                        value: None,
                        storage: HashMap::new(),
                        expected: Some(
                            MatterLabsCaseInputExpected::successful_evm_interpreter_benchmark(
                                false,
                            ),
                        ),
                        expected_eravm: None,
                        expected_evm: None,
                    },
                    MatterLabsCaseInput {
                        comment: None,
                        instance: template.to_owned(),
                        caller: default_caller_address(),
                        method: "#fallback".to_owned(),
                        calldata: MatterLabsCaseInputCalldata::List(vec![]),
                        value: None,
                        storage: HashMap::new(),
                        expected: Some(
                            MatterLabsCaseInputExpected::successful_evm_interpreter_benchmark(
                                false,
                            ),
                        ),
                        expected_eravm: None,
                        expected_evm: None,
                    },
                    MatterLabsCaseInput {
                        comment: None,
                        instance: full.to_owned(),
                        caller: default_caller_address(),
                        method: "#fallback".to_owned(),
                        calldata: MatterLabsCaseInputCalldata::List(vec![]),
                        value: None,
                        storage: HashMap::new(),
                        expected: Some(
                            MatterLabsCaseInputExpected::successful_evm_interpreter_benchmark(
                                exception,
                            ),
                        ),
                        expected_eravm: None,
                        expected_evm: None,
                    },
                ],
                ignore: false,
                cycles: None,
                expected: Some(
                    MatterLabsCaseInputExpected::successful_evm_interpreter_benchmark(exception),
                ),
                expected_eravm: None,
                expected_evm: None,
            })
        }
        Some(metadata_cases)
    }
}

impl Buildable for MatterLabsTest {
    fn build_for_eravm(
        &self,
        mut mode: Mode,
        compiler: Arc<dyn Compiler>,
        environment: Environment,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<Test> {
        mode.enable_eravm_extensions(self.metadata.enable_eravm_extensions);
        self.check_filters(filters, &mode, era_compiler_common::Target::EraVM)?;

        let mut contracts = self.metadata.contracts.clone();
        self.push_default_contract(&mut contracts, compiler.allows_multi_contract_files());

        let mut eravm_address_iterator = EraVMAddressIterator::new();
        let evm_address_iterator = EVMAddressIterator::default();

        let test_description = TestDescription {
            group: None,
            mode: Some(mode.clone()),
            selector: self.selector.clone(),
        };

        let (libraries, library_addresses) = self.get_libraries(&mut eravm_address_iterator);
        let eravm_input = match compiler
            .compile_for_eravm(
                self.selector.path.to_string(),
                self.sources.clone(),
                libraries,
                &mode,
                vec![],
                debug_config,
            )
            .map_err(|error| anyhow::anyhow!("Failed to compile sources:\n{error}"))
        {
            Ok(vm_input) => vm_input,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };

        let mut instances = match eravm_input.get_instances(
            &contracts,
            library_addresses,
            web3::types::Address::zero(),
        ) {
            Ok(instances) => instances,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };

        let evm_instances = match self.get_evm_instances() {
            Ok(evm_instances) => evm_instances,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };
        instances.extend(evm_instances);

        let metadata_cases = {
            let mut base_cases = self.metadata.cases.to_owned();
            if let Some(opcode_test_cases) = self.evm_interpreter_benchmark_cases() {
                base_cases.extend(opcode_test_cases);
            }
            base_cases
        };

        let mut cases = Vec::with_capacity(metadata_cases.len());
        for case in metadata_cases.into_iter() {
            if let Some(filters) = case.modes.as_ref() {
                if !mode.check_extended_filters(filters.as_slice()) {
                    continue;
                }
            }

            let case = match case.normalize(&contracts, &instances, environment) {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, test_description, error);
                    return None;
                }
            };

            match case.set_variables(
                &mut instances,
                eravm_address_iterator.clone(),
                evm_address_iterator.clone(),
                &mode,
            ) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, test_description, error);
                    return None;
                }
            }

            let case_name = case.name.to_owned();
            let case = match Case::try_from_matter_labs(
                case,
                &mode,
                &instances,
                &eravm_input.method_identifiers,
                era_compiler_common::Target::EraVM,
            )
            .map_err(|error| anyhow::anyhow!("Case `{case_name}` is invalid: {error}"))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, test_description, error);
                    return None;
                }
            };

            cases.push(case);
        }

        let builds = eravm_input
            .builds
            .into_values()
            .map(|build| {
                (
                    web3::types::U256::from_big_endian(
                        build.bytecode_hash.expect("Always exists").as_slice(),
                    ),
                    build.bytecode,
                )
            })
            .collect();

        Some(Test::new(
            self.selector.to_string(),
            cases,
            mode,
            self.metadata.group.clone(),
            builds,
            None,
        ))
    }

    fn build_for_evm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        environment: Environment,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<Test> {
        self.check_filters(filters, &mode, era_compiler_common::Target::EVM)?;

        let mut contracts = self.metadata.contracts.clone();
        self.push_default_contract(&mut contracts, compiler.allows_multi_contract_files());

        let mut evm_address_iterator = EVMAddressIterator::default();

        let sources = self.sources.to_owned();
        let (libraries, library_addresses) = self.get_libraries(&mut evm_address_iterator);

        let test_description = TestDescription {
            group: None,
            mode: Some(mode.clone()),
            selector: self.selector.clone(),
        };

        let evm_input = match compiler
            .compile_for_evm(
                self.selector.path.to_string(),
                sources,
                libraries,
                &mode,
                None,
                vec![],
                debug_config,
            )
            .map_err(|error| anyhow::anyhow!("Failed to compile sources:\n{error}"))
        {
            Ok(output) => output,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };

        let mut instances = match evm_input.get_instances(&contracts, library_addresses, None) {
            Ok(instances) => instances,
            Err(error) => {
                Summary::invalid(summary, test_description, error);
                return None;
            }
        };

        let mut cases = Vec::with_capacity(self.metadata.cases.len());
        for case in self.metadata.cases.iter() {
            if let Some(filters) = case.modes.as_ref() {
                if !mode.check_extended_filters(filters.as_slice()) {
                    continue;
                }
            }

            let case = match case
                .to_owned()
                .normalize(&contracts, &instances, environment)
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, test_description, error);
                    return None;
                }
            };

            match case.set_variables(
                &mut instances,
                EraVMAddressIterator::new(),
                evm_address_iterator.clone(),
                &mode,
            ) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, test_description, error);
                    return None;
                }
            }

            let case_name = case.name.to_owned();
            let case = match Case::try_from_matter_labs(
                case,
                &mode,
                &instances,
                &evm_input.method_identifiers,
                era_compiler_common::Target::EVM,
            )
            .map_err(|error| anyhow::anyhow!("Case `{}` is invalid: {}", case_name, error))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, test_description, error);
                    return None;
                }
            };

            cases.push(case);
        }

        Some(Test::new(
            self.selector.to_string(),
            cases,
            mode,
            self.metadata.group.clone(),
            HashMap::new(),
            None,
        ))
    }
}
