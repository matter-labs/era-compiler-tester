//!
//! The Matter Labs compiler test.
//!

pub mod metadata;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
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
use crate::test::case::Case;
use crate::test::eravm::Test as EraVMTest;
use crate::test::evm::Test as EVMTest;
use crate::test::instance::Instance;
use crate::vm::eravm::deployers::address_predictor::AddressPredictor as EraVMAddressPredictor;
use crate::vm::evm::address_predictor::AddressPredictor as EVMAddressPredictor;
use crate::vm::AddressPredictorIterator;

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
pub struct MatterLabsTest {
    /// The test path.
    path: PathBuf,
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
        let test_path = path.to_string_lossy().to_string();

        if !filters.check_test_path(&test_path) {
            return None;
        }

        let metadata_file_string = match std::fs::read_to_string(path.as_path()) {
            Ok(metadata_file_string) => metadata_file_string,
            Err(error) => {
                Summary::invalid(summary, None, test_path, error);
                return None;
            }
        };

        let mut metadata = match Metadata::from_str(metadata_file_string.as_str())
            .map_err(|err| anyhow::anyhow!("Invalid metadata json: {}", err))
        {
            Ok(metadata) => metadata,
            Err(error) => {
                Summary::invalid(summary, None, test_path, error);
                return None;
            }
        };

        if metadata.ignore {
            Summary::ignored(summary, test_path);
            return None;
        }

        if !filters.check_group(&metadata.group) {
            return None;
        }

        let sources = if metadata.contracts.is_empty() {
            vec![(path.to_string_lossy().to_string(), metadata_file_string)]
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
                    .map_err(|err| anyhow::anyhow!("Failed to read source code file: {}", err))
                {
                    Ok(source) => source,
                    Err(error) => {
                        Summary::invalid(summary, None, test_path, error);
                        return None;
                    }
                };
                sources.insert(path, source_code);
            }
            sources.into_iter().collect()
        };

        metadata.cases.retain(|case| {
            let case_name = format!("{}::{}", test_path, case.name);
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
            metadata,
            sources,
        })
    }
}

