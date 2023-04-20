//!
//! The compiler tester library.
//!

pub(crate) mod compilers;
pub(crate) mod deployers;
pub(crate) mod directories;
pub(crate) mod filters;
pub(crate) mod llvm_options;
pub(crate) mod summary;
pub(crate) mod test;
pub(crate) mod utils;
pub(crate) mod zkevm;

pub use self::deployers::native_deployer::NativeDeployer;
pub use self::deployers::system_contract_deployer::SystemContractDeployer;
pub use self::filters::Filters;
pub use self::llvm_options::LLVMOptions;
pub use self::summary::Summary;

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use colored::Colorize;
use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::compilers::downloader::Downloader as CompilerDownloader;
use crate::compilers::llvm::LLVMCompiler;
use crate::compilers::mode::Mode;
use crate::compilers::solidity::SolidityCompiler;
use crate::compilers::vyper::VyperCompiler;
use crate::compilers::yul::YulCompiler;
use crate::compilers::zkevm::zkEVMCompiler;
use crate::compilers::Compiler;
use crate::deployers::Deployer;
use crate::directories::ethereum::EthereumDirectory;
use crate::directories::matter_labs::MatterLabsDirectory;
use crate::directories::Buildable;
use crate::directories::TestsDirectory;
use crate::zkevm::zkEVM;

/// The debug directory path.
pub const DEBUG_DIRECTORY: &str = "./debug/";

/// The trace directory path.
pub const TRACE_DIRECTORY: &str = "./trace/";

///
/// The compiler test generic representation.
///
type Test = (Arc<dyn Buildable>, Mode);

///
/// The compiler-tester.
///
pub struct CompilerTester {
    /// The summary.
    summary: Arc<Mutex<Summary>>,
    /// The filters.
    filters: Filters,
    /// The debug config.
    debug_config: Option<compiler_llvm_context::DebugConfig>,
    /// The initial zkEVM.
    initial_vm: Arc<zkEVM>,
}

impl CompilerTester {
    /// The Solidity simple tests directory.
    const SOLIDITY_SIMPLE: &'static str = "tests/solidity/simple";
    /// The Solidity complex tests directory.
    const SOLIDITY_COMPLEX: &'static str = "tests/solidity/complex";
    /// The Solidity Ethereum tests directory.
    const SOLIDITY_ETHEREUM: &'static str = "tests/solidity/ethereum";

    /// The Vyper simple tests directory.
    const VYPER_SIMPLE: &'static str = "tests/vyper/simple";
    /// The Vyper complex tests directory.
    const VYPER_COMPLEX: &'static str = "tests/vyper/complex";
    /// The Vyper Ethereum tests directory.
    const VYPER_ETHEREUM: &'static str = "tests/vyper/ethereum";

    /// The Yul simple tests directory.
    const YUL_SIMPLE: &'static str = "tests/yul";

    /// The LLVM simple tests directory.
    const LLVM_SIMPLE: &'static str = "tests/llvm";

    /// The zkEVM simple tests directory.
    const ZKEVM_SIMPLE: &'static str = "tests/zkevm";
}

impl CompilerTester {
    ///
    /// A shortcut constructor.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        summary: Arc<Mutex<Summary>>,
        filters: Filters,

        debug_config: Option<compiler_llvm_context::DebugConfig>,

        binary_download_config_paths: Vec<PathBuf>,
        system_contracts_download_config_path: PathBuf,
        system_contracts_debug_config: Option<compiler_llvm_context::DebugConfig>,
        system_contracts_path: Option<PathBuf>,
        system_contracts_save_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let mut http_client_builder = reqwest::blocking::ClientBuilder::new();
        http_client_builder = http_client_builder.connect_timeout(Duration::from_secs(60));
        http_client_builder = http_client_builder.pool_idle_timeout(Duration::from_secs(60));
        http_client_builder = http_client_builder.timeout(Duration::from_secs(60));
        let http_client = http_client_builder.build()?;

        let download_time_start = Instant::now();
        println!(" {} compiler binaries", "Downloading".bright_green().bold());
        let system_contracts_solc_downloader_config = CompilerDownloader::new(http_client.clone())
            .download(system_contracts_download_config_path.as_path())?;
        for config_path in binary_download_config_paths.into_iter() {
            CompilerDownloader::new(http_client.clone()).download(config_path.as_path())?;
        }
        println!(
            "    {} downloading compiler binaries in {}m{:02}s",
            "Finished".bright_green().bold(),
            download_time_start.elapsed().as_secs() / 60,
            download_time_start.elapsed().as_secs() % 60,
        );

        let initial_vm = Arc::new(zkEVM::initialize(
            system_contracts_solc_downloader_config,
            system_contracts_debug_config,
            system_contracts_path,
            system_contracts_save_path,
        )?);

        Ok(Self {
            summary,
            filters,

            debug_config,

            initial_vm,
        })
    }

    ///
    /// Runs all the tests.
    ///
    pub fn run<D, const M: bool>(self) -> anyhow::Result<()>
    where
        D: Deployer,
    {
        let mut tests = Vec::new();
        tests.extend(self.directory::<MatterLabsDirectory, SolidityCompiler>(
            Self::SOLIDITY_SIMPLE,
            compiler_common::EXTENSION_SOLIDITY,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory, VyperCompiler>(
            Self::VYPER_SIMPLE,
            compiler_common::EXTENSION_VYPER,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory, YulCompiler>(
            Self::YUL_SIMPLE,
            compiler_common::EXTENSION_YUL,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory, LLVMCompiler>(
            Self::LLVM_SIMPLE,
            compiler_common::EXTENSION_LLVM_SOURCE,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory, zkEVMCompiler>(
            Self::ZKEVM_SIMPLE,
            compiler_common::EXTENSION_ZKEVM_ASSEMBLY,
        )?);

        tests.extend(self.directory::<MatterLabsDirectory, SolidityCompiler>(
            Self::SOLIDITY_COMPLEX,
            compiler_common::EXTENSION_JSON,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory, VyperCompiler>(
            Self::VYPER_COMPLEX,
            compiler_common::EXTENSION_JSON,
        )?);

        tests.extend(self.directory::<EthereumDirectory, SolidityCompiler>(
            Self::SOLIDITY_ETHEREUM,
            compiler_common::EXTENSION_SOLIDITY,
        )?);
        tests.extend(self.directory::<EthereumDirectory, VyperCompiler>(
            Self::VYPER_ETHEREUM,
            compiler_common::EXTENSION_VYPER,
        )?);

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, mode)| {
                if let Some(test) = test.build(mode, self.summary.clone(), &self.filters) {
                    test.run::<D, M>(self.summary.clone(), self.initial_vm.clone());
                }
            })
            .collect();

        Ok(())
    }

    ///
    /// Returns all test from the specified directory.
    ///
    fn directory<T, C>(&self, path: &str, extension: &'static str) -> anyhow::Result<Vec<Test>>
    where
        C: Compiler,
        T: TestsDirectory<C>,
    {
        Ok(T::all_tests(
            Path::new(path),
            extension,
            self.summary.clone(),
            self.debug_config.clone(),
            &self.filters,
        )
        .map_err(|error| {
            anyhow::anyhow!("Failed to read the tests directory `{}`: {}", path, error)
        })?
        .into_iter()
        .map(|test| Arc::new(test) as Arc<dyn Buildable>)
        .cartesian_product(C::modes())
        .collect())
    }
}
