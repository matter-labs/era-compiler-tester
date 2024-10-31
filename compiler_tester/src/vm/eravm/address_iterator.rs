//!
//! The EraVM deploy address iterator.
//!

use std::collections::HashMap;
use std::str::FromStr;

use crate::vm::address_iterator::AddressIterator;

///
/// The EraVM deploy address iterator.
///
#[derive(Debug, Clone)]
pub struct EraVMAddressIterator {
    /// The accounts create nonces.
    pub nonces: HashMap<web3::types::Address, usize>,
}

impl EraVMAddressIterator {
    /// The create prefix, `keccak256("zksyncCreate")`.
    const CREATE_PREFIX: &'static str =
        "63bae3a9951d38e8a3fbb7b70909afc1200610fc5bc55ade242f815974674f23";
}

impl Default for EraVMAddressIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl EraVMAddressIterator {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self {
            nonces: HashMap::new(),
        }
    }
}

impl AddressIterator for EraVMAddressIterator {
    fn next(
        &mut self,
        caller: &web3::types::Address,
        increment_nonce: bool,
    ) -> web3::types::Address {
        let nonce = self.nonces.get(caller).cloned().unwrap_or_default();

        let mut bytes = web3::types::H256::from_str(Self::CREATE_PREFIX)
            .expect("Invalid constant create prefix")
            .as_bytes()
            .to_vec();
        bytes.extend(
            [0; era_compiler_common::BYTE_LENGTH_FIELD
                - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS],
        );
        bytes.extend(caller.to_fixed_bytes());
        bytes.extend(
            [0; era_compiler_common::BYTE_LENGTH_FIELD - era_compiler_common::BYTE_LENGTH_X64],
        );
        bytes.extend(nonce.to_be_bytes());

        let address = web3::types::Address::from_slice(
            &web3::signing::keccak256(bytes.as_slice())[era_compiler_common::BYTE_LENGTH_FIELD
                - era_compiler_common::BYTE_LENGTH_ETH_ADDRESS..],
        );

        if increment_nonce {
            self.increment_nonce(caller);
        }

        address
    }

    fn increment_nonce(&mut self, caller: &web3::types::Address) {
        let nonce = self.nonce(caller);
        self.nonces.insert(*caller, nonce + 1);
    }

    fn nonce(&mut self, caller: &web3::types::Address) -> usize {
        *self.nonces.entry(*caller).or_default()
    }
}
