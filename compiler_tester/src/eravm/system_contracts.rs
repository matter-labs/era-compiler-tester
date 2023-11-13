//!
//! The system contracts.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use colored::Colorize;
use serde::Deserialize;
use serde::Serialize;

use crate::compilers::mode::solidity::Mode as SolidityMode;
use crate::compilers::mode::yul::Mode as YulMode;
use crate::compilers::mode::Mode;
use crate::compilers::output::build::Build as EraVMContractBuild;
use crate::compilers::solidity::SolidityCompiler;
use crate::compilers::yul::YulCompiler;
use crate::compilers::Compiler;

///
/// The system contracts.
///
#[derive(Serialize, Deserialize)]
pub struct SystemContracts {
    /// The deployed system contracts builds.
    pub deployed_contracts: Vec<(web3::types::Address, EraVMContractBuild)>,
    /// The default account abstraction contract build.
    pub default_aa: EraVMContractBuild,
}

impl SystemContracts {
    /// The empty contract implementation path.
    const PATH_EMPTY_CONTRACT: &'static str =
        "system-contracts/contracts/EmptyContract.sol:EmptyContract";

    /// The `keccak256` system contract implementation path.
    const PATH_KECCAK256: &'static str = "system-contracts/contracts/precompiles/Keccak256.yul";

    /// The `ecrecover` system contract implementation path.
    const PATH_ECRECOVER: &'static str = "system-contracts/contracts/precompiles/Ecrecover.yul";

    /// The `sha256` system contract implementation path.
    const PATH_SHA256: &'static str = "system-contracts/contracts/precompiles/SHA256.yul";

    /// The account code storage system contract implementation path.
    const PATH_ACCOUNT_CODE_STORAGE: &'static str =
        "system-contracts/contracts/AccountCodeStorage.sol:AccountCodeStorage";

    /// The contract deployer system contract implementation path.
    const PATH_CONTRACT_DEPLOYER: &'static str =
        "system-contracts/contracts/ContractDeployer.sol:ContractDeployer";

    /// The nonce holder system contract implementation path.
    const PATH_NONCE_HOLDER: &'static str =
        "system-contracts/contracts/NonceHolder.sol:NonceHolder";

    /// The knows codes storage system contract implementation path.
    const PATH_KNOWN_CODES_STORAGE: &'static str =
        "system-contracts/contracts/KnownCodesStorage.sol:KnownCodesStorage";

    /// The immutable simulator system contract implementation path.
    const PATH_IMMUTABLE_SIMULATOR: &'static str =
        "system-contracts/contracts/ImmutableSimulator.sol:ImmutableSimulator";

    /// The L1-messenger system contract implementation path.
    const PATH_L1_MESSENGER: &'static str =
        "system-contracts/contracts/L1Messenger.sol:L1Messenger";

    /// The `msg.value` simulator system contract implementation path.
    const PATH_MSG_VALUE_SIMULATOR: &'static str =
        "system-contracts/contracts/MsgValueSimulator.sol:MsgValueSimulator";

    /// The system context system contract implementation path.
    const PATH_SYSTEM_CONTEXT: &'static str =
        "system-contracts/contracts/SystemContext.sol:SystemContext";

    /// The event writer system contract implementation path.
    const PATH_EVENT_WRITER: &'static str = "system-contracts/contracts/EventWriter.yul";

    /// The ETH token system contract implementation path.
    const PATH_ETH_TOKEN: &'static str = "system-contracts/contracts/L2EthToken.sol:L2EthToken";

    /// The default account abstraction contract implementation path.
    const PATH_DEFAULT_AA: &'static str =
        "system-contracts/contracts/DefaultAccount.sol:DefaultAccount";

