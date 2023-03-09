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
use crate::deployers::address_predictor::AddressPredictor;
use crate::directories::Buildable;
use crate::filters::Filters;
use crate::summary::Summary;
use crate::test::case::Case;
use crate::test::instance::Instance;
use crate::test::Test;

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
pub struct MatterLabsTest<C>
where
    C: Compiler,
{
    /// The test name.
    test_name: String,
    /// The test compiler.
    compiler: C,
    /// The address predictor.
    address_predictor: AddressPredictor,
    /// The test metadata.
    metadata: Metadata,
    /// The libraries addresses.
    libraries_instances: HashMap<String, web3::types::Address>,
}

impl<C> MatterLabsTest<C>
where
    C: Compiler,
{
    ///
    /// Try to create new test.
    ///
    pub fn new(
        path: PathBuf,
        summary: Arc<Mutex<Summary>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        filters: &Filters,
    ) -> Option<Self> {
        let test_name = path.to_string_lossy().to_string();

        if !filters.check_test_path(&test_name) {
            return None;
        }

        let metadata_file_string = match std::fs::read_to_string(path.as_path()) {
            Ok(metadata_file_string) => metadata_file_string,
            Err(error) => {
                Summary::invalid(summary, None, test_name, error);
                return None;
            }
        };

        let mut metadata = match Metadata::from_str(metadata_file_string.as_str())
            .map_err(|err| anyhow::anyhow!("Invalid metadata json: {}", err))
        {
            Ok(metadata) => metadata,
            Err(error) => {
                Summary::invalid(summary, None, test_name, error);
                return None;
            }
        };

        if metadata.ignore {
            Summary::ignored(summary, test_name);
            return None;
        }

        if !filters.check_group(&metadata.group) {
            return None;
        }

        let sources = if metadata.contracts.is_empty() {
            let contract_name = if C::has_many_contracts() {
                format!("{}:{}", path.to_string_lossy(), SIMPLE_TESTS_CONTRACT_NAME)
            } else {
                path.to_string_lossy().to_string()
            };
            metadata
                .contracts
                .insert(SIMPLE_TESTS_INSTANCE.to_owned(), contract_name);
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
                        Summary::invalid(summary, None, test_name, error);
                        return None;
                    }
                };
                sources.insert(path, source_code);
            }
            sources.into_iter().collect()
        };

        let mut address_predictor = AddressPredictor::new();

        let instances = metadata
            .contracts
            .keys()
            .cloned()
            .collect::<BTreeSet<String>>();

        let mut libraries_instances = Vec::new();
        let mut libraries_for_compiler = BTreeMap::new();
        let mut libraries_instances_addresses = HashMap::new();

        for (file, metadata_file_libraries) in metadata.libraries.iter() {
            let mut file_path = path.clone();
            file_path.pop();
            file_path.push(file);
            let mut file_libraries = BTreeMap::new();
            for (name, instance) in metadata_file_libraries.iter() {
                let address = address_predictor.next_address(
                    web3::types::Address::from_str(DEFAULT_CALLER_ADDRESS)
                        .expect("Invalid default caller address constant"),
                );
                file_libraries.insert(
                    name.to_owned(),
                    format!("0x{}", crate::utils::address_as_string(&address)),
                );
                libraries_instances_addresses.insert(instance.to_owned(), address);
                libraries_instances.push(instance.to_owned());
            }
            libraries_for_compiler.insert(file_path.to_string_lossy().to_string(), file_libraries);
        }

        let mut cases = Vec::with_capacity(metadata.cases.len());
        for mut case in metadata.cases.into_iter() {
            let case_name = format!("{}::{}", test_name, case.name);
            if case.ignore {
                Summary::ignored(summary.clone(), case_name);
                continue;
            }

            if !filters.check_case_path(&case_name) {
                continue;
            }

            match case.normalize_deployer_calls(&instances, &libraries_instances) {
                Ok(_) => {}
                Err(error) => {
                    Summary::invalid(summary, None, test_name, error);
                    return None;
                }
            }
            case.normalize_expected();
            cases.push(case);
        }

        metadata.cases = cases;

        Some(Self {
            test_name,
            compiler: C::new(
                sources,
                libraries_for_compiler,
                debug_config,
                metadata.system_mode,
            ),
            address_predictor,
            metadata,
            libraries_instances: libraries_instances_addresses,
        })
    }
}

impl<C> Buildable for MatterLabsTest<C>
where
    C: Compiler,
{
    fn build(&self, mode: Mode, summary: Arc<Mutex<Summary>>, filters: &Filters) -> Option<Test> {
        if !filters.check_mode(&mode) {
            return None;
        }

        if let Some(filters) = self.metadata.modes.as_ref() {
            if !Filters::check_mode_filters(&mode, filters.as_slice()) {
                return None;
            }
        }

        if !self.compiler.check_pragmas(&mode) {
            return None;
        }

        let mut instances = HashMap::new();

        let builds = match self
            .compiler
            .compile(&mode, false)
            .map_err(|error| anyhow::anyhow!("Failed to compile sources: {}", error))
            .and_then(|builds| {
                for (instance, path) in self.metadata.contracts.iter() {
                    let build = builds.get(path).ok_or_else(|| {
                        anyhow::anyhow!("{} not found in the compiler build artifacts", path)
                    })?;
                    let hash = build.bytecode_hash;
                    let address = self.libraries_instances.get(instance).cloned();
                    instances.insert(instance.clone(), Instance::new(path.clone(), address, hash));
                }
                Ok(builds
                    .into_values()
                    .map(|build| (build.bytecode_hash, build.assembly))
                    .collect())
            }) {
            Ok(builds) => builds,
            Err(error) => {
                Summary::invalid(summary, Some(mode.clone()), self.test_name.clone(), error);
                return None;
            }
        };

        let mut cases = Vec::with_capacity(self.metadata.cases.len());
        for case in self.metadata.cases.iter() {
            let mut address_predictor = self.address_predictor.clone();

            if let Some(filters) = case.modes.as_ref() {
                if !Filters::check_mode_filters(&mode, filters.as_slice()) {
                    continue;
                }
            }

            let instances_addresses = match case
                .instances_addresses(
                    &self
                        .libraries_instances
                        .keys()
                        .cloned()
                        .collect::<BTreeSet<String>>(),
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
                    Summary::invalid(summary, Some(mode.clone()), self.test_name.clone(), error);
                    return None;
                }
            };
            let mut instances = instances.clone();
            for (instance, address) in instances_addresses {
                if let Some(instance) = instances.get_mut(&instance) {
                    instance.address = Some(address);
                }
            }

            let case = match Case::from_matter_labs(case, &mode, &instances, &self.compiler)
                .map_err(|error| anyhow::anyhow!("Case `{}` is invalid: {}", case.name, error))
            {
                Ok(case) => case,
                Err(error) => {
                    Summary::invalid(summary, Some(mode.clone()), self.test_name.clone(), error);
                    return None;
                }
            };

            cases.push(case);
        }

        Some(Test::new(
            self.test_name.clone(),
            self.metadata.group.clone(),
            mode,
            builds,
            cases,
        ))
    }
}
