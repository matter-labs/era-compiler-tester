//!
//! The compiler tester library.
//!

pub(crate) mod compilers;
pub(crate) mod directories;
pub(crate) mod filters;
pub(crate) mod llvm_options;
pub(crate) mod summary;
pub(crate) mod test;
pub(crate) mod utils;
pub(crate) mod vm;

pub use self::filters::Filters;
pub use self::llvm_options::LLVMOptions;
pub use self::summary::Summary;
pub use crate::vm::eravm::deployers::native_deployer::NativeDeployer as EraVMNativeDeployer;
pub use crate::vm::eravm::deployers::system_contract_deployer::SystemContractDeployer as EraVMSystemContractDeployer;
pub use crate::vm::eravm::EraVM;
pub use crate::vm::evm::EVM;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::compilers::eravm::EraVMCompiler;
use crate::compilers::llvm::LLVMCompiler;
use crate::compilers::mode::Mode;
use crate::compilers::solidity::SolidityCompiler;
use crate::compilers::vyper::VyperCompiler;
use crate::compilers::yul::YulCompiler;
use crate::compilers::Compiler;
use crate::directories::ethereum::EthereumDirectory;
use crate::directories::matter_labs::MatterLabsDirectory;
use crate::directories::Buildable;
use crate::directories::TestsDirectory;
use crate::vm::eravm::deployers::Deployer as EraVMDeployer;

/// The debug directory path.
pub const DEBUG_DIRECTORY: &str = "./debug/";

/// The trace directory path.
pub const TRACE_DIRECTORY: &str = "./trace/";

///
/// The compiler test generic representation.
///
type Test = (Arc<dyn Buildable>, Arc<dyn Compiler>, Mode);

///
/// The compiler tester.
///
pub struct CompilerTester {
    /// The summary.
    summary: Arc<Mutex<Summary>>,
    /// The filters.
    filters: Filters,
    /// The debug config.
    debug_config: Option<era_compiler_llvm_context::DebugConfig>,
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

    /// The EraVM simple tests directory.
    const ERAVM_SIMPLE: &'static str = "tests/zkevm";
}

impl CompilerTester {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        summary: Arc<Mutex<Summary>>,
        filters: Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            summary,
            filters,
            debug_config,
        })
    }

    ///
    /// Runs all tests on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(self, vm: EraVM) -> anyhow::Result<()>
    where
        D: EraVMDeployer,
    {
        let tests = self.all_tests()?;
        let vm = Arc::new(vm);

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, compiler, mode)| {
                if let Some(test) = test.build_for_eravm(
                    mode,
                    compiler,
                    self.summary.clone(),
                    &self.filters,
                    self.debug_config.clone(),
                ) {
                    test.run::<D, M>(self.summary.clone(), vm.clone());
                }
            })
            .collect();

        Ok(())
    }

    ///
    /// Runs all tests on EraVM.
    ///
    pub fn run_evm(self) -> anyhow::Result<()> {
        let tests = self.all_tests()?;

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, compiler, mode)| {
                if let Some(test) = test.build_for_evm(
                    mode,
                    compiler,
                    self.summary.clone(),
                    &self.filters,
                    self.debug_config.clone(),
                ) {
                    test.run(self.summary.clone());
                }
            })
            .collect();

        Ok(())
    }

    ///
    /// Returns all tests from the specified directory for the specified compiler.
    ///
    fn directory<T>(
        &self,
        path: &str,
        extension: &'static str,
        compiler: Arc<dyn Compiler>,
    ) -> anyhow::Result<Vec<Test>>
    where
        T: TestsDirectory,
    {
        Ok(T::all_tests(
            Path::new(path),
            extension,
            self.summary.clone(),
            &self.filters,
        )
        .map_err(|error| {
            anyhow::anyhow!("Failed to read the tests directory `{}`: {}", path, error)
        })?
        .into_iter()
        .map(|test| Arc::new(test) as Arc<dyn Buildable>)
        .cartesian_product(compiler.modes())
        .map(|(test, mode)| (test, compiler.clone() as Arc<dyn Compiler>, mode))
        .collect())
    }

    ///
    /// Returns all tests from all directories.
    ///
    fn all_tests(&self) -> anyhow::Result<Vec<Test>> {
        let solidity_compiler = Arc::new(SolidityCompiler::new());
        let vyper_compiler = Arc::new(VyperCompiler::new());
        let yul_compiler = Arc::new(YulCompiler);
        let llvm_compiler = Arc::new(LLVMCompiler);
        let eravm_compiler = Arc::new(EraVMCompiler);

        let mut tests = Vec::with_capacity(16384);

        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::SOLIDITY_SIMPLE,
            era_compiler_common::EXTENSION_SOLIDITY,
            solidity_compiler.clone(),
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::VYPER_SIMPLE,
            era_compiler_common::EXTENSION_VYPER,
            vyper_compiler.clone(),
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::YUL_SIMPLE,
            era_compiler_common::EXTENSION_YUL,
            yul_compiler,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::LLVM_SIMPLE,
            era_compiler_common::EXTENSION_LLVM_SOURCE,
            llvm_compiler,
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::ERAVM_SIMPLE,
            era_compiler_common::EXTENSION_ERAVM_ASSEMBLY,
            eravm_compiler,
        )?);

        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::SOLIDITY_COMPLEX,
            era_compiler_common::EXTENSION_JSON,
            solidity_compiler.clone(),
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::VYPER_COMPLEX,
            era_compiler_common::EXTENSION_JSON,
            vyper_compiler.clone(),
        )?);

        tests.extend(self.directory::<EthereumDirectory>(
            Self::SOLIDITY_ETHEREUM,
            era_compiler_common::EXTENSION_SOLIDITY,
            solidity_compiler,
        )?);
        tests.extend(self.directory::<EthereumDirectory>(
            Self::VYPER_ETHEREUM,
            era_compiler_common::EXTENSION_VYPER,
            vyper_compiler,
        )?);

        Ok(tests)
    }
}
