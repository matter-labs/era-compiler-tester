//!
//! The compiler tester Solidity mode.
//!

use itertools::Itertools;

use crate::compilers::mode::llvm_options::LLVMOptions;

use crate::compilers::mode::Mode as ModeWrapper;

///
/// The compiler tester Solidity mode.
///
#[derive(Debug, Clone)]
pub struct Mode {
    /// The Solidity compiler version.
    pub solc_version: semver::Version,
    /// The Solidity compiler output type.
    pub solc_pipeline: era_compiler_solidity::SolcPipeline,
    /// Whether to enable the EVMLA codegen via Yul IR.
    pub via_ir: bool,
    /// Whether to run the Solidity compiler optimizer.
    pub solc_optimize: bool,
    /// The optimizer settings.
    pub llvm_optimizer_settings: era_compiler_llvm_context::OptimizerSettings,
    /// Whether the EraVM extensions are enabled.
    pub enable_eravm_extensions: bool,
    /// The system contract mode.
    pub is_system_contracts_mode: bool,
}

impl Mode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        solc_version: semver::Version,
        solc_pipeline: era_compiler_solidity::SolcPipeline,
        via_ir: bool,
        solc_optimize: bool,
        mut llvm_optimizer_settings: era_compiler_llvm_context::OptimizerSettings,
        enable_eravm_extensions: bool,
        is_system_contracts_mode: bool,
    ) -> Self {
        let llvm_options = LLVMOptions::get();
        llvm_optimizer_settings.enable_fallback_to_size();
        llvm_optimizer_settings.is_verify_each_enabled = llvm_options.is_verify_each_enabled();
        llvm_optimizer_settings.is_debug_logging_enabled = llvm_options.is_debug_logging_enabled();

        Self {
            solc_version,
            solc_pipeline,
            via_ir,
            solc_optimize,
            llvm_optimizer_settings,
            enable_eravm_extensions,
            is_system_contracts_mode,
        }
    }

    ///
    /// Unwrap mode.
    ///
    /// # Panics
    ///
    /// Will panic if the inner is non-Solidity mode.
    ///
    pub fn unwrap(mode: &ModeWrapper) -> &Self {
        match mode {
            ModeWrapper::Solidity(mode) => mode,
            _ => panic!("Non-Solidity mode"),
        }
    }

    ///
    /// Checks if the mode is compatible with the source code pragmas.
    ///
    pub fn check_pragmas(&self, sources: &[(String, String)]) -> bool {
        sources.iter().all(|(_, source_code)| {
            match source_code.lines().find_map(|line| {
                let mut split = line.split_whitespace();
                if let (Some("pragma"), Some("solidity")) = (split.next(), split.next()) {
                    let version = split.join(",").replace(';', "");
                    semver::VersionReq::parse(version.as_str()).ok()
                } else {
                    None
                }
            }) {
                Some(pragma_version_req) => pragma_version_req.matches(&self.solc_version),
                None => true,
            }
        })
    }

    ///
    /// Checks if the mode is compatible with the Ethereum tests params.
    ///
    pub fn check_ethereum_tests_params(&self, params: &solidity_adapter::Params) -> bool {
        if !params.evm_version.matches_any(&[
            solidity_adapter::EVM::TangerineWhistle,
            solidity_adapter::EVM::SpuriousDragon,
            solidity_adapter::EVM::Byzantium,
            solidity_adapter::EVM::Constantinople,
            solidity_adapter::EVM::Petersburg,
            solidity_adapter::EVM::Istanbul,
            solidity_adapter::EVM::Berlin,
            solidity_adapter::EVM::London,
            solidity_adapter::EVM::Paris,
            solidity_adapter::EVM::Shanghai,
            solidity_adapter::EVM::Cancun,
        ]) {
            return false;
        }

        match self.solc_pipeline {
            era_compiler_solidity::SolcPipeline::Yul => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::False
                    && params.abi_encoder_v1_only != solidity_adapter::ABIEncoderV1Only::True
            }
            era_compiler_solidity::SolcPipeline::EVMLA if self.via_ir => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::False
                    && params.abi_encoder_v1_only != solidity_adapter::ABIEncoderV1Only::True
            }
            era_compiler_solidity::SolcPipeline::EVMLA => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::True
            }
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{} {}",
            match self.solc_pipeline {
                era_compiler_solidity::SolcPipeline::Yul => "Y",
                era_compiler_solidity::SolcPipeline::EVMLA if self.via_ir => "I",
                era_compiler_solidity::SolcPipeline::EVMLA => "E",
            },
            if self.solc_optimize { '+' } else { '-' },
            self.llvm_optimizer_settings,
            self.solc_version,
        )
    }
}
