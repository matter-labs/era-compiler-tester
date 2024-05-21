//!
//! The EraVM interface.
//!

pub mod address_iterator;
pub mod deployers;
pub mod input;
pub mod system_context;
pub mod system_contracts;

#[cfg(feature = "vm2")]
mod vm2_adapter;

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use colored::Colorize;

use crate::compilers::downloader::Downloader as CompilerDownloader;
use crate::vm::execution_result::ExecutionResult;

use self::system_context::SystemContext;
use self::system_contracts::SystemContracts;

///
/// The EraVM interface.
///
#[derive(Clone)]
pub struct EraVM {
    /// The known contracts.
    known_contracts: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
    /// The default account abstraction contract code hash.
    default_aa_code_hash: web3::types::U256,
    /// The EVM interpreter contract code hash.
    evm_interpreter_code_hash: web3::types::U256,
    /// The deployed contracts.
    deployed_contracts: HashMap<web3::types::Address, zkevm_assembly::Assembly>,
    /// The published EVM bytecodes
    published_evm_bytecodes: HashMap<web3::types::U256, Vec<web3::types::U256>>,
    /// The storage state.
    storage: HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>,
}

impl EraVM {
    /// The default address of the benchmark caller.
    pub const DEFAULT_BENCHMARK_CALLER_ADDRESS: &'static str =
        "eeaffc9ff130f15d470945fd04b9017779c95dbf";

    /// The extra amount of gas consumed by every call to the EVM interpreter.
    pub const EVM_INTERPRETER_GAS_OVERHEAD: u64 = 2500;

