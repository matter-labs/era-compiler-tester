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
use crate::filters::Filters;
use crate::summary::Summary;
use crate::target::Target;
use crate::test::case::Case;
use crate::test::instance::Instance;
use crate::test::Test;
use crate::vm::address_iterator::AddressIterator;
use crate::vm::eravm::address_iterator::EraVMAddressIterator;
use crate::vm::evm::address_iterator::EVMAddressIterator;

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
    /// The test identifier.
    identifier: String,
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
        let identifier = path.to_string_lossy().to_string();

        if !filters.check_test_path(identifier.as_str()) {
            return None;
        }

        let main_file_string = match std::fs::read_to_string(path.as_path()) {
            Ok(data) => data,
            Err(error) => {
                Summary::invalid(summary, None, identifier.clone(), error);
                return None;
            }
        };

        let mut metadata = match Metadata::from_str(main_file_string.as_str())
            .map_err(|error| anyhow::anyhow!("Invalid metadata JSON: {}", error))
        {
            Ok(metadata) => metadata,
            Err(error) => {
                Summary::invalid(summary, None, identifier.clone(), error);
                return None;
            }
        };

        if metadata.ignore {
            Summary::ignored(summary, identifier.clone());
            return None;
        }

        if !filters.check_group(&metadata.group) {
            return None;
        }

        let sources = if metadata.contracts.is_empty() {
            if path.ends_with("test.json") {
                vec![]
            } else {
                vec![(path.to_string_lossy().to_string(), main_file_string)]
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
                *path_string = if let Some(contract_name) = contract_name {
                    format!("{}:{}", file_path.to_string_lossy(), contract_name)
                } else {
                    file_path.to_string_lossy().to_string()
                };
                paths.insert(file_path.to_string_lossy().to_string());
            }

            let mut test_directory_path = path.clone();
            test_directory_path.pop();
            for entry in
                glob::glob(format!("{}/**/*.sol", test_directory_path.to_string_lossy()).as_str())
                    .expect("Always valid")
                    .filter_map(Result::ok)
            {
                paths.insert(entry.to_string_lossy().to_string());
            }

            for path in paths.into_iter() {
                let source_code = match std::fs::read_to_string(path.as_str())
                    .map_err(|err| anyhow::anyhow!("Reading source file error: {}", err))
                {
                    Ok(source) => source,
                    Err(error) => {
                        Summary::invalid(summary, None, identifier.clone(), error);
                        return None;
                    }
                };
                sources.insert(path, source_code);
            }
            sources.into_iter().collect()
        };

        metadata.cases.retain(|case| {
            let case_name = format!("{}::{}", identifier, case.name);
            if case.ignore {
                Summary::ignored(summary.clone(), case_name);
                return false;
            }

            if !filters.check_case_path(&case_name) {
                return false;
            }
            true
        });

        Some(Self {
            path,
            identifier,
            metadata,
            sources,
        })
    }

    ///
    /// Checks if the test is not filtered out.
    ///
    fn check_filters(&self, filters: &Filters, mode: &Mode) -> Option<()> {
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
                format!("{}:{}", self.identifier, SIMPLE_TESTS_CONTRACT_NAME)
            } else {
                self.identifier.to_owned()
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
        BTreeMap<String, BTreeMap<String, String>>,
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
                library_addresses.insert(
                    format!("{}:{}", file_path.to_string_lossy().as_ref(), name),
                    address,
                );
            }
            libraries.insert(file_path.to_string_lossy().to_string(), file_libraries);
        }

        (libraries, library_addresses)
    }

    ///
    /// Returns precompiled EVM contract instances.
    ///
    fn get_evm_instances(&self) -> anyhow::Result<BTreeMap<String, Instance>> {
        let mut instances = BTreeMap::new();

        for (instance, evm_contract) in self.metadata.evm_contracts.iter() {
            let instruction_name = instance.split('_').next().expect("Always exists");
            let runtime_code = evm_contract.runtime_code(instruction_name);
            let mut bytecode = evm_contract.init_code(runtime_code.len());
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

    ///
    /// Returns cases needed for running benchmarks on the EVM interpreter.
    ///
    fn evm_interpreter_benchmark_cases(&self) -> Vec<MatterLabsCase> {
        if self.metadata.group.as_deref()
            != Some(benchmark_analyzer::Benchmark::EVM_INTERPRETER_GROUP_NAME)
        {
            return vec![];
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
                    },
                ],
                expected: MatterLabsCaseInputExpected::successful_evm_interpreter_benchmark(
                    exception,
                ),
                ignore: false,
                cycles: None,
            })
        }
        metadata_cases
    }
}