    ///
    /// Load or build the system contracts.
    ///
    pub fn load_or_build(
        solc_version: semver::Version,
        system_contracts_debug_config: Option<compiler_llvm_context::DebugConfig>,
        system_contracts_path: Option<PathBuf>,
        system_contracts_save_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let system_contracts = if let Some(system_contracts_path) = system_contracts_path {
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
        debug_config: Option<compiler_llvm_context::DebugConfig>,
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
                    zkevm_opcode_defs::ADDRESS_EVENT_WRITER.into(),
                ),
                Self::PATH_EVENT_WRITER,
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
                Self::PATH_ETH_TOKEN,
            ),
        ];

        let mut yul_file_paths = Vec::with_capacity(yul_system_contracts.len() + 1);
        for (_, path) in yul_system_contracts.iter() {
            let file_path = path.split(':').next().expect("Always valid");
            yul_file_paths.push(file_path.to_owned());
        }
        let yul_mode = YulMode::new(compiler_llvm_context::OptimizerSettings::cycles()).into();
        let mut builds = Self::compile(
            YulCompiler::new(),
            &yul_mode,
            yul_file_paths,
            debug_config.clone(),
        )?;

        let mut solidity_file_paths = Vec::with_capacity(solidity_system_contracts.len() + 1);
        for (_, path) in solidity_system_contracts.iter() {
            let file_path = path.split(':').next().expect("Always valid");
            solidity_file_paths.push(file_path.to_owned());
        }
        for path in glob::glob("system-contracts/**/*.sol")?.filter_map(Result::ok) {
            let path = path.to_string_lossy().to_string();
            if !solidity_file_paths.contains(&path) {
                solidity_file_paths.push(path);
            }
        }
        let solidity_mode = SolidityMode::new(
            solc_version,
            compiler_solidity::SolcPipeline::Yul,
            true,
            true,
            compiler_llvm_context::OptimizerSettings::cycles(),
        )
        .into();
        builds.extend(Self::compile(
            SolidityCompiler::new(),
            &solidity_mode,
            solidity_file_paths,
            debug_config,
        )?);

        let mut system_contracts =
            Vec::with_capacity(solidity_system_contracts.len() + yul_system_contracts.len());
        system_contracts.extend(solidity_system_contracts);
        system_contracts.extend(yul_system_contracts);

        let mut deployed_contracts = Vec::with_capacity(system_contracts.len());
        for (address, path) in system_contracts.into_iter() {
            let build = builds.remove(path).unwrap_or_else(|| {
                panic!("System contract source file `{path}` not found in the builds")
            });
            deployed_contracts.push((address, build));
        }

        let default_aa = builds.remove(Self::PATH_DEFAULT_AA).ok_or_else(|| {
            anyhow::anyhow!("Default account code not found in the compiler build artifacts")
        })?;

        println!(
            "    {} building system contracts in {}.{:03}s",
            "Finished".bright_green().bold(),
            build_time_start.elapsed().as_secs(),
            build_time_start.elapsed().subsec_millis(),
        );

        Ok(Self {
            deployed_contracts,
            default_aa,
        })
    }

    ///
    /// Load the system contracts build from the given file.
    ///
    fn load(system_contracts_path: PathBuf) -> anyhow::Result<Self> {
        let system_contracts_file = File::open(system_contracts_path.as_path())?;
        let system_contracts: SystemContracts = bincode::deserialize_from(system_contracts_file)
            .map_err(|error| anyhow::anyhow!("System contract deserialization: {}", error))?;
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
        bincode::serialize_into(system_contracts_file, self)
            .map_err(|error| anyhow::anyhow!("System contracts serialization: {}", error,))?;
        println!(
            "       {} the System Contracts to `{}`",
            "Saved".bright_green().bold(),
            system_contracts_path.to_string_lossy()
        );
        Ok(())
    }

    ///
    /// Compiles the system contracts with a compiler.
    ///
    fn compile<C>(
        compiler: C,
        mode: &Mode,
        paths: Vec<String>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> anyhow::Result<HashMap<String, EraVMContractBuild>>
    where
        C: Compiler,
    {
        let mut sources = Vec::new();
        for path in paths.into_iter() {
            let file_path = if compiler.has_many_contracts() {
                path.split(':').next().expect("Always valid").to_string()
            } else {
                path
            };
            let source = std::fs::read_to_string(
                PathBuf::from_str(file_path.as_str())
                    .expect("Always valid")
                    .as_path(),
            )
            .map_err(|error| {
                anyhow::anyhow!("System contract file `{}` reading: {}", file_path, error)
            })?;
            sources.push((file_path.to_string(), source));
        }
        compiler
            .compile(
                "system-contracts".to_owned(),
                sources,
                BTreeMap::new(),
                mode,
                true,
                true,
                debug_config,
            )
            .map(|output| output.builds)
            .map_err(|error| anyhow::anyhow!("Failed to compile system contracts: {}", error))
    }
}
