//!
//! The deploy address predictor.
//!

use std::collections::HashMap;
use std::str::FromStr;

///
/// The deploy address predictor.
///
#[derive(Debug, Clone)]
pub struct AddressPredictor {
    /// The accounts create nonces.
    nonces: HashMap<web3::types::Address, usize>,
}

impl AddressPredictor {
    /// The create prefix.
    const CREATE_PREFIX: &'static str =
        "63bae3a9951d38e8a3fbb7b70909afc1200610fc5bc55ade242f815974674f23"; // keccak256("zksyncCreate")
}

impl AddressPredictor {
    ///
    /// Create new address predictor instance.
    ///
    pub fn new() -> Self {
        Self {
            nonces: HashMap::new(),
        }
    }

    ///
    /// Get the next deploy address (without nonce incrementing).
    ///
    pub fn advance_next_address(&self, caller: &web3::types::Address) -> web3::types::Address {
        let nonce = self.nonces.get(caller).cloned().unwrap_or_default();

        let mut bytes = web3::types::H256::from_str(Self::CREATE_PREFIX)
            .expect("Invalid constant create prefix")
            .as_bytes()
            .to_vec();
        bytes.extend(
            [0; compiler_common::BYTE_LENGTH_FIELD - compiler_common::BYTE_LENGTH_ETH_ADDRESS],
        );
        bytes.extend(caller.to_fixed_bytes());
        bytes.extend([0; compiler_common::BYTE_LENGTH_FIELD - std::mem::size_of::<usize>()]);
        bytes.extend(nonce.to_be_bytes());

        let address = web3::types::Address::from_slice(
            &web3::signing::keccak256(bytes.as_slice())
                [compiler_common::BYTE_LENGTH_FIELD - compiler_common::BYTE_LENGTH_ETH_ADDRESS..],
        );
        address
    }

    ///
    /// Increments caller nonce.
    ///
    pub fn increment_nonce(&mut self, caller: web3::types::Address) {
        let nonce = self.nonces.entry(caller).or_insert(0);
        *nonce += 1;
    }

    ///
    /// Get the next deploy address (increments nonce when called).
    ///
    pub fn next_address(&mut self, caller: web3::types::Address) -> web3::types::Address {
        let address = self.advance_next_address(&caller);
        self.increment_nonce(caller);
        address
    }
}
