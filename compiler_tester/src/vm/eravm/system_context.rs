//!
//! The EraVM system context.
//!

use std::collections::HashMap;
use std::ops::Add;
use std::str::FromStr;

use crate::target::Target;

use solidity_adapter::EVMVersion::{self, Lesser, LesserEquals};
use solidity_adapter::EVM::Paris;

///
/// The EraVM system context.
///
pub struct SystemContext;

pub struct EVMContext {
    pub chain_id: u64,
    pub coinbase: &'static str,
    pub block_number: u128,
    pub block_timestamp: u128,
    pub block_gas_limit: u64,
    pub block_difficulty: &'static str,
    pub base_fee: u64,
    pub zero_block_hash: &'static str,
}

impl SystemContext {
    /// The system context chain ID value position in the storage.
    const SYSTEM_CONTEXT_CHAIN_ID_POSITION: u64 = 0;

    /// The system context origin value position in the storage.
    const SYSTEM_CONTEXT_ORIGIN_POSITION: u64 = 1;

    /// The system context gas price value position in the storage.
    const SYSTEM_CONTEXT_GAS_PRICE_POSITION: u64 = 2;

    /// The system context block gas limit value position in the storage.
    const SYSTEM_CONTEXT_BLOCK_GAS_LIMIT_POSITION: u64 = 3;

    /// The system context coinbase value position in the storage.
    const SYSTEM_CONTEXT_COINBASE_POSITION: u64 = 4;

    /// The system context difficulty value position in the storage.
    const SYSTEM_CONTEXT_DIFFICULTY_POSITION: u64 = 5;

    /// The system context base fee value position in the storage.
    const SYSTEM_CONTEXT_BASE_FEE_POSITION: u64 = 6;

    /// The system context block hashes mapping position in the storage.
    const SYSTEM_CONTEXT_BLOCK_HASH_POSITION: u64 = 8;

    /// The system context current virtual L2 block info value position in the storage.
    const SYSTEM_CONTEXT_VIRTUAL_L2_BLOCK_INFO_POSITION: u64 = 268;

    /// The system context virtual blocks upgrade info position in the storage.
    const SYSTEM_CONTEXT_VIRTUAL_BLOCK_UPGRADE_INFO_POSITION: u64 = 269;

    /// The ZKsync chain ID.
    const CHAIND_ID_ERAVM: u64 = 280;
    /// The Ethereum chain ID.
    const CHAIND_ID_EVM: u64 = 1;

    /// The default origin for tests.
    const TX_ORIGIN: &'static str =
        "0x0000000000000000000000009292929292929292929292929292929292929292";

    /// The default gas price for tests.
    const GAS_PRICE: u64 = 3000000000;

    /// The default block gas limit for EraVM tests.
    const BLOCK_GAS_LIMIT_ERAVM: u64 = (1 << 30);
    /// The default block gas limit for EVM tests.
    const BLOCK_GAS_LIMIT_EVM: u64 = 20000000;

    /// The default coinbase for EraVM tests.
    const COIN_BASE_ERAVM: &'static str =
        "0x0000000000000000000000000000000000000000000000000000000000008001";
    /// The default coinbase for EVM tests.
    const COIN_BASE_EVM: &'static str =
        "0x0000000000000000000000007878787878787878787878787878787878787878";

    /// The default block difficulty for EraVM tests.
    const BLOCK_DIFFICULTY_ERAVM: u64 = 2500000000000000;
    /// The block difficulty for EVM tests using a post paris version.
    const BLOCK_DIFFICULTY_EVM_POST_PARIS: &'static str =
        "0xa86c2e601b6c44eb4848f7d23d9df3113fbcac42041c49cbed5000cb4f118777";
    /// The block difficulty for EVM tests using a pre paris version.
    const BLOCK_DIFFICULTY_EVM_PRE_PARIS: &'static str =
        "0x000000000000000000000000000000000000000000000000000000000bebc200";

    /// The default base fee for tests.
    const BASE_FEE: u64 = 7;

    /// The default current block number for EraVM tests.
    const CURRENT_BLOCK_NUMBER_ERAVM: u128 = 300;
    /// The default current block number for EVM tests.
    const CURRENT_BLOCK_NUMBER_EVM: u128 = 1;

    /// The default current block timestamp for EraVM tests.
    const CURRENT_BLOCK_TIMESTAMP_ERAVM: u128 = 0xdeadbeef;
    /// The default current block timestamp for EVM tests.
    const CURRENT_BLOCK_TIMESTAMP_EVM: u128 = 40;

    /// The default zero block hash for EraVM tests.
    const ZERO_BLOCK_HASH_ERAVM: &'static str =
        "0x3737373737373737373737373737373737373737373737373737373737373737";
    /// The default zero block hash for EVM tests.
    const ZERO_BLOCK_HASH_EVM: &'static str =
        "0x3737373737373737373737373737373737373737373737373737373737373737";

