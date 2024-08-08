//!
//! The EraVM system contracts.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use colored::Colorize;

use crate::compilers::mode::Mode;
use crate::compilers::solidity::mode::Mode as SolidityMode;
use crate::compilers::solidity::SolidityCompiler;
use crate::compilers::yul::mode::Mode as YulMode;
use crate::compilers::yul::YulCompiler;
use crate::compilers::Compiler;

pub const ADDRESS_EVM_GAS_MANAGER: u16 = 0x8013;

///
/// The EraVM system contracts.
///
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SystemContracts {
    /// The deployed system contracts builds.
    pub deployed_contracts: Vec<(web3::types::Address, era_compiler_llvm_context::EraVMBuild)>,
    /// The default account abstraction contract build.
    pub default_aa: era_compiler_llvm_context::EraVMBuild,
    /// The EVM interpreter contract build.
    pub evm_interpreter: era_compiler_llvm_context::EraVMBuild,
}

impl SystemContracts {
    /// The empty contract implementation path.
    const PATH_EMPTY_CONTRACT: &'static str =
        "era-contracts/system-contracts/contracts/EmptyContract.sol:EmptyContract";

    /// The default account abstraction contract implementation path.
    const PATH_DEFAULT_AA: &'static str =
        "era-contracts/system-contracts/contracts/DefaultAccount.sol:DefaultAccount";

    /// The EVM interpreter system contract implementation path.
    const PATH_EVM_INTERPRETER: &'static str =
        "era-contracts/system-contracts/contracts/EvmInterpreterPreprocessed.yul";

    /// The `keccak256` system contract implementation path.
    const PATH_KECCAK256: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/Keccak256.yul";

    /// The `ecrecover` system contract implementation path.
    const PATH_ECRECOVER: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/Ecrecover.yul";

    /// The `sha256` system contract implementation path.
    const PATH_SHA256: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/SHA256.yul";

    /// The `ecadd` system contract implementation path.
    const PATH_ECADD: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/EcAdd.yul";

    /// The `ecmul` system contract implementation path.
    const PATH_ECMUL: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/EcMul.yul";

    /// The account code storage system contract implementation path.
    const PATH_ACCOUNT_CODE_STORAGE: &'static str =
        "era-contracts/system-contracts/contracts/AccountCodeStorage.sol:AccountCodeStorage";

    /// The contract deployer system contract implementation path.
    const PATH_CONTRACT_DEPLOYER: &'static str =
        "era-contracts/system-contracts/contracts/ContractDeployer.sol:ContractDeployer";

    /// The nonce holder system contract implementation path.
    const PATH_NONCE_HOLDER: &'static str =
        "era-contracts/system-contracts/contracts/NonceHolder.sol:NonceHolder";

    /// The knows codes storage system contract implementation path.
    const PATH_KNOWN_CODES_STORAGE: &'static str =
        "era-contracts/system-contracts/contracts/KnownCodesStorage.sol:KnownCodesStorage";

    /// The immutable simulator system contract implementation path.
    const PATH_IMMUTABLE_SIMULATOR: &'static str =
        "era-contracts/system-contracts/contracts/ImmutableSimulator.sol:ImmutableSimulator";

    /// The L1-messenger system contract implementation path.
    const PATH_L1_MESSENGER: &'static str =
        "era-contracts/system-contracts/contracts/L1Messenger.sol:L1Messenger";

    /// The `msg.value` simulator system contract implementation path.
    const PATH_MSG_VALUE_SIMULATOR: &'static str =
        "era-contracts/system-contracts/contracts/MsgValueSimulator.sol:MsgValueSimulator";

    /// The system context system contract implementation path.
    const PATH_SYSTEM_CONTEXT: &'static str =
        "era-contracts/system-contracts/contracts/SystemContext.sol:SystemContext";

    /// The event writer system contract implementation path.
    const PATH_EVENT_WRITER: &'static str =
        "era-contracts/system-contracts/contracts/EventWriter.yul";

    /// The code oracle system contract implementation path.
    const PATH_CODE_ORACLE: &'static str =
        "era-contracts/system-contracts/contracts/precompiles/CodeOracle.yul";

    /// The base token system contract implementation path.
    const PATH_BASE_TOKEN: &'static str =
        "era-contracts/system-contracts/contracts/L2EthToken.sol:L2EthToken";

