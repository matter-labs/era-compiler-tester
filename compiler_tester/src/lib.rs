//!
//! The compiler tester library.
//!

#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]

pub(crate) mod compilers;
pub(crate) mod directories;
pub(crate) mod filters;
pub(crate) mod summary;
pub(crate) mod target;
pub(crate) mod test;
pub(crate) mod utils;
pub(crate) mod vm;
pub(crate) mod workflow;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

pub use crate::compilers::eravm::EraVMCompiler;
pub use crate::compilers::llvm::LLVMCompiler;
pub use crate::compilers::mode::llvm_options::LLVMOptions;
pub use crate::compilers::mode::Mode;
pub use crate::compilers::solidity::mode::Mode as SolidityMode;
pub use crate::compilers::solidity::upstream::SolidityCompiler as SolidityUpstreamCompiler;
pub use crate::compilers::solidity::SolidityCompiler;
pub use crate::compilers::vyper::VyperCompiler;
pub use crate::compilers::yul::YulCompiler;
pub use crate::compilers::Compiler;
pub use crate::directories::ethereum::test::EthereumTest;
pub use crate::directories::ethereum::EthereumDirectory;
pub use crate::directories::matter_labs::MatterLabsDirectory;
pub use crate::directories::Buildable;
pub use crate::directories::Collection;
pub use crate::filters::Filters;
pub use crate::summary::Summary;
pub use crate::target::Target;
pub use crate::vm::eravm::deployers::dummy_deployer::DummyDeployer as EraVMNativeDeployer;
pub use crate::vm::eravm::deployers::system_contract_deployer::SystemContractDeployer as EraVMSystemContractDeployer;
pub use crate::vm::eravm::deployers::EraVMDeployer;
pub use crate::vm::eravm::EraVM;
pub use crate::vm::evm::EVM;
pub use crate::workflow::Workflow;

/// The debug directory path.
pub const DEBUG_DIRECTORY: &str = "./debug/";

///
/// The compiler test generic representation.
///
type Test = (Arc<dyn Buildable>, Arc<dyn Compiler>, Mode);

///
/// The compiler tester.
///
pub struct CompilerTester {
    /// The summary.
    pub summary: Arc<Mutex<Summary>>,
    /// The filters.
    pub filters: Filters,
    /// The debug config.
    pub debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    /// Actions to perform.
    pub workflow: Workflow,
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
    const ERAVM_SIMPLE: &'static str = "tests/eravm";
}

impl CompilerTester {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        summary: Arc<Mutex<Summary>>,
        filters: Filters,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
        workflow: Workflow,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            summary,
            filters,
            debug_config,
            workflow,
        })
    }

    ///
    /// Runs all tests on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(self, vm: EraVM) -> anyhow::Result<()>
    where
        D: EraVMDeployer,
    {
        let tests = self.all_tests(false)?;
        let vm = Arc::new(vm);

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, compiler, mode)| {
                let mode_string = mode.to_string();
                let specialized_debug_config = self
                    .debug_config
                    .as_ref()
                    .and_then(|config| config.create_subdirectory(mode_string.as_str()).ok());
                if let Some(test) = test.build_for_eravm(
                    mode,
                    compiler,
                    Target::EraVM,
                    self.summary.clone(),
                    &self.filters,
                    specialized_debug_config,
                ) {
                    if let Workflow::BuildAndRun = self.workflow {
                        test.run_eravm::<D, M>(self.summary.clone(), vm.clone())
                    };
                }
            })
            .collect();

        Ok(())
    }

    ///
    /// Runs all tests on EVM.
    ///
    pub fn run_evm(self, use_upstream_solc: bool) -> anyhow::Result<()> {
        let tests = self.all_tests(use_upstream_solc)?;

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, compiler, mode)| {
                let mode_string = mode.to_string();
                let specialized_debug_config = self
                    .debug_config
                    .as_ref()
                    .and_then(|config| config.create_subdirectory(mode_string.as_str()).ok());
                if let Some(test) = test.build_for_evm(
                    mode,
                    compiler,
                    Target::EVM,
                    self.summary.clone(),
                    &self.filters,
                    specialized_debug_config,
                ) {
                    if let Workflow::BuildAndRun = self.workflow {
                        test.run_evm(self.summary.clone())
                    };
                }
            })
            .collect();

        Ok(())
    }

    ///
    /// Runs all tests on EVM interpreter.
    ///
    pub fn run_evm_interpreter<D, const M: bool>(
        self,
        vm: EraVM,
        use_upstream_solc: bool,
    ) -> anyhow::Result<()>
    where
        D: EraVMDeployer,
    {
        let tests = self.all_tests(use_upstream_solc)?;
        let vm = Arc::new(vm);

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, compiler, mode)| {
                if let Some(test) = test.build_for_evm(
                    mode,
                    compiler,
                    Target::EVMInterpreter,
                    self.summary.clone(),
                    &self.filters,
                    self.debug_config.clone(),
                ) {
                    if let Workflow::BuildAndRun = self.workflow {
                        test.run_evm_interpreter::<D, M>(self.summary.clone(), vm.clone());
                    }
                }
            })
            .collect();

        Ok(())
    }

    ///
    /// Returns all tests from all directories.
    ///
    fn all_tests(&self, use_upstream_solc: bool) -> anyhow::Result<Vec<Test>> {
        let solidity_compiler = Arc::new(SolidityCompiler::new());
        let solidity_upstream_compiler = Arc::new(SolidityUpstreamCompiler::new());
        let vyper_compiler = Arc::new(VyperCompiler::new());
        let yul_compiler = Arc::new(YulCompiler);
        let llvm_compiler = Arc::new(LLVMCompiler);
        let eravm_compiler = Arc::new(EraVMCompiler);

        let mut tests = Vec::with_capacity(16384);

        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::SOLIDITY_SIMPLE,
            era_compiler_common::EXTENSION_SOLIDITY,
            if use_upstream_solc {
                solidity_upstream_compiler.clone()
            } else {
                solidity_compiler.clone()
            },
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
            if use_upstream_solc {
                solidity_upstream_compiler.clone()
            } else {
                solidity_compiler.clone()
            },
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            Self::VYPER_COMPLEX,
            era_compiler_common::EXTENSION_JSON,
            vyper_compiler.clone(),
        )?);

        tests.extend(self.directory::<EthereumDirectory>(
            Self::SOLIDITY_ETHEREUM,
            era_compiler_common::EXTENSION_SOLIDITY,
            if use_upstream_solc {
                solidity_upstream_compiler.clone()
            } else {
                solidity_compiler.clone()
            },
        )?);
        tests.extend(self.directory::<EthereumDirectory>(
            Self::VYPER_ETHEREUM,
            era_compiler_common::EXTENSION_VYPER,
            vyper_compiler,
        )?);

        Ok(tests)
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
        T: Collection,
    {
        Ok(T::read_all(
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
        .cartesian_product(compiler.all_modes())
        .map(|(test, mode)| (test, compiler.clone() as Arc<dyn Compiler>, mode))
        .collect())
    }
}
