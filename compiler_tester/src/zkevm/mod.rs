//!
//! The zkEVM wrapper.
//!

pub mod execution_result;
pub mod system_context;
pub mod system_contracts;

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::compilers::downloader::config::Config as DownloaderConfig;

use self::execution_result::ExecutionResult;
use self::system_context::SystemContext;
use self::system_contracts::SystemContracts;

///
/// The zkEVM wrapper.
///
#[derive(Clone)]
#[allow(non_camel_case_types)]
pub struct zkEVM {
    /// The storage state.
    storage: HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>,
    /// The deployed contracts.
    deployed_contracts: HashMap<web3::types::Address, zkevm_assembly::Assembly>,
    /// The default account abstraction contract code hash.
    default_aa_code_hash: web3::types::U256,
    /// The known contracts.
    known_contracts: HashMap<web3::types::U256, zkevm_assembly::Assembly>,
}

impl zkEVM {
    ///
    /// Creates and initializes new zkEVM instance.
    ///
    pub fn initialize(
        system_contracts_solc_downloader_config: DownloaderConfig,
        system_contracts_debug_config: Option<compiler_llvm_context::DebugConfig>,
        system_contracts_path: Option<PathBuf>,
        system_contracts_save_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let solc_version = system_contracts_solc_downloader_config
            .binaries
            .keys()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "zkEVM initializer could find the `solc` version for system contracts"
                )
            })?;
        let solc_version = semver::Version::parse(solc_version.as_str())?;

        let system_contracts = SystemContracts::load_or_build(
            solc_version,
            system_contracts_debug_config,
            system_contracts_path,
            system_contracts_save_path,
        )?;

        let storage = SystemContext::create_storage();

        let mut vm = Self {
            storage,
            deployed_contracts: HashMap::new(),
            default_aa_code_hash: system_contracts.default_aa.bytecode_hash,
            known_contracts: HashMap::new(),
        };

        vm.add_known_contract(
            system_contracts.default_aa.assembly,
            system_contracts.default_aa.bytecode_hash,
        );

        for (address, build) in system_contracts.deployed_contracts {
            vm.add_deployed_contract(address, build.bytecode_hash, Some(build.assembly));
        }

        Ok(vm)
    }

    ///
    /// Clones the vm instance from arc and adds known contracts.
    ///
    /// TODO: check if can be made copyless
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
    /// Runs a contract call transaction.
    ///
    pub fn contract_call<const M: bool>(
        &mut self,
        test_name: String,
        entry_address: web3::types::Address,
        caller: web3::types::Address,
        value: Option<u128>,
        calldata: Vec<u8>,
    ) -> anyhow::Result<ExecutionResult> {
        let context_u128_value;
        let mut entry_address = entry_address;
        let vm_launch_option;

        if M {
            context_u128_value = 0;
            if let Some(value) = value {
                self.mint_ether(caller, web3::types::U256::from(value));

                let r3 = Some(web3::types::U256::from(value));
                let r4 = Some(web3::types::U256::from_big_endian(entry_address.as_bytes()));
                let r5 = Some(web3::types::U256::from(u8::from(
                    compiler_llvm_context::SYSTEM_CALL_BIT,
                )));

                entry_address = web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_MSG_VALUE.into(),
                );

                vm_launch_option =
                    zkevm_tester::runners::compiler_tests::VmLaunchOption::ManualCallABI(
                        zkevm_tester::runners::compiler_tests::FullABIParams {
                            is_constructor: false,
                            is_system_call: true,
                            r3_value: r3,
                            r4_value: r4,
                            r5_value: r5,
                        },
                    );
            } else {
                vm_launch_option = zkevm_tester::runners::compiler_tests::VmLaunchOption::Call;
            }
        } else {
            vm_launch_option = zkevm_tester::runners::compiler_tests::VmLaunchOption::Call;
            if let Some(value) = value {
                self.mint_ether(entry_address, web3::types::U256::from(value));
                context_u128_value = value;
            } else {
                context_u128_value = 0;
            }
        }

        self.run(
            test_name,
            entry_address,
            caller,
            context_u128_value,
            calldata,
            vm_launch_option,
        )
    }

    ///
    /// Runs the several contracts on the VM with the specified data and returns the result.
    ///
    pub fn run(
        &mut self,
        test_name: String,
        entry_address: web3::types::Address,
        caller: web3::types::Address,
        u128_value: u128,
        calldata: Vec<u8>,
        vm_launch_option: zkevm_tester::runners::compiler_tests::VmLaunchOption,
    ) -> anyhow::Result<ExecutionResult> {
        let mut trace_file_path = PathBuf::from_str("./trace/").expect("Always valid");
        let trace_file_name = regex::Regex::new("[^A-Za-z0-9]+")
            .expect("Always valid")
            .replace_all(test_name.as_str(), "_")
            .to_string();
        trace_file_path.push(trace_file_name);

        let context = zkevm_tester::runners::compiler_tests::VmExecutionContext::new(
            entry_address,
            caller,
            u128_value,
            0,
        );

        let snapshot = tokio::runtime::Runtime::new()
            .expect("Tokio error")
            .block_on(
                zkevm_tester::runners::compiler_tests::run_vm_multi_contracts(
                    trace_file_path.to_string_lossy().to_string(),
                    self.deployed_contracts.clone(),
                    calldata,
                    self.storage.clone(),
                    entry_address,
                    Some(context),
                    vm_launch_option,
                    <usize>::MAX,
                    self.known_contracts.clone(),
                    self.default_aa_code_hash,
                ),
            )
            .map_err(|error| anyhow::anyhow!("Internal error: failed to run vm: {}", error))?;

        let result = ExecutionResult::from(&snapshot);
        self.storage = snapshot.storage;
        for (address, assembly) in snapshot.deployed_contracts.into_iter() {
            if self.deployed_contracts.contains_key(&address) {
                continue;
            }

            self.deployed_contracts.insert(address, assembly);
        }

        Ok(result)
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
    /// Mints some Ether value for the specified caller.
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
    /// Burns some Ether value for the specified caller.
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
        values: HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>,
    ) {
        self.storage.extend(values);
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
            .compile_to_bytecode_for_mode::<16, zkevm_opcode_defs::decoding::encoding_mode_testing::EncodingModeTesting>()?.into_iter().flatten().count())
    }

    ///
    /// Gets the balance storage key for the specified address.
    ///
    fn balance_storage_key(
        address: web3::types::Address,
    ) -> zkevm_tester::runners::compiler_tests::StorageKey {
        let mut key_preimage = Vec::with_capacity(compiler_common::BYTE_LENGTH_FIELD * 2);
        key_preimage.extend(vec![
            0u8;
            compiler_common::BYTE_LENGTH_FIELD
                - compiler_common::BYTE_LENGTH_ETH_ADDRESS
        ]);
        key_preimage.extend_from_slice(address.as_bytes());
        key_preimage.extend(vec![0u8; compiler_common::BYTE_LENGTH_FIELD]);

        let key_string = compiler_llvm_context::keccak256(key_preimage.as_slice());
        let key = web3::types::U256::from_str(key_string.as_str()).expect("Always valid");
        zkevm_tester::runners::compiler_tests::StorageKey {
            address: web3::types::Address::from_low_u64_be(
                zkevm_opcode_defs::ADDRESS_ETH_TOKEN.into(),
            ),
            key,
        }
    }

    ///
    /// Adds known contract.
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
}