    ///
    /// Returns the storage values for the system context.
    ///
    pub fn create_storage(
        target: Target,
    ) -> HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256> {
        let chain_id = match target {
            Target::EraVM => Self::CHAIND_ID_ERAVM,
            Target::EVM | Target::EVMEmulator => Self::CHAIND_ID_EVM,
        };
        let coinbase = match target {
            Target::EraVM => Self::COIN_BASE_ERAVM,
            Target::EVM | Target::EVMEmulator => Self::COIN_BASE_EVM,
        };

        let block_number = match target {
            Target::EraVM => Self::CURRENT_BLOCK_NUMBER_ERAVM,
            Target::EVM | Target::EVMEmulator => Self::CURRENT_BLOCK_NUMBER_EVM,
        };
        let block_timestamp = match target {
            Target::EraVM => Self::CURRENT_BLOCK_TIMESTAMP_ERAVM,
            Target::EVM | Target::EVMEmulator => Self::CURRENT_BLOCK_TIMESTAMP_EVM,
        };
        let block_gas_limit = match target {
            Target::EraVM => Self::BLOCK_GAS_LIMIT_ERAVM,
            Target::EVM | Target::EVMEmulator => Self::BLOCK_GAS_LIMIT_EVM,
        };

        let mut system_context_values = vec![
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_CHAIN_ID_POSITION),
                web3::types::H256::from_low_u64_be(chain_id),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_ORIGIN_POSITION),
                web3::types::H256::from_str(Self::TX_ORIGIN).expect("Always valid"),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_GAS_PRICE_POSITION),
                web3::types::H256::from_low_u64_be(Self::GAS_PRICE),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_BLOCK_GAS_LIMIT_POSITION),
                web3::types::H256::from_low_u64_be(block_gas_limit),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_COINBASE_POSITION),
                web3::types::H256::from_str(coinbase).expect("Always valid"),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_DIFFICULTY_POSITION),
                match target {
                    Target::EraVM => {
                        web3::types::H256::from_low_u64_be(Self::BLOCK_DIFFICULTY_ERAVM)
                    }
                    // This block difficulty is set by default, but it can be overridden if the test needs it.
                    Target::EVM | Target::EVMEmulator => {
                        web3::types::H256::from_str(Self::BLOCK_DIFFICULTY_EVM_POST_PARIS)
                            .expect("Always valid")
                    }
                },
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_BASE_FEE_POSITION),
                web3::types::H256::from_low_u64_be(Self::BASE_FEE),
            ),
            (
                web3::types::H256::from_low_u64_be(
                    Self::SYSTEM_CONTEXT_VIRTUAL_BLOCK_UPGRADE_INFO_POSITION,
                ),
                web3::types::H256::from_low_u64_be(block_number as u64),
            ),
        ];

        let block_info_bytes = [block_number.to_be_bytes(), block_timestamp.to_be_bytes()].concat();

        system_context_values.push((
            web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_VIRTUAL_L2_BLOCK_INFO_POSITION),
            web3::types::H256::from_slice(block_info_bytes.as_slice()),
        ));

        for index in 0..block_number {
            let padded_index = [[0u8; 16], index.to_be_bytes()].concat();
            let padded_slot =
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_BLOCK_HASH_POSITION)
                    .to_fixed_bytes()
                    .to_vec();
            let key = web3::signing::keccak256([padded_index, padded_slot].concat().as_slice());

            let mut hash = web3::types::U256::from_str(match target {
                Target::EraVM => Self::ZERO_BLOCK_HASH_ERAVM,
                Target::EVM | Target::EVMEmulator => Self::ZERO_BLOCK_HASH_EVM,
            })
            .expect("Invalid zero block hash const");
            hash = hash.add(web3::types::U256::from(index));
            let mut hash_bytes = [0u8; era_compiler_common::BYTE_LENGTH_FIELD];
            hash.to_big_endian(&mut hash_bytes);

            system_context_values.push((
                web3::types::H256::from(key),
                web3::types::H256::from_slice(hash_bytes.as_slice()),
            ));
        }

        let mut storage = HashMap::new();

        for (key, value) in system_context_values {
            storage.insert(
                zkevm_tester::runners::compiler_tests::StorageKey {
                    address: web3::types::Address::from_low_u64_be(
                        zkevm_opcode_defs::ADDRESS_SYSTEM_CONTEXT.into(),
                    ),
                    key: web3::types::U256::from_big_endian(key.as_bytes()),
                },
                value,
            );
        }

        if target == Target::EVM {
            let rich_addresses: Vec<web3::types::Address> = (0..=9)
                .map(|address_id| {
                    format!(
                        "0x121212121212121212121212121212000000{}{}",
                        address_id, "012"
                    )
                })
                .map(|string| {
                    web3::types::Address::from_str(string.as_str()).expect("Always valid")
                })
                .collect();
            rich_addresses.iter().for_each(|address| {
                let address_h256 = crate::utils::address_to_h256(address);
                let bytes = [
                    address_h256.as_bytes(),
                    &[0; era_compiler_common::BYTE_LENGTH_FIELD],
                ]
                .concat();
                let key = web3::signing::keccak256(&bytes).into();
                let storage_key = zkevm_tester::runners::compiler_tests::StorageKey {
                    address: web3::types::Address::from_low_u64_be(
                        zkevm_opcode_defs::ADDRESS_ETH_TOKEN.into(),
                    ),
                    key,
                };
                let initial_balance =
                    crate::utils::u256_to_h256(&(web3::types::U256::one() << 100));
                storage.insert(storage_key, initial_balance);
            });

            // Fund the 0x01 address with 1 token to match the behavior of upstream Solidity tests.
            let address_ecrecover = crate::utils::address_to_h256(
                &web3::types::Address::from_low_u64_be(zkevm_opcode_defs::ADDRESS_ECRECOVER.into()),
            );
            let bytes = [
                address_ecrecover.as_bytes(),
                &[0; era_compiler_common::BYTE_LENGTH_FIELD],
            ]
            .concat();
            let key = web3::signing::keccak256(&bytes).into();
            let storage_key = zkevm_tester::runners::compiler_tests::StorageKey {
                address: web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_ETH_TOKEN.into(),
                ),
                key,
            };
            let initial_balance = crate::utils::u256_to_h256(&web3::types::U256::one());
            storage.insert(storage_key, initial_balance);
        };

        storage
    }

    ///
    /// Returns constants for the specified EVM version.
    ///
    pub fn get_constants_evm(evm_version: Option<EVMVersion>) -> EVMContext {
        match evm_version {
            Some(Lesser(Paris) | LesserEquals(Paris)) => EVMContext {
                chain_id: SystemContext::CHAIND_ID_EVM,
                coinbase: &SystemContext::COIN_BASE_EVM[2..],
                block_number: SystemContext::CURRENT_BLOCK_NUMBER_EVM,
                block_timestamp: SystemContext::CURRENT_BLOCK_TIMESTAMP_EVM,
                block_gas_limit: SystemContext::BLOCK_GAS_LIMIT_EVM,
                block_difficulty: &SystemContext::BLOCK_DIFFICULTY_EVM_PRE_PARIS[2..],
                base_fee: SystemContext::BASE_FEE,
                zero_block_hash: SystemContext::ZERO_BLOCK_HASH_EVM,
            },
            _ => EVMContext {
                chain_id: SystemContext::CHAIND_ID_EVM,
                coinbase: &SystemContext::COIN_BASE_EVM[2..],
                block_number: SystemContext::CURRENT_BLOCK_NUMBER_EVM,
                block_timestamp: SystemContext::CURRENT_BLOCK_TIMESTAMP_EVM,
                block_gas_limit: SystemContext::BLOCK_GAS_LIMIT_EVM,
                block_difficulty: &SystemContext::BLOCK_DIFFICULTY_EVM_POST_PARIS[2..],
                base_fee: SystemContext::BASE_FEE,
                zero_block_hash: SystemContext::ZERO_BLOCK_HASH_EVM,
            },
        }
    }

    ///
    /// Returns addresses that must be funded for testing.
    ///
    pub fn get_rich_addresses() -> Vec<web3::types::Address> {
        (0..=9)
            .map(|address_id| {
                format!(
                    "0x121212121212121212121212121212000000{}{}",
                    address_id, "012"
                )
            })
            .map(|string| web3::types::Address::from_str(&string).unwrap())
            .collect()
    }

    ///
    /// Sets the storage values for the system context to the pre-Paris values.
    ///
    pub fn set_pre_paris_contracts(
        storage: &mut HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>,
    ) {
        storage.insert(
            zkevm_tester::runners::compiler_tests::StorageKey {
                address: web3::types::Address::from_low_u64_be(
                    zkevm_opcode_defs::ADDRESS_SYSTEM_CONTEXT.into(),
                ),
                key: web3::types::U256::from_big_endian(
                    web3::types::H256::from_low_u64_be(
                        SystemContext::SYSTEM_CONTEXT_DIFFICULTY_POSITION,
                    )
                    .as_bytes(),
                ),
            },
            web3::types::H256::from_str(SystemContext::BLOCK_DIFFICULTY_EVM_PRE_PARIS)
                .expect("Always valid"),
        );
    }
}
