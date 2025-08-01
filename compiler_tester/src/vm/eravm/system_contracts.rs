//!
//! The EraVM system contracts.
//!

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::time::Instant;

use colored::Colorize;

use crate::compilers::mode::Mode;
use crate::compilers::solidity::zksolc::mode::Mode as ZksolcMode;
use crate::compilers::solidity::zksolc::SolidityCompiler as ZksolcCompiler;
use crate::compilers::yul::mode::Mode as YulMode;
use crate::compilers::yul::YulCompiler;
use crate::compilers::Compiler;

/// The EVMGasManager system contract address.
pub const ADDRESS_EVM_GAS_MANAGER: u16 = 0x8013;

/// The EVMHashesStorage system contract address.
pub const ADDRESS_EVM_HASHES_STORAGE: u16 = 0x8015;

///
/// The EraVM system contracts.
///
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SystemContracts {
    /// The deployed system contracts builds.
    pub deployed_contracts: Vec<(web3::types::Address, era_compiler_llvm_context::EraVMBuild)>,
    /// The default account abstraction contract build.
    pub default_aa: era_compiler_llvm_context::EraVMBuild,
    /// The EVM emulator contract build.
    pub evm_emulator: era_compiler_llvm_context::EraVMBuild,
}

impl SystemContracts {
    /// The empty contract implementation path.
    const PATH_EMPTY_CONTRACT: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/EmptyContract.sol",
        "EmptyContract",
    );

    /// The default account abstraction contract implementation path.
    const PATH_DEFAULT_AA: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/DefaultAccount.sol",
        "DefaultAccount",
    );

    /// The EVM emulator system contract implementation path.
    const PATH_EVM_EMULATOR: &'static str =
        "era-contracts/system-contracts/contracts/EvmEmulator.yul";

    /// The `keccak256` system contract implementation path.
    const PATH_KECCAK256: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/Keccak256.yul";

    /// The `ecrecover` system contract implementation path.
    const PATH_ECRECOVER: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/Ecrecover.yul";

    /// The `sha256` system contract implementation path.
    const PATH_SHA256: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/SHA256.yul";

    /// The `identity` system contract implementation path.
    const PATH_IDENTITY: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/Identity.yul";

    /// The `ecadd` system contract implementation path.
    const PATH_ECADD: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/EcAdd.yul";

    /// The `ecmul` system contract implementation path.
    const PATH_ECMUL: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/EcMul.yul";

    /// The account code storage system contract implementation path.
    const PATH_ACCOUNT_CODE_STORAGE: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/AccountCodeStorage.sol",
        "AccountCodeStorage",
    );

    /// The contract deployer system contract implementation path.
    const PATH_CONTRACT_DEPLOYER: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/ContractDeployer.sol",
        "ContractDeployer",
    );

    /// The nonce holder system contract implementation path.
    const PATH_NONCE_HOLDER: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/NonceHolder.sol",
        "NonceHolder",
    );

    /// The knows codes storage system contract implementation path.
    const PATH_KNOWN_CODES_STORAGE: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/KnownCodesStorage.sol",
        "KnownCodesStorage",
    );

    /// The immutable simulator system contract implementation path.
    const PATH_IMMUTABLE_SIMULATOR: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/ImmutableSimulator.sol",
        "ImmutableSimulator",
    );

    /// The L1-messenger system contract implementation path.
    const PATH_L1_MESSENGER: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/L1Messenger.sol",
        "L1Messenger",
    );

    /// The `msg.value` simulator system contract implementation path.
    const PATH_MSG_VALUE_SIMULATOR: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/MsgValueSimulator.sol",
        "MsgValueSimulator",
    );

    /// The system context system contract implementation path.
    const PATH_SYSTEM_CONTEXT: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/SystemContext.sol",
        "SystemContext",
    );

    /// The event writer system contract implementation path.
    const PATH_EVENT_WRITER: &'static str =
        "era-contracts/system-contracts/contracts/EventWriter.yul";

    /// The code oracle system contract implementation path.
    const PATH_CODE_ORACLE: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/CodeOracle.yul";

    /// The base token system contract implementation path.
    const PATH_BASE_TOKEN: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/L2BaseToken.sol",
        "L2BaseToken",
    );

    /// The EVM gas manager system contract implementation path.
    const PATH_EVM_GAS_MANAGER: &'static str =
        "era-contracts/system-contracts/contracts/EvmGasManager.yul";

    /// The EVM hashes storage system contract implementation path.
    const PATH_EVM_HASHES_STORAGE: (&'static str, &'static str) = (
        "era-contracts/system-contracts/contracts/EvmHashesStorage.sol",
        "EvmHashesStorage",
    );

    ///
    /// Loads or builds the system contracts.
    ///
    pub fn load_or_build(
        solc_version: semver::Version,
        system_contracts_debug_config: Option<era_compiler_llvm_context::DebugConfig>,
        system_contracts_load_path: Option<PathBuf>,
        system_contracts_save_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let system_contracts = if let Some(system_contracts_path) = system_contracts_load_path {
            Self::load(system_contracts_path)
                .map_err(|error| anyhow::anyhow!("System contracts loading: {}", error))?
        } else {
            Self::build(solc_version, system_contracts_debug_config)
                .map_err(|error| anyhow::anyhow!("System contracts building: {}", error))?
        };

        if let Some(system_contracts_save_path) = system_contracts_save_path {
            system_contracts
                .save(system_contracts_save_path)
                .map_err(|error| anyhow::anyhow!("System contracts saving: {}", error))?;
        }

        Ok(system_contracts)
    }

    ///
    /// Builds the system contracts.
    ///
    fn build(
        solc_version: semver::Version,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<Self> {
        let build_time_start = Instant::now();
        println!("    {} system contracts", "Building".bright_green().bold());

        let yul_system_contracts = [
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_KECCAK256.into()),
                Self::PATH_KECCAK256.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_ECRECOVER.into()),
                Self::PATH_ECRECOVER.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_SHA256.into()),
                Self::PATH_SHA256.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_IDENTITY.into()),
                Self::PATH_IDENTITY.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::system_params::ADDRESS_ECADD.into(),
                ),
                Self::PATH_ECADD.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::system_params::ADDRESS_ECMUL.into(),
                ),
                Self::PATH_ECMUL.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_EVENT_WRITER.into(),
                ),
                Self::PATH_EVENT_WRITER.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(0x8012),
                Self::PATH_CODE_ORACLE.to_owned(),
            ),
            (
                web3::types::Address::from_low_u64_be(ADDRESS_EVM_GAS_MANAGER.into()),
                Self::PATH_EVM_GAS_MANAGER.to_owned(),
            ),
        ];

        let solidity_system_contracts = vec![
            (
                web3::types::Address::zero(),
                Self::normalize_name_fs(
                    Self::PATH_EMPTY_CONTRACT.0,
                    Some(Self::PATH_EMPTY_CONTRACT.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_ACCOUNT_CODE_STORAGE.into(),
                ),
                Self::normalize_name_fs(
                    Self::PATH_ACCOUNT_CODE_STORAGE.0,
                    Some(Self::PATH_ACCOUNT_CODE_STORAGE.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_NONCE_HOLDER.into(),
                ),
                Self::normalize_name_fs(Self::PATH_NONCE_HOLDER.0, Some(Self::PATH_NONCE_HOLDER.1)),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_KNOWN_CODES_STORAGE.into(),
                ),
                Self::normalize_name_fs(
                    Self::PATH_KNOWN_CODES_STORAGE.0,
                    Some(Self::PATH_KNOWN_CODES_STORAGE.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_IMMUTABLE_SIMULATOR.into(),
                ),
                Self::normalize_name_fs(
                    Self::PATH_IMMUTABLE_SIMULATOR.0,
                    Some(Self::PATH_IMMUTABLE_SIMULATOR.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER.into(),
                ),
                Self::normalize_name_fs(
                    Self::PATH_CONTRACT_DEPLOYER.0,
                    Some(Self::PATH_CONTRACT_DEPLOYER.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_L1_MESSENGER.into(),
                ),
                Self::normalize_name_fs(Self::PATH_L1_MESSENGER.0, Some(Self::PATH_L1_MESSENGER.1)),
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_MSG_VALUE.into()),
                Self::normalize_name_fs(
                    Self::PATH_MSG_VALUE_SIMULATOR.0,
                    Some(Self::PATH_MSG_VALUE_SIMULATOR.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_SYSTEM_CONTEXT.into(),
                ),
                Self::normalize_name_fs(
                    Self::PATH_SYSTEM_CONTEXT.0,
                    Some(Self::PATH_SYSTEM_CONTEXT.1),
                ),
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_ETH_TOKEN.into()),
                Self::normalize_name_fs(Self::PATH_BASE_TOKEN.0, Some(Self::PATH_BASE_TOKEN.1)),
            ),
            (
                web3::types::Address::from_low_u64_be(ADDRESS_EVM_HASHES_STORAGE.into()),
                Self::normalize_name_fs(
                    Self::PATH_EVM_HASHES_STORAGE.0,
                    Some(Self::PATH_EVM_HASHES_STORAGE.1),
                ),
            ),
        ];

        let mut yul_file_paths = Vec::with_capacity(yul_system_contracts.len() + 1);
        for (_, path) in yul_system_contracts.iter() {
            yul_file_paths.push(PathBuf::from(path));
        }
        yul_file_paths.push(PathBuf::from(Self::PATH_EVM_EMULATOR));
        let yul_optimizer_settings = era_compiler_llvm_context::OptimizerSettings::cycles();
        let yul_mode = YulMode::new(yul_optimizer_settings, true).into();
        let yul_llvm_options = vec![
            "-eravm-jump-table-density-threshold",
            "10",
            "-tail-dup-size",
            "6",
            "-eravm-enable-split-loop-phi-live-ranges",
            "-tail-merge-only-bbs-without-succ",
            "-tail-dup-fallthrough-bbs",
        ]
        .into_iter()
        .map(|option| option.to_owned())
        .collect();
        let mut builds = Self::compile(
            YulCompiler::Zksolc,
            yul_file_paths,
            &yul_mode,
            yul_llvm_options,
            debug_config.clone(),
        )?;

        let mut solidity_file_paths = Vec::with_capacity(solidity_system_contracts.len() + 2);
        for pattern in [
            "tests/solidity/complex/interpreter/*.sol",
            "era-contracts/system-contracts/contracts/*.sol",
            "era-contracts/system-contracts/contracts/libraries/**/*.sol",
            "era-contracts/system-contracts/contracts/interfaces/**/*.sol",
            "era-contracts/system-contracts/contracts/openzeppelin/**/*.sol",
            "era-contracts/system-contracts/lib/openzeppelin-contracts-v4/contracts/**/*.sol",
        ]
        .into_iter()
        .map(PathBuf::from)
        {
            for path in glob::glob(pattern.to_str().expect("Always valid"))?.filter_map(Result::ok)
            {
                if !solidity_file_paths.contains(&path) {
                    solidity_file_paths.push(path);
                }
            }
        }

        let solidity_optimizer_settings = era_compiler_llvm_context::OptimizerSettings::cycles();
        let solidity_mode = ZksolcMode::new(
            solc_version,
            era_solc::StandardJsonInputCodegen::Yul,
            true,
            solidity_optimizer_settings,
            true,
            true,
        )
        .into();
        builds.extend(Self::compile(
            ZksolcCompiler::new(),
            solidity_file_paths,
            &solidity_mode,
            vec![],
            debug_config,
        )?);

        let default_aa = builds
            .remove(
                Self::normalize_name_solc(Self::PATH_DEFAULT_AA.0, Some(Self::PATH_DEFAULT_AA.1))
                    .as_str(),
            )
            .ok_or_else(|| {
                anyhow::anyhow!("The default AA code not found in the compiler build artifacts")
            })?;
        let evm_emulator = builds
            .remove(Self::normalize_name_solc(Self::PATH_EVM_EMULATOR, None).as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("The EVM emulator code not found in the compiler build artifacts")
            })?;

        let mut system_contracts =
            Vec::with_capacity(solidity_system_contracts.len() + yul_system_contracts.len());
        system_contracts.extend(solidity_system_contracts);
        system_contracts.extend(yul_system_contracts);

        let mut deployed_contracts = Vec::with_capacity(system_contracts.len());
        for (address, path) in system_contracts.into_iter() {
            let build = builds
                .remove(Self::normalize_name_solc(path.as_str(), None).as_str())
                .unwrap_or_else(|| panic!("System contract `{path}` not found in the builds"));
            deployed_contracts.push((address, build));
        }

        println!(
            "    {} building system contracts in {}.{:03}s",
            "Finished".bright_green().bold(),
            build_time_start.elapsed().as_secs(),
            build_time_start.elapsed().subsec_millis(),
        );

        Ok(Self {
            deployed_contracts,
            default_aa,
            evm_emulator,
        })
    }

    ///
    /// Load the system contracts build from the given file.
    ///
    fn load(system_contracts_path: PathBuf) -> anyhow::Result<Self> {
        let system_contracts_file = File::open(system_contracts_path.as_path())?;
        let system_contracts: SystemContracts = bincode::deserialize_from(system_contracts_file)
            .map_err(|error| {
                anyhow::anyhow!(
                    "System contract {system_contracts_path:?} deserialization: {error}"
                )
            })?;
        println!(
            "      {} the System Contracts from `{}`",
            "Loaded".bright_green().bold(),
            system_contracts_path.to_string_lossy()
        );
        Ok(system_contracts)
    }

    ///
    /// Save the system contracts build to the given file.
    ///
    fn save(&self, system_contracts_path: PathBuf) -> anyhow::Result<()> {
        let system_contracts_file = File::create(system_contracts_path.as_path())?;
        bincode::serialize_into(system_contracts_file, self).map_err(|error| {
            anyhow::anyhow!("System contracts {system_contracts_path:?} serialization: {error}")
        })?;

        println!(
            "       {} the System Contracts to `{}`",
            "Saved".bright_green().bold(),
            system_contracts_path.to_string_lossy()
        );
        Ok(())
    }

    ///
    /// Normalizes contract names with respect to the file system.
    ///
    fn normalize_name_fs(path: &str, name: Option<&str>) -> String {
        let contract_name = era_compiler_common::ContractName::new(
            path.replace("/", std::path::MAIN_SEPARATOR_STR),
            name.map(|name| name.to_string()),
        );
        contract_name.full_path
    }

    ///
    /// Normalizes contract names with respect to `solc`.
    ///
    fn normalize_name_solc(path: &str, name: Option<&str>) -> String {
        let contract_name = era_compiler_common::ContractName::new(
            path.replace(std::path::MAIN_SEPARATOR_STR, "/"),
            name.map(|name| name.to_string()),
        );
        contract_name.full_path
    }

    ///
    /// Compiles the system contracts.
    ///
    fn compile<C>(
        compiler: C,
        paths: Vec<PathBuf>,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>
    where
        C: Compiler,
    {
        let mut sources = Vec::new();
        for path in paths.into_iter() {
            let mut source = std::fs::read_to_string(path.as_path()).map_err(|error| {
                anyhow::anyhow!("System contract file {path:?} reading error: {error}",)
            })?;

            if path == PathBuf::from("era-contracts/system-contracts/contracts/Constants.sol") {
                source = source.replace("{{SYSTEM_CONTRACTS_OFFSET}}", "0x8000");
            }

            let path = crate::utils::path_to_string_normalized(path.as_path());

            if path.as_str() == "era-contracts/system-contracts/contracts/SystemContractErrors.sol"
            {
                sources.push((
                    "contracts/SystemContractErrors.sol".to_owned(),
                    source.clone(),
                ));
            }
            if path.starts_with(
                "era-contracts/system-contracts/lib/openzeppelin-contracts-v4/contracts/",
            ) {
                sources.push((
                    path.replace(
                        "era-contracts/system-contracts/lib/openzeppelin-contracts-v4/contracts/",
                        "@openzeppelin/contracts-v4/",
                    ),
                    source.clone(),
                ));
            }

            sources.push((path, source));
        }

        compiler
            .compile_for_eravm(
                "system-contracts".to_owned(),
                sources,
                era_compiler_common::Libraries::default(),
                mode,
                llvm_options,
                debug_config,
            )
            .map(|output| output.builds)
            .map_err(|error| anyhow::anyhow!("Failed to compile system contracts: {}", error))
    }
}