    /// The EVM gas manager system contract implementation path.
    const PATH_EVM_GAS_MANAGER: &'static str =
        "era-contracts/system-contracts/contracts/EvmGasManager.sol:EvmGasManager";

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
                Self::PATH_KECCAK256,
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_ECRECOVER.into()),
                Self::PATH_ECRECOVER,
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_SHA256.into()),
                Self::PATH_SHA256,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::system_params::ADDRESS_ECADD.into(),
                ),
                Self::PATH_ECADD,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::system_params::ADDRESS_ECMUL.into(),
                ),
                Self::PATH_ECMUL,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_EVENT_WRITER.into(),
                ),
                Self::PATH_EVENT_WRITER,
            ),
            (
                web3::types::Address::from_low_u64_be(0x8012),
                Self::PATH_CODE_ORACLE,
            ),
        ];

        let solidity_system_contracts = vec![
            (web3::types::Address::zero(), Self::PATH_EMPTY_CONTRACT),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_ACCOUNT_CODE_STORAGE.into(),
                ),
                Self::PATH_ACCOUNT_CODE_STORAGE,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_NONCE_HOLDER.into(),
                ),
                Self::PATH_NONCE_HOLDER,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_KNOWN_CODES_STORAGE.into(),
                ),
                Self::PATH_KNOWN_CODES_STORAGE,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_IMMUTABLE_SIMULATOR.into(),
                ),
                Self::PATH_IMMUTABLE_SIMULATOR,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_CONTRACT_DEPLOYER.into(),
                ),
                Self::PATH_CONTRACT_DEPLOYER,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_L1_MESSENGER.into(),
                ),
                Self::PATH_L1_MESSENGER,
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_MSG_VALUE.into()),
                Self::PATH_MSG_VALUE_SIMULATOR,
            ),
            (
                web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_SYSTEM_CONTEXT.into(),
                ),
                Self::PATH_SYSTEM_CONTEXT,
            ),
            (
                web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_ETH_TOKEN.into()),
                Self::PATH_BASE_TOKEN,
            ),
            (
                web3::types::Address::from_low_u64_be(ADDRESS_EVM_GAS_MANAGER.into()),
                Self::PATH_EVM_GAS_MANAGER,
            ),
        ];

        let mut yul_file_paths = Vec::with_capacity(yul_system_contracts.len() + 1);
        for (_, path) in yul_system_contracts.into_iter() {
            yul_file_paths.push(path.to_owned());
        }
        yul_file_paths.push(Self::PATH_EVM_INTERPRETER.to_owned());
        let yul_optimizer_settings = era_compiler_llvm_context::OptimizerSettings::cycles();
        let yul_mode = YulMode::new(yul_optimizer_settings, true).into();
        let yul_llvm_options = vec!["-eravm-jump-table-density-threshold", "10"]
            .into_iter()
            .map(|option| option.to_owned())
            .collect();
        let mut builds = Self::compile(
            YulCompiler,
            yul_file_paths,
            &yul_mode,
            yul_llvm_options,
            debug_config.clone(),
        )?;

        let mut solidity_file_paths = Vec::with_capacity(solidity_system_contracts.len() + 1);
        for pattern in [
            "era-contracts/system-contracts/contracts/*.sol",
            "era-contracts/system-contracts/contracts/libraries/**/*.sol",
            "era-contracts/system-contracts/contracts/interfaces/**/*.sol",
            "era-contracts/system-contracts/contracts/openzeppelin/**/*.sol",
            "tests/solidity/complex/interpreter/*.sol",
        ] {
            for path in glob::glob(pattern)?.filter_map(Result::ok) {
                let path = path.to_string_lossy().to_string();
                if !solidity_file_paths.contains(&path) {
                    solidity_file_paths.push(path);
                }
            }
        }

        let solidity_optimizer_settings = era_compiler_llvm_context::OptimizerSettings::cycles();
        let solidity_mode = SolidityMode::new(
            solc_version,
            era_compiler_solidity::SolcPipeline::Yul,
            true,
            true,
            solidity_optimizer_settings,
            true,
            true,
        )
        .into();
        builds.extend(Self::compile(
            SolidityCompiler::new(),
            solidity_file_paths,
            &solidity_mode,
            vec![],
            debug_config,
        )?);

        let default_aa = builds.remove(Self::PATH_DEFAULT_AA).ok_or_else(|| {
            anyhow::anyhow!("The default AA code not found in the compiler build artifacts")
        })?;
        let evm_interpreter = builds.remove(Self::PATH_EVM_INTERPRETER).ok_or_else(|| {
            anyhow::anyhow!("The EVM interpreter code not found in the compiler build artifacts")
        })?;

        let mut system_contracts =
            Vec::with_capacity(solidity_system_contracts.len() + yul_system_contracts.len());
        system_contracts.extend(solidity_system_contracts);
        system_contracts.extend(yul_system_contracts);

        let mut deployed_contracts = Vec::with_capacity(system_contracts.len());
        for (address, path) in system_contracts.into_iter() {
            let build = builds
                .remove(path)
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
            evm_interpreter,
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
                    "System contract {:?} deserialization: {}",
                    system_contracts_path,
                    error
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
            anyhow::anyhow!(
                "System contracts {:?} serialization: {}",
                system_contracts_path,
                error
            )
        })?;

        println!(
            "       {} the System Contracts to `{}`",
            "Saved".bright_green().bold(),
            system_contracts_path.to_string_lossy()
        );
        Ok(())
    }

    ///
    /// Compiles the system contracts.
    ///
    fn compile<C>(
        compiler: C,
        paths: Vec<String>,
        mode: &Mode,
        llvm_options: Vec<String>,
        debug_config: Option<era_compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<HashMap<String, era_compiler_llvm_context::EraVMBuild>>
    where
        C: Compiler,
    {
        let mut sources = Vec::new();
        for path in paths.into_iter() {
            let file_path = if compiler.allows_multi_contract_files() {
                path.split(':').next().expect("Always valid").to_string()
            } else {
                path
            };

            let mut source = std::fs::read_to_string(
                PathBuf::from_str(file_path.as_str())
                    .expect("Always valid")
                    .as_path(),
            )
            .map_err(|error| {
                anyhow::anyhow!(
                    "System contract file `{}` reading error: {}",
                    file_path,
                    error
                )
            })?;

            if file_path == "era-contracts/system-contracts/contracts/Constants.sol" {
                source = source.replace("{{SYSTEM_CONTRACTS_OFFSET}}", "0x8000");
            }

            sources.push((file_path.to_string(), source));
        }

        compiler
            .compile_for_eravm(
                "system-contracts".to_owned(),
                sources,
                BTreeMap::new(),
                mode,
                llvm_options,
                debug_config,
            )
            .map(|output| output.builds)
            .map_err(|error| anyhow::anyhow!("Failed to compile system contracts: {}", error))
    }
}
