//!
//! The EVM deploy address iterator.
//!

use std::collections::HashMap;
use std::str::FromStr;

use crate::vm::address_iterator::AddressIterator;

///
/// The EVM deploy address iterator.
///
#[derive(Debug, Clone)]
pub struct EVMAddressIterator {
    /// The accounts create nonces.
    nonces: HashMap<web3::types::Address, usize>,
    /// Whether to start nonce from zero.
    start_nonce_from_zero: bool,
}

impl EVMAddressIterator {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(start_nonce_from_zero: bool) -> Self {
        Self {
            nonces: HashMap::new(),
            start_nonce_from_zero,
        }
    }
}

impl AddressIterator for EVMAddressIterator {
    fn next(
        &mut self,
        caller: &web3::types::Address,
        increment_nonce: bool,
    ) -> web3::types::Address {
        let mut stream = rlp::RlpStream::new_list(2);
        stream.append(caller);
        stream.append(&self.nonce(caller));

        let hash = era_compiler_common::Hash::keccak256(&stream.out());
        let address = web3::types::Address::from_str(
            &hash.to_string()[2 + 2
                * (era_compiler_common::BYTE_LENGTH_FIELD
                    - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS)..],
        )
        .expect("Always valid");

        if increment_nonce {
            self.increment_nonce(caller);
        }

        address
    }

    fn increment_nonce(&mut self, caller: &web3::types::Address) {
        let nonce = self
            .nonces
            .entry(*caller)
            .or_insert(if self.start_nonce_from_zero { 0 } else { 1 });
        *nonce += 1;
    }

    fn nonce(&mut self, caller: &web3::types::Address) -> usize {
        *self
            .nonces
            .entry(*caller)
            .or_insert(if self.start_nonce_from_zero { 0 } else { 1 })
    }
}
