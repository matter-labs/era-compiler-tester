//!
//! The compiler tester utils.
//!

///
/// Overrides the default formatting for `Address`, which replaces the middle with an ellipsis.
///
pub fn address_as_string(value: &web3::types::Address) -> String {
    hex::encode(value.as_bytes())
}

///
/// Overrides the default formatting for `U256`, which replaces the middle with an ellipsis.
///
pub fn u256_as_string(value: &web3::types::U256) -> String {
    let mut bytes = vec![0; era_compiler_common::BYTE_LENGTH_FIELD];
    value.to_big_endian(&mut bytes);
    hex::encode(bytes)
}

///
/// Converts `[u8; 32]` into `Address`.
///
pub fn bytes32_to_address(
    bytes: &[u8; era_compiler_common::BYTE_LENGTH_FIELD],
) -> web3::types::Address {
    web3::types::Address::from_slice(
        &bytes[bytes.len() - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS..],
    )
}

///
/// Converts `U256` into `Address`.
///
pub fn u256_to_address(value: &web3::types::U256) -> web3::types::Address {
    let mut bytes = vec![0; era_compiler_common::BYTE_LENGTH_FIELD];
    value.to_big_endian(&mut bytes);
    web3::types::Address::from_slice(
        &bytes[bytes.len() - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS..],
    )
}

///
/// Converts `U256` into `H256`.
///
pub fn u256_to_h256(value: &web3::types::U256) -> web3::types::H256 {
    let mut bytes = vec![0; era_compiler_common::BYTE_LENGTH_FIELD];
    value.to_big_endian(&mut bytes);
    web3::types::H256::from_slice(bytes.as_slice())
}

///
/// Converts `H256` into `U256`.
///
pub fn h256_to_u256(value: &web3::types::H256) -> web3::types::U256 {
    web3::types::U256::from_big_endian(value.as_bytes())
}