impl Buildable for MatterLabsTest {
    fn build_for_eravm(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> Option<EraVMTest> {
        let test_path = self.path.to_string_lossy().to_string();
        if !filters.check_mode(&mode) {
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

        let mut contracts = self.metadata.contracts.clone();
        if contracts.is_empty() {
            let contract_name = if compiler.has_multiple_contracts() {
                format!("{}:{}", test_path, SIMPLE_TESTS_CONTRACT_NAME)
            } else {
                test_path.clone()
            };
            contracts.insert(SIMPLE_TESTS_INSTANCE.to_owned(), contract_name);
        }

        let mut address_predictor = EraVMAddressPredictor::new();

        let mut libraries_instances_names = Vec::new();
        let mut libraries_for_compiler = BTreeMap::new();
        let mut libraries_instances_addresses = HashMap::new();

        for (file, metadata_file_libraries) in self.metadata.libraries.iter() {
            let mut file_path = self.path.clone();
            file_path.pop();
            file_path.push(file);
            let mut file_libraries = BTreeMap::new();
            for (name, instance) in metadata_file_libraries.iter() {
                let address = address_predictor.next(
                    &web3::types::Address::from_str(DEFAULT_CALLER_ADDRESS)
                        .expect("Invalid default caller address constant"),
                    true,
                );
                file_libraries.insert(
                    name.to_owned(),
                    format!("0x{}", crate::utils::address_as_string(&address)),
                );
                libraries_instances_addresses.insert(instance.to_owned(), address);
                libraries_instances_names.push(instance.to_owned());
            }
            libraries_for_compiler.insert(file_path.to_string_lossy().to_string(), file_libraries);
        }

        let vm_input = match compiler
            .compile_for_eravm(
                test_path.clone(),
                self.sources.clone(),
                libraries_for_compiler,
                &mode,
                self.metadata.system_mode,
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

        let instances_names = contracts.keys().cloned().collect::<BTreeSet<String>>();
        let mut instances = HashMap::new();

        for (instance, path) in contracts.into_iter() {
            let build = match vm_input.builds.get(&path).ok_or_else(|| {
                anyhow::anyhow!("{} not found in the compiler build artifacts", path)
            }) {
                Ok(build) => build,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };
            let hash = build.bytecode_hash;
            let address = libraries_instances_addresses.get(&instance).cloned();
            instances.insert(instance, Instance::new(path, address, hash));
        }

        let mut cases = Vec::with_capacity(self.metadata.cases.len());
        for case in self.metadata.cases.iter() {
            if let Some(filters) = case.modes.as_ref() {
                if !mode.check_extended_filters(filters.as_slice()) {
                    continue;
                }
            }

            let mut case = case.clone();
            match case.normalize_deployer_calls(&instances_names, &libraries_instances_names) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            }
            case.normalize_expected();

            let mut address_predictor = address_predictor.clone();

            let instances_addresses = match case
                .instance_addresses(
                    &libraries_instances_names.clone().into_iter().collect(),
                    &mut address_predictor,
                    &mode,
                )
                .map_err(|error| {
                    anyhow::anyhow!(
                        "Case `{}` is invalid: Failed to compute instances addresses: {}",
                        case.name,
                        error
                    )
                }) {
                Ok(addresses) => addresses,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };
            let mut instances = instances.clone();
            for (instance, address) in instances_addresses {
                let instance = instances
                    .get_mut(&instance)
                    .expect("Redundant instance from the instances_addresses case method");
                instance.address = Some(address);
            }

            let case = match Case::try_from_matter_labs(
                &case,
                &mode,
                &instances,
                &vm_input.method_identifiers,
            )
            .map_err(|error| anyhow::anyhow!("Case `{}` is invalid: {}", case.name, error))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };

            cases.push(case);
        }

        let builds = vm_input
            .builds
            .into_values()
            .map(|build| (build.bytecode_hash, build.assembly))
            .collect();

        Some(EraVMTest::new(
            test_path,
            self.metadata.group.clone(),
            mode,
            builds,
            cases,
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
        let test_path = self.path.to_string_lossy().to_string();
        if !filters.check_mode(&mode) {
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

        let mut contracts = self.metadata.contracts.clone();
        if contracts.is_empty() {
            let contract_name = if compiler.has_multiple_contracts() {
                format!("{}:{}", test_path, SIMPLE_TESTS_CONTRACT_NAME)
            } else {
                test_path.clone()
            };
            contracts.insert(SIMPLE_TESTS_INSTANCE.to_owned(), contract_name);
        }

        let mut address_predictor = EVMAddressPredictor::new();

        let mut libraries_instances_names = Vec::new();
        let mut libraries_for_compiler = BTreeMap::new();
        let mut libraries_instances_addresses = HashMap::new();

        for (file, metadata_file_libraries) in self.metadata.libraries.iter() {
            let mut file_path = self.path.clone();
            file_path.pop();
            file_path.push(file);
            let mut file_libraries = BTreeMap::new();
            for (name, instance) in metadata_file_libraries.iter() {
                let address = address_predictor.next(
                    &web3::types::Address::from_str(DEFAULT_CALLER_ADDRESS)
                        .expect("Invalid default caller address constant"),
                    true,
                );
                file_libraries.insert(
                    name.to_owned(),
                    format!("0x{}", crate::utils::address_as_string(&address)),
                );
                libraries_instances_addresses.insert(instance.to_owned(), address);
                libraries_instances_names.push(instance.to_owned());
            }
            libraries_for_compiler.insert(file_path.to_string_lossy().to_string(), file_libraries);
        }

        let mut vm_input = match compiler
            .compile_for_evm(
                test_path.clone(),
                self.sources.clone(),
                BTreeMap::new(),
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

        let instances_names = contracts.keys().cloned().collect::<BTreeSet<String>>();
        let mut instances = HashMap::new();

        for (instance, path) in contracts.into_iter() {
            let build = match vm_input.builds.get_mut(&path).ok_or_else(|| {
                anyhow::anyhow!("{} not found in the compiler build artifacts", path)
            }) {
                Ok(build) => build,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };
            let address = libraries_instances_addresses.get(&instance).cloned();
            instances.insert(
                instance,
                Instance::new(path, address, web3::types::U256::zero()),
            );
        }

        let mut cases = Vec::with_capacity(self.metadata.cases.len());
        for case in self.metadata.cases.iter() {
            if let Some(filters) = case.modes.as_ref() {
                if !mode.check_extended_filters(filters.as_slice()) {
                    continue;
                }
            }

            let mut case = case.clone();
            match case.normalize_deployer_calls(&instances_names, &libraries_instances_names) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            }
            case.normalize_expected();

            let mut address_predictor = address_predictor.clone();

            let instances_addresses = match case
                .instance_addresses(
                    &libraries_instances_names.clone().into_iter().collect(),
                    &mut address_predictor,
                    &mode,
                )
                .map_err(|error| {
                    anyhow::anyhow!(
                        "Case `{}` is invalid: Failed to compute instances addresses: {}",
                        case.name,
                        error
                    )
                }) {
                Ok(addresses) => addresses,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };
            let mut instances = instances.clone();
            for (instance, address) in instances_addresses {
                let instance = instances
                    .get_mut(&instance)
                    .expect("Redundant instance from the instances_addresses case method");
                instance.address = Some(address);
            }

            let case = match Case::try_from_matter_labs(
                &case,
                &mode,
                &instances,
                &vm_input.method_identifiers,
            )
            .map_err(|error| anyhow::anyhow!("Case `{}` is invalid: {}", case.name, error))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode), test_path, error);
                    return None;
                }
            };

            cases.push(case);
        }

        let builds = vm_input
            .builds
            .into_values()
            .map(|build| (web3::types::Address::zero(), build))
            .collect();

        Some(EVMTest::new(
            test_path,
            self.metadata.group.clone(),
            mode,
            builds,
            cases,
        ))
    }
}
