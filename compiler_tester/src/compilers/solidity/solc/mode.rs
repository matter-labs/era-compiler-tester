//!
//! The compiler tester Solidity mode.
//!

use itertools::Itertools;

use crate::compilers::mode::{imode::IMode, Mode as ModeWrapper};

///
/// The compiler tester Solidity mode.
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Mode {
    /// The Solidity compiler version.
    pub solc_version: semver::Version,
    /// The Solidity compiler output type.
    pub solc_codegen: era_solc::StandardJsonInputCodegen,
    /// Whether to enable the EVMLA codegen via Yul IR.
    pub via_ir: bool,
    /// Whether to enable the MLIR codegen.
    pub via_mlir: bool,
    /// Whether to run the Solidity compiler optimizer.
    pub solc_optimize: bool,
}

impl Mode {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        solc_version: semver::Version,
        solc_codegen: era_solc::StandardJsonInputCodegen,
        via_ir: bool,
        via_mlir: bool,
        solc_optimize: bool,
    ) -> Self {
        Self {
            solc_version,
            solc_codegen,
            via_ir,
            via_mlir,
            solc_optimize,
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
            ModeWrapper::Solc(mode) => mode,
            _ => panic!("Non-Solidity-upstream mode"),
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

        match self.solc_codegen {
            era_solc::StandardJsonInputCodegen::Yul => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::False
                    && params.abi_encoder_v1_only != solidity_adapter::ABIEncoderV1Only::True
            }
            era_solc::StandardJsonInputCodegen::EVMLA if self.via_ir => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::False
                    && params.abi_encoder_v1_only != solidity_adapter::ABIEncoderV1Only::True
            }
            era_solc::StandardJsonInputCodegen::EVMLA => {
                params.compile_via_yul != solidity_adapter::CompileViaYul::True
            }
        }
    }
}

impl IMode for Mode {
    fn optimizations(&self) -> Option<String> {
        Some((if self.solc_optimize { "+" } else { "-" }).to_string())
    }

    fn codegen(&self) -> Option<String> {
        Some(
            (if self.via_mlir {
                "L"
            } else {
                match self.solc_codegen {
                    era_solc::StandardJsonInputCodegen::Yul => "Y",
                    era_solc::StandardJsonInputCodegen::EVMLA if self.via_ir => "I",
                    era_solc::StandardJsonInputCodegen::EVMLA => "E",
                }
            })
            .to_string(),
        )
    }

    fn version(&self) -> Option<String> {
        Some(self.solc_version.to_string())
    }
}