    ///
    /// Creates and initializes a new EraVM instance.
    ///
    pub fn new(
        binary_download_config_paths: Vec<PathBuf>,
        system_contracts_solc_downloader_config_path: PathBuf,
        system_contracts_debug_config: Option<era_compiler_llvm_context::DebugConfig>,
        system_contracts_load_path: Option<PathBuf>,
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
            .download(system_contracts_solc_downloader_config_path.as_path())?;
        for config_path in binary_download_config_paths.into_iter() {
            CompilerDownloader::new(http_client.clone()).download(config_path.as_path())?;
        }
        println!(
            "    {} downloading compiler binaries in {}m{:02}s",
            "Finished".bright_green().bold(),
            download_time_start.elapsed().as_secs() / 60,
            download_time_start.elapsed().as_secs() % 60,
        );

        let solc_version = system_contracts_solc_downloader_config
            .binaries
            .keys()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "EraVM initializer could find the `solc` version for system contracts"
                )
            })?;
        let solc_version = semver::Version::parse(solc_version.as_str())?;

        let system_contracts = SystemContracts::load_or_build(
            solc_version,
            system_contracts_debug_config,
            system_contracts_load_path,
            system_contracts_save_path,
        )?;

        let storage = SystemContext::create_storage();

        let mut vm = Self {
            known_contracts: HashMap::new(),
            default_aa_code_hash: system_contracts.default_aa.bytecode_hash,
            evm_interpreter_code_hash: system_contracts.evm_interpreter.bytecode_hash,
            deployed_contracts: HashMap::new(),
            storage,
            published_evm_bytecodes: HashMap::new(),
        };

        vm.add_known_contract(
            system_contracts.default_aa.assembly,
            system_contracts.default_aa.bytecode_hash,
        );
        vm.add_known_contract(
            system_contracts.evm_interpreter.assembly,
            system_contracts.evm_interpreter.bytecode_hash,
        );
        vm.add_known_contract(
            zkevm_assembly::Assembly::from_string(
                era_compiler_vyper::FORWARDER_CONTRACT_ASSEMBLY.to_owned(),
                None,
            )
            .expect("Always valid"),
            web3::types::U256::from_str_radix(
                era_compiler_vyper::FORWARDER_CONTRACT_HASH.as_str(),
                era_compiler_common::BASE_HEXADECIMAL,
            )
            .expect("Always valid"),
        );

        for (address, build) in system_contracts.deployed_contracts {
            vm.add_deployed_contract(address, build.bytecode_hash, Some(build.assembly));
        }

        Ok(vm)
    }

    ///
    /// Clones the VM instance from and adds known contracts for a single test run.
    ///
    /// TODO: make copyless when the VM supports it.
    ///
    pub fn clone_with_contracts(
        vm: Arc<Self>,
        known_contracts: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
    ) -> Self {
        let mut new_vm = (*vm).clone();
        for (bytecode_hash, assembly) in known_contracts.into_iter() {
            new_vm.add_known_contract(assembly, bytecode_hash);
        }
        new_vm
    }

    ///
    /// Runs a test transaction.
    ///
    pub fn execute<const M: bool>(
        &mut self,
        test_name: String,
        mut entry_address: web3::types::Address,
        caller: web3::types::Address,
        value: Option<u128>,
        calldata: Vec<u8>,
        vm_launch_option: Option<zkevm_tester::runners::compiler_tests::VmLaunchOption>,
    ) -> anyhow::Result<ExecutionResult> {
        let (vm_launch_option, context_u128_value) =
            if let Some(vm_launch_option) = vm_launch_option {
                (vm_launch_option, value)
            } else if M {
                match value {
                    Some(value) => {
                        self.mint_ether(caller, web3::types::U256::from(value));

                        let r3 = Some(web3::types::U256::from(value));
                        let r4 = Some(web3::types::U256::from_big_endian(entry_address.as_bytes()));
                        let r5 = Some(web3::types::U256::from(u8::from(
                            era_compiler_llvm_context::eravm_const::SYSTEM_CALL_BIT,
                        )));

                        entry_address = web3::types::Address::from_low_u64_be(
                            zkevm_opcode_defs::ADDRESS_MSG_VALUE.into(),
                        );

                        let vm_launch_option =
                            zkevm_tester::runners::compiler_tests::VmLaunchOption::ManualCallABI(
                                zkevm_tester::runners::compiler_tests::FullABIParams {
                                    is_constructor: false,
                                    is_system_call: true,
                                    r3_value: r3,
                                    r4_value: r4,
                                    r5_value: r5,
                                },
                            );
                        (vm_launch_option, None)
                    }
                    None => (
                        zkevm_tester::runners::compiler_tests::VmLaunchOption::Call,
                        None,
                    ),
                }
            } else {
                if let Some(value) = value {
                    self.mint_ether(entry_address, web3::types::U256::from(value));
                }

                (
                    zkevm_tester::runners::compiler_tests::VmLaunchOption::Call,
                    value,
                )
            };

        let mut trace_file_path = PathBuf::from_str("./trace/").expect("Always valid");
        let trace_file_name = regex::Regex::new("[^A-Za-z0-9]+")
            .expect("Always valid")
            .replace_all(test_name.as_str(), "_")
            .to_string();
        trace_file_path.push(trace_file_name);

        let context = zkevm_tester::runners::compiler_tests::VmExecutionContext::new(
            entry_address,
            caller,
            context_u128_value.unwrap_or_default(),
            0,
        );

        #[cfg(not(feature = "vm2"))]
        {
            let snapshot = zkevm_tester::runners::compiler_tests::run_vm_multi_contracts(
                trace_file_path.to_string_lossy().to_string(),
                self.deployed_contracts.clone(),
                &calldata,
                self.storage.clone(),
                entry_address,
                Some(context),
                vm_launch_option,
                usize::MAX,
                self.known_contracts.clone(),
                self.published_evm_bytecodes.clone(),
                self.default_aa_code_hash,
                self.evm_interpreter_code_hash,
            )?;

            for (address, assembly) in snapshot.deployed_contracts.iter() {
                if self.deployed_contracts.contains_key(address) {
                    continue;
                }

                self.deployed_contracts
                    .insert(*address, assembly.to_owned());
            }

            for (hash, preimage) in snapshot.published_sha256_blobs.iter() {
                if self.published_evm_bytecodes.contains_key(hash) {
                    continue;
                }

                self.published_evm_bytecodes.insert(*hash, preimage.clone());
            }

            self.storage.clone_from(&snapshot.storage);

            Ok(snapshot.into())
        }
        #[cfg(feature = "vm2")]
        {
            let (result, storage_changes, deployed_contracts) = vm2_adapter::run_vm(
                self.deployed_contracts.clone(),
                &calldata,
                self.storage.clone(),
                entry_address,
                Some(context),
                vm_launch_option,
                self.known_contracts.clone(),
                self.default_aa_code_hash,
                self.evm_interpreter_code_hash,
            )
            .map_err(|error| anyhow::anyhow!("EraVM failure: {}", error))?;

            for (key, value) in storage_changes.into_iter() {
                self.storage.insert(key, value);
            }
            for (address, assembly) in deployed_contracts.into_iter() {
                if self.deployed_contracts.contains_key(&address) {
                    continue;
                }

                self.deployed_contracts.insert(address, assembly);
            }

            Ok(result)
        }
    }

    ///
    /// Performs the check for the storage emptiness, that is, if all its values, except for those
    /// related to system contracts and auxiliary data inaccessible by the user code, are zeros.
    ///
    /// Mostly used by the Ethereum tests.
    ///
    pub fn is_storage_empty(&self) -> bool {
        for (key, value) in self.storage.iter() {
            if key.address
                < web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_UNRESTRICTED_SPACE,
                )
            {
                continue;
            }

            if !value.is_zero() {
                return false;
            }
        }

        true
    }

    ///
    /// Mints some Ether value at the specified address.
    /// Is needed for payable calls simulation.
    ///
    pub fn mint_ether(&mut self, address: web3::types::Address, amount: web3::types::U256) {
        let key = Self::balance_storage_key(address);
        let old_amount = web3::types::U256::from_big_endian(
            self.storage
                .get(&key)
                .cloned()
                .unwrap_or_default()
                .as_bytes(),
        );
        let new_amount = old_amount + amount;
        let new_amount = crate::utils::u256_to_h256(&new_amount);
        self.storage.insert(key, new_amount);
    }

    ///
    /// Burns some Ether value for at specified address.
    ///
    pub fn burn_ether(&mut self, address: web3::types::Address, amount: web3::types::U256) {
        let key = Self::balance_storage_key(address);
        let old_amount = web3::types::U256::from_big_endian(
            self.storage
                .get(&key)
                .cloned()
                .unwrap_or_default()
                .as_bytes(),
        );
        let new_amount = old_amount - amount;
        let new_amount = crate::utils::u256_to_h256(&new_amount);
        self.storage.insert(key, new_amount);
    }

    ///
    /// Returns the balance of the specified address.
    ///
    pub fn get_balance(&self, address: web3::types::Address) -> web3::types::U256 {
        let key = Self::balance_storage_key(address);
        let balance = self.storage.get(&key).copied().unwrap_or_default();
        web3::types::U256::from_big_endian(balance.as_bytes())
    }

    ///
    /// Adds a known contract.
    ///
    fn add_known_contract(
        &mut self,
        assembly: zkevm_assembly::Assembly,
        bytecode_hash: web3::types::U256,
    ) {
        self.storage.insert(
            zkevm_tester::runners::compiler_tests::StorageKey {
                address: web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_KNOWN_CODES_STORAGE.into(),
                ),
                key: bytecode_hash,
            },
            web3::types::H256::from_low_u64_be(1),
        );
        self.known_contracts.insert(bytecode_hash, assembly);
    }

    ///
    /// Set contract as deployed on `address`. If `assembly` is none - trying to get assembly from known contracts.
    ///
    /// # Panics
    ///
    /// Will panic if some contract already deployed at `address` or `assembly` in none and contract is not found in known contracts.
    ///
    pub fn add_deployed_contract(
        &mut self,
        address: web3::types::Address,
        bytecode_hash: web3::types::U256,
        assembly: Option<zkevm_assembly::Assembly>,
    ) {
        assert!(
            !self.deployed_contracts.contains_key(&address),
            "Contract at this address already exist"
        );
        self.storage.insert(
            zkevm_tester::runners::compiler_tests::StorageKey {
                address: web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_ACCOUNT_CODE_STORAGE.into(),
                ),
                key: web3::types::U256::from_big_endian(address.as_bytes()),
            },
            crate::utils::u256_to_h256(&bytecode_hash),
        );
        let assembly = match assembly {
            Some(assembly) => assembly,
            None => self
                .known_contracts
                .get(&bytecode_hash)
                .expect("Contract not found in known contracts for deploy")
                .clone(),
        };
        self.deployed_contracts.insert(address, assembly);
    }

    ///
    /// Remove deployed contract.
    ///
    /// # Panics
    ///
    /// Will panic if any contract is not deployed at `address`
    ///
    pub fn remove_deployed_contract(&mut self, address: web3::types::Address) {
        self.storage
            .remove(&zkevm_tester::runners::compiler_tests::StorageKey {
                address: web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_ACCOUNT_CODE_STORAGE.into(),
                ),
                key: web3::types::U256::from_big_endian(address.as_bytes()),
            })
            .expect("Contract not found");
        self.deployed_contracts
            .remove(&address)
            .expect("Contract not found");
    }

    ///
    /// Adds values to storage.
    ///
    pub fn populate_storage(
        &mut self,
        values: HashMap<(web3::types::Address, web3::types::U256), web3::types::H256>,
    ) {
        self.storage.extend(values.into_iter().map(
            |((address, key), value)| (zkevm_tester::runners::compiler_tests::StorageKey {
                address,
                key,
            },
            value),
        ).collect::<HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>>());
    }

    ///
    /// Returns known contract size by code_hash, None if not found.
    ///
    pub fn get_contract_size(&self, code_hash: web3::types::U256) -> anyhow::Result<usize> {
        let mut assembly = self
            .known_contracts
            .get(&code_hash)
            .cloned()
            .expect("Always exists");
        Ok(assembly
            .compile_to_bytecode_for_mode::<16, zkevm_assembly::zkevm_opcode_defs::decoding::encoding_mode_testing::EncodingModeTesting>()?.into_iter().flatten().count())
    }

    ///
    /// Gets the balance storage key for the specified address.
    ///
    fn balance_storage_key(
        address: web3::types::Address,
    ) -> zkevm_tester::runners::compiler_tests::StorageKey {
        let mut key_preimage = Vec::with_capacity(era_compiler_common::BYTE_LENGTH_FIELD * 2);
        key_preimage.extend(vec![
            0u8;
            era_compiler_common::BYTE_LENGTH_FIELD
                - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS
        ]);
        key_preimage.extend_from_slice(address.as_bytes());
        key_preimage.extend(vec![0u8; era_compiler_common::BYTE_LENGTH_FIELD]);

        let key_string = era_compiler_llvm_context::eravm_utils::keccak256(key_preimage.as_slice());
        let key = web3::types::U256::from_str(key_string.as_str()).expect("Always valid");
        zkevm_tester::runners::compiler_tests::StorageKey {
            address: web3::types::Address::from_low_u64_be(
                zkevm_opcode_defs::ADDRESS_ETH_TOKEN.into(),
            ),
            key,
        }
    }
}
