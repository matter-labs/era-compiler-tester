//!
//! The compiler tester library.
//!

#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub(crate) mod compilers;
pub(crate) mod directories;
pub(crate) mod environment;
pub(crate) mod filters;
pub(crate) mod summary;
pub(crate) mod test;
pub(crate) mod toolchain;
pub(crate) mod utils;
pub(crate) mod vm;
pub(crate) mod workflow;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

pub use crate::compilers::eravm_assembly::EraVMAssemblyCompiler;
pub use crate::compilers::llvm_ir::LLVMIRCompiler;
pub use crate::compilers::mode::llvm_options::LLVMOptions;
pub use crate::compilers::mode::Mode;
pub use crate::compilers::solidity::solc::compiler::standard_json::input::language::Language as SolcStandardJsonInputLanguage;
pub use crate::compilers::solidity::solc::SolidityCompiler as SolcCompiler;
pub use crate::compilers::solidity::solx::SolidityCompiler as SolxCompiler;
pub use crate::compilers::solidity::zksolc::mode::Mode as ZksolcMode;
pub use crate::compilers::solidity::zksolc::SolidityCompiler as ZksolcCompiler;
pub use crate::compilers::vyper::VyperCompiler;
pub use crate::compilers::yul::YulCompiler;
pub use crate::compilers::Compiler;
pub use crate::directories::ethereum::test::EthereumTest;
pub use crate::directories::ethereum::EthereumDirectory;
pub use crate::directories::matter_labs::MatterLabsDirectory;
pub use crate::directories::Buildable;
pub use crate::directories::Collection;
pub use crate::environment::Environment;
pub use crate::filters::Filters;
pub use crate::summary::Summary;
pub use crate::toolchain::Toolchain;
pub use crate::vm::eravm::deployers::dummy_deployer::DummyDeployer as EraVMNativeDeployer;
pub use crate::vm::eravm::deployers::system_contract_deployer::SystemContractDeployer as EraVMSystemContractDeployer;
pub use crate::vm::eravm::deployers::EraVMDeployer;
pub use crate::vm::eravm::EraVM;
pub use crate::vm::revm::REVM;
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
    /// The Solidity Ethereum upstream tests directory.
    const SOLIDITY_ETHEREUM_UPSTREAM: &'static str = "era-solidity/test/libsolidity/semanticTests";

    /// The Vyper simple tests directory.
    const VYPER_SIMPLE: &'static str = "tests/vyper/simple";
    /// The Vyper complex tests directory.
    const VYPER_COMPLEX: &'static str = "tests/vyper/complex";
    /// The Vyper Ethereum tests directory.
    const VYPER_ETHEREUM: &'static str = "tests/vyper/ethereum";

    /// The Yul simple tests directory.
    const YUL_SIMPLE: &'static str = "tests/yul";

    /// The EraVM LLVM IR simple tests directory.
    const LLVM_SIMPLE_ERAVM: &'static str = "tests/llvm/eravm";
    /// The EVM LLVM IR simple tests directory.
    const LLVM_SIMPLE_EVM: &'static str = "tests/llvm/evm";

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
    pub fn run_eravm<D, const M: bool>(self, vm: EraVM, toolchain: Toolchain) -> anyhow::Result<()>
    where
        D: EraVMDeployer,
    {
        let tests = self.all_tests(
            benchmark_analyzer::Target::EraVM,
            Environment::ZkEVM,
            toolchain,
            None,
        )?;
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
                    Environment::ZkEVM,
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
    /// Runs all tests on REVM.
    ///
    pub fn run_revm(self, toolchain: Toolchain, solx: Option<PathBuf>) -> anyhow::Result<()> {
        let tests = self.all_tests(
            benchmark_analyzer::Target::EVM,
            Environment::REVM,
            toolchain,
            solx,
        )?;

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
                    Environment::REVM,
                    self.summary.clone(),
                    &self.filters,
                    specialized_debug_config,
                ) {
                    if let Workflow::BuildAndRun = self.workflow {
                        test.run_revm(self.summary.clone())
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
        toolchain: Toolchain,
        solx: Option<PathBuf>,
    ) -> anyhow::Result<()>
    where
        D: EraVMDeployer,
    {
        let tests = self.all_tests(
            benchmark_analyzer::Target::EVM,
            Environment::EVMInterpreter,
            toolchain,
            solx,
        )?;
        let vm = Arc::new(vm);

        let _: Vec<()> = tests
            .into_par_iter()
            .map(|(test, compiler, mode)| {
                if let Some(test) = test.build_for_evm(
                    mode,
                    compiler,
                    Environment::EVMInterpreter,
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
    fn all_tests(
        &self,
        target: benchmark_analyzer::Target,
        environment: Environment,
        toolchain: Toolchain,
        solx: Option<PathBuf>,
    ) -> anyhow::Result<Vec<Test>> {
        let solx_path = solx.unwrap_or_else(|| PathBuf::from("solx"));

        let (solidity_compiler, yul_compiler, llvm_ir_compiler): (
            Arc<dyn Compiler>,
            Arc<dyn Compiler>,
            Arc<dyn Compiler>,
        ) = match (target, toolchain) {
            (benchmark_analyzer::Target::EraVM, Toolchain::IrLLVM) => {
                let solidity_compiler = Arc::new(ZksolcCompiler::new());
                let yul_compiler = Arc::new(YulCompiler::Zksolc);
                let llvm_ir_compiler = Arc::new(LLVMIRCompiler::Zksolc);
                (solidity_compiler, yul_compiler, llvm_ir_compiler)
            }
            (benchmark_analyzer::Target::EVM, Toolchain::IrLLVM) => {
                let solidity_compiler = Arc::new(SolxCompiler::try_from_path(solx_path)?);
                let yul_compiler = Arc::new(YulCompiler::Solx(solidity_compiler.clone()));
                let llvm_ir_compiler = Arc::new(LLVMIRCompiler::Solx(solidity_compiler.clone()));
                (solidity_compiler, yul_compiler, llvm_ir_compiler)
            }
            (_, Toolchain::Zksolc) => {
                let solidity_compiler = Arc::new(ZksolcCompiler::new());
                let yul_compiler = Arc::new(YulCompiler::Zksolc);
                let llvm_ir_compiler = Arc::new(LLVMIRCompiler::Zksolc);
                (solidity_compiler, yul_compiler, llvm_ir_compiler)
            }
            (_, Toolchain::Solc | Toolchain::SolcLLVM) => {
                let solidity_compiler = Arc::new(SolcCompiler::new(
                    SolcStandardJsonInputLanguage::Solidity,
                    toolchain,
                ));
                let yul_compiler = Arc::new(SolcCompiler::new(
                    SolcStandardJsonInputLanguage::Yul,
                    toolchain,
                ));
                let llvm_ir_compiler = Arc::new(LLVMIRCompiler::Zksolc);
                (solidity_compiler, yul_compiler, llvm_ir_compiler)
            }
        };
        let eravm_assembly_compiler = Arc::new(EraVMAssemblyCompiler::default());

        let mut tests = Vec::with_capacity(16384);

        tests.extend(self.directory::<MatterLabsDirectory>(
            target,
            environment,
            Self::SOLIDITY_SIMPLE,
            era_compiler_common::EXTENSION_SOLIDITY,
            solidity_compiler.clone(),
        )?);
        tests.extend(self.directory::<MatterLabsDirectory>(
            target,
            environment,
            Self::SOLIDITY_COMPLEX,
            era_compiler_common::EXTENSION_JSON,
            solidity_compiler.clone(),
        )?);
        tests.extend(self.directory::<EthereumDirectory>(
            target,
            environment,
            match target {
                benchmark_analyzer::Target::EraVM => Self::SOLIDITY_ETHEREUM,
                benchmark_analyzer::Target::EVM => Self::SOLIDITY_ETHEREUM_UPSTREAM,
            },
            era_compiler_common::EXTENSION_SOLIDITY,
            solidity_compiler.clone(),
        )?);

        tests.extend(self.directory::<MatterLabsDirectory>(
            target,
            environment,
            Self::YUL_SIMPLE,
            era_compiler_common::EXTENSION_YUL,
            yul_compiler,
        )?);

        tests.extend(self.directory::<MatterLabsDirectory>(
            target,
            environment,
            match target {
                benchmark_analyzer::Target::EraVM => Self::LLVM_SIMPLE_ERAVM,
                benchmark_analyzer::Target::EVM => Self::LLVM_SIMPLE_EVM,
            },
            era_compiler_common::EXTENSION_LLVM_SOURCE,
            llvm_ir_compiler,
        )?);

        if let benchmark_analyzer::Target::EraVM = target {
            let vyper_compiler = Arc::new(VyperCompiler::new());
            tests.extend(self.directory::<MatterLabsDirectory>(
                target,
                environment,
                Self::ERAVM_SIMPLE,
                era_compiler_common::EXTENSION_ERAVM_ASSEMBLY,
                eravm_assembly_compiler,
            )?);

            tests.extend(self.directory::<MatterLabsDirectory>(
                target,
                environment,
                Self::VYPER_SIMPLE,
                era_compiler_common::EXTENSION_VYPER,
                vyper_compiler.clone(),
            )?);
            tests.extend(self.directory::<MatterLabsDirectory>(
                target,
                environment,
                Self::VYPER_COMPLEX,
                era_compiler_common::EXTENSION_JSON,
                vyper_compiler.clone(),
            )?);
            tests.extend(self.directory::<EthereumDirectory>(
                target,
                environment,
                Self::VYPER_ETHEREUM,
                era_compiler_common::EXTENSION_VYPER,
                vyper_compiler,
            )?);
        }

        Ok(tests)
    }

    ///
    /// Returns all tests from the specified directory for the specified compiler.
    ///
    fn directory<T>(
        &self,
        target: benchmark_analyzer::Target,
        environment: Environment,
        path: &str,
        extension: &'static str,
        compiler: Arc<dyn Compiler>,
    ) -> anyhow::Result<Vec<Test>>
    where
        T: Collection,
    {
        let directory_path = crate::utils::str_to_path_normalized(path);
        Ok(T::read_all(
            target,
            environment,
            directory_path.as_path(),
            extension,
            self.summary.clone(),
            &self.filters,
        )
        .map_err(|error| {
            anyhow::anyhow!("Failed to read the tests directory {directory_path:?}: {error}")
        })?
        .into_iter()
        .map(|test| Arc::new(test) as Arc<dyn Buildable>)
        .cartesian_product(compiler.all_modes())
        .map(|(test, mode)| (test, compiler.clone() as Arc<dyn Compiler>, mode))
        .collect())
    }
}