impl Buildable for MatterLabsTest {
    fn build_for_eravm(
        &self,
        mut mode: Mode,
        compiler: Arc<dyn Compiler>,
        target: Target,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<Test> {
        mode.enable_eravm_extensions(self.metadata.enable_eravm_extensions);

        self.check_filters(filters, &mode)?;

        let mut contracts = self.metadata.contracts.clone();
        self.push_default_contract(&mut contracts, compiler.allows_multi_contract_files());

        let mut eravm_address_iterator = EraVMAddressIterator::new();
        let evm_address_iterator =
            EVMAddressIterator::new(matches!(target, Target::EVMInterpreter));

        let (libraries, library_addresses) = self.get_libraries(&mut eravm_address_iterator);

        let eravm_input = match compiler
            .compile_for_eravm(
                self.identifier.to_owned(),
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
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
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
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };

        let evm_instances = match self.get_evm_instances() {
            Ok(evm_instances) => evm_instances,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                return None;
            }
        };
        instances.extend(evm_instances);

        let mut metadata_cases = self.metadata.cases.to_owned();
        metadata_cases.extend(self.evm_interpreter_benchmark_cases());

        let mut cases = Vec::with_capacity(metadata_cases.len());
        for case in metadata_cases.into_iter() {
            if let Some(filters) = case.modes.as_ref() {
                if !mode.check_extended_filters(filters.as_slice()) {
                    continue;
                }
            }

            let case = match case.normalize(&contracts, &instances, target) {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                    return None;
                }
            };

            match case.set_instance_addresses(
                &mut instances,
                eravm_address_iterator.clone(),
                evm_address_iterator.clone(),
                &mode,
            ) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                    return None;
                }
            }

            let case_name = case.name.to_owned();
            let case = match Case::try_from_matter_labs(
                case,
                &mode,
                &instances,
                &eravm_input.method_identifiers,
            )
            .map_err(|error| anyhow::anyhow!("Case `{}` is invalid: {}", case_name, error))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
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
                    web3::types::U256::from_big_endian(build.bytecode_hash.as_slice()),
                    build.bytecode,
                )
            })
            .collect();

        Some(Test::new(
            self.identifier.to_owned(),
            self.metadata.group.clone(),
            mode,
            builds,
            HashMap::new(),
            cases,
        ))
    }

    fn build_for_evm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        target: Target,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<Test> {
        self.check_filters(filters, &mode)?;

        let mut contracts = self.metadata.contracts.clone();
        self.push_default_contract(&mut contracts, compiler.allows_multi_contract_files());
        let sources = self.sources.to_owned();

        let mut evm_address_iterator =
            EVMAddressIterator::new(matches!(target, Target::EVMInterpreter));

        let (libraries, library_addresses) = self.get_libraries(&mut evm_address_iterator);

        let evm_input = match compiler
            .compile_for_evm(
                self.identifier.to_owned(),
                sources,
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

        let mut instances = match evm_input.get_instances(&contracts, library_addresses, None) {
            Ok(instances) => instances,
            Err(error) => {
                Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
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

            let case = match case.to_owned().normalize(&contracts, &instances, target) {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                    return None;
                }
            };

            match case.set_instance_addresses(
                &mut instances,
                EraVMAddressIterator::new(),
                evm_address_iterator.clone(),
                &mode,
            ) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                    return None;
                }
            }

            let case_name = case.name.to_owned();
            let case = match Case::try_from_matter_labs(
                case,
                &mode,
                &instances,
                &evm_input.method_identifiers,
            )
            .map_err(|error| anyhow::anyhow!("Case `{}` is invalid: {}", case_name, error))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), self.identifier.to_owned(), error);
                    return None;
                }
            };

            cases.push(case);
        }

        Some(Test::new(
            self.identifier.to_owned(),
            self.metadata.group.clone(),
            mode,
            HashMap::new(),
            evm_input.builds,
            cases,
        ))
    }
}
