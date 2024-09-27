//!
//! The Solidity adapter library.
//!

#![allow(clippy::assigning_clones)]

pub mod index;
pub mod test;

use std::ops::Add;
use std::str::FromStr;

pub use self::index::enabled::EnabledTest;
pub use self::index::FSEntity;
pub use self::test::function_call::event::Event;
pub use self::test::function_call::gas_option::GasOption;
pub use self::test::function_call::FunctionCall;
pub use self::test::params::abi_encoder_v1_only::ABIEncoderV1Only;
pub use self::test::params::compile_to_ewasm::CompileToEwasm;
pub use self::test::params::compile_via_yul::CompileViaYul;
pub use self::test::params::evm_version::EVMVersion;
pub use self::test::params::evm_version::EVM;
pub use self::test::params::revert_strings::RevertStrings;
pub use self::test::params::Params;
pub use self::test::Test;

/// The default contract address.
pub const DEFAULT_CONTRACT_ADDRESS: &str = "c06afe3a8444fc0004668591e8306bfb9968e79e";

/// The index of the account used as the default caller.
pub const DEFAULT_ACCOUNT_INDEX: usize = 0;

///
/// First pre-generated account address.
///
const ZERO_ADDRESS: &str = "1212121212121212121212121212120000000012";

/// The caller address multiplier.
const ADDRESS_INDEX_MULTIPLIER: usize = 4096; // 16^3

///
/// Returns address of pre-generated account by index.
///
pub fn account_address(index: usize) -> web3::types::Address {
    let address = web3::types::U256::from_str(ZERO_ADDRESS).expect("Default address");
    let address = address.add(index * ADDRESS_INDEX_MULTIPLIER);

    let mut bytes = [0u8; era_compiler_common::BYTE_LENGTH_FIELD];
    address.to_big_endian(&mut bytes);
    web3::types::Address::from_slice(
        &bytes[era_compiler_common::BYTE_LENGTH_FIELD
            - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS..],
    )
}
