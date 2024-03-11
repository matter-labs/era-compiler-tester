//!
//! The EVM deploy address predictor.
//!

use std::collections::HashMap;
use std::str::FromStr;

use crate::vm::AddressPredictorIterator;

///
/// The EVM deploy address predictor.
///
#[derive(Debug, Clone)]
pub struct AddressPredictor {
    /// The accounts create nonces.
    nonces: HashMap<web3::types::Address, usize>,
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
    /// Increments caller nonce.
    ///
    pub fn increment_nonce(&mut self, caller: &web3::types::Address) {
        let nonce = self.nonces.entry(*caller).or_insert(0);
        *nonce += 1;
    }
}

impl AddressPredictorIterator for AddressPredictor {
    fn next(
        &mut self,
        caller: &web3::types::Address,
        increment_nonce: bool,
    ) -> web3::types::Address {
        let address = web3::types::Address::from_str("9f1ebbf13029eaa4d453a2eb221f322404be895b")
            .expect("Always valid");

        if increment_nonce {
            self.increment_nonce(caller);
        }

        address
    }
}
