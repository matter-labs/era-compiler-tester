//!
//! The EraVM system context.
//!

use std::collections::HashMap;
use std::ops::Add;
use std::str::FromStr;

///
/// The EraVM system context.
///
pub struct SystemContext;

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
    const CHAIND_ID: u64 = 280;

    /// The default origin for tests.
    const TX_ORIGIN: &'static str =
        "0x0000000000000000000000009292929292929292929292929292929292929292";

    /// The default gas price for tests.
    const GAS_PRICE: u64 = 3000000000;

    /// The default block gas limit for tests.
    const BLOCK_GAS_LIMIT: u64 = (1 << 30);

    /// The default coinbase for tests.
    const COIN_BASE: &'static str =
        "0x0000000000000000000000000000000000000000000000000000000000008001";

    /// The default block difficulty for tests.
    const BLOCK_DIFFICULTY: u64 = 2500000000000000;

    /// The default base fee for tests.
    const BASE_FEE: u64 = 7;

    /// The default current block number for tests.
    const CURRENT_BLOCK_NUMBER: u128 = 300;

    /// The default current block timestamp for tests.
    const CURRENT_BLOCK_TIMESTAMP: u128 = 0xdeadbeef;

    /// The default zero block hash for tests.
    const ZERO_BLOCK_HASH: &'static str =
        "0x3737373737373737373737373737373737373737373737373737373737373737";

    ///
    /// Returns the storage values for the system context.
    ///
    pub fn create_storage() -> HashMap<zkevm_tester::compiler_tests::StorageKey, web3::types::H256>
    {
        let mut system_context_values = vec![
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_CHAIN_ID_POSITION),
                web3::types::H256::from_low_u64_be(Self::CHAIND_ID),
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
                web3::types::H256::from_low_u64_be(Self::BLOCK_GAS_LIMIT),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_COINBASE_POSITION),
                web3::types::H256::from_str(Self::COIN_BASE).expect("Always valid"),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_DIFFICULTY_POSITION),
                web3::types::H256::from_low_u64_be(Self::BLOCK_DIFFICULTY),
            ),
            (
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_BASE_FEE_POSITION),
                web3::types::H256::from_low_u64_be(Self::BASE_FEE),
            ),
            (
                web3::types::H256::from_low_u64_be(
                    Self::SYSTEM_CONTEXT_VIRTUAL_BLOCK_UPGRADE_INFO_POSITION,
                ),
                web3::types::H256::from_low_u64_be(Self::CURRENT_BLOCK_NUMBER as u64),
            ),
        ];

        let block_info_bytes = [
            Self::CURRENT_BLOCK_NUMBER.to_be_bytes(),
            Self::CURRENT_BLOCK_TIMESTAMP.to_be_bytes(),
        ]
        .concat();

        system_context_values.push((
            web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_VIRTUAL_L2_BLOCK_INFO_POSITION),
            web3::types::H256::from_slice(block_info_bytes.as_slice()),
        ));

        for index in 0..Self::CURRENT_BLOCK_NUMBER {
            let padded_index = [[0u8; 16], index.to_be_bytes()].concat();
            let padded_slot =
                web3::types::H256::from_low_u64_be(Self::SYSTEM_CONTEXT_BLOCK_HASH_POSITION)
                    .to_fixed_bytes()
                    .to_vec();
            let key = web3::signing::keccak256([padded_index, padded_slot].concat().as_slice());

            let mut hash = web3::types::U256::from_str(Self::ZERO_BLOCK_HASH)
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
                zkevm_tester::compiler_tests::StorageKey {
                    address: web3::types::Address::from_low_u64_be(
                        zkevm_opcode_defs::ADDRESS_SYSTEM_CONTEXT.into(),
                    ),
                    key: web3::types::U256::from_big_endian(key.as_bytes()),
                },
                value,
            );
        }

        storage
    }
}
