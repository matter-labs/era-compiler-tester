//!
//! The EVM runtime.
//!

use std::collections::HashMap;

///
/// The EVM runtime.
///
#[derive(Debug, Default)]
pub struct Runtime {
    /// The contract codes.
    pub codes: HashMap<web3::types::Address, Vec<u8>>,
    /// The contract balances.
    pub balances: HashMap<web3::types::Address, web3::types::U256>,
    /// The contract nonces.
    pub nonces: HashMap<web3::types::Address, web3::types::U256>,
    /// The contract storages.
    pub storages: HashMap<web3::types::Address, HashMap<web3::types::H256, web3::types::H256>>,
    /// The contract logs.
    pub logs: Vec<evm::Log>,
}

impl Runtime {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        codes: HashMap<web3::types::Address, Vec<u8>>,
        balances: HashMap<web3::types::Address, web3::types::U256>,
        nonces: HashMap<web3::types::Address, web3::types::U256>,
        storages: HashMap<web3::types::Address, HashMap<web3::types::H256, web3::types::H256>>,
        logs: Vec<evm::Log>,
    ) -> Self {
        Self {
            codes,
            balances,
            nonces,
            storages,
            logs,
        }
    }
}

impl evm::RuntimeEnvironment for Runtime {
    fn block_hash(&self, number: web3::types::U256) -> web3::types::H256 {
        crate::utils::u256_to_h256(
            &web3::types::U256::from_str_radix(
                "3737373737373737373737373737373737373737373737373737373737373862",
                era_compiler_common::BASE_HEXADECIMAL,
            )
            .expect("Always valid"),
        )
    }

    fn block_number(&self) -> web3::types::U256 {
        web3::types::U256::from_str_radix("12c", era_compiler_common::BASE_HEXADECIMAL)
            .expect("Always valid")
    }

    fn block_coinbase(&self) -> web3::types::H160 {
        crate::utils::u256_to_address(
            &web3::types::U256::from_str_radix("8001", era_compiler_common::BASE_HEXADECIMAL)
                .expect("Always valid"),
        )
    }

    fn block_timestamp(&self) -> web3::types::U256 {
        web3::types::U256::from_str_radix("deadbeef", era_compiler_common::BASE_HEXADECIMAL)
            .expect("Always valid")
    }

    fn block_difficulty(&self) -> web3::types::U256 {
        web3::types::U256::from_str_radix("8e1bc9bf04000", era_compiler_common::BASE_HEXADECIMAL)
            .expect("Always valid")
    }

    fn block_randomness(&self) -> Option<web3::types::H256> {
        None
    }

    fn block_gas_limit(&self) -> web3::types::U256 {
        web3::types::U256::from_str_radix("40000000", era_compiler_common::BASE_HEXADECIMAL)
            .expect("Always valid")
    }

    fn block_base_fee_per_gas(&self) -> web3::types::U256 {
        web3::types::U256::from_dec_str("7").expect("Always valid")
    }

    fn chain_id(&self) -> web3::types::U256 {
        web3::types::U256::from_dec_str("280").expect("Always valid")
    }
}

impl evm::RuntimeBaseBackend for Runtime {
    fn balance(&self, address: web3::types::H160) -> web3::types::U256 {
        self.balances
            .get(&address)
            .cloned()
            .unwrap_or(web3::types::U256::zero())
    }

    fn code_size(&self, address: web3::types::H160) -> web3::types::U256 {
        self.codes
            .get(&address)
            .map(|code| web3::types::U256::from(code.len()))
            .unwrap_or(web3::types::U256::zero())
    }

    fn code_hash(&self, address: web3::types::H160) -> web3::types::H256 {
        web3::types::H256::zero()
    }

    fn code(&self, address: web3::types::H160) -> Vec<u8> {
        self.codes.get(&address).cloned().unwrap_or_default()
    }

    fn storage(&self, address: web3::types::H160, index: web3::types::H256) -> web3::types::H256 {
        self.storages
            .get(&address)
            .and_then(|storage| storage.get(&index))
            .cloned()
            .unwrap_or_default()
    }

    fn exists(&self, address: web3::types::H160) -> bool {
        self.codes.contains_key(&address)
    }

    fn nonce(&self, address: web3::types::H160) -> web3::types::U256 {
        self.nonces.get(&address).copied().unwrap_or_default()
    }
}

impl evm::RuntimeBackend for Runtime {
    fn original_storage(
        &self,
        address: web3::types::H160,
        index: web3::types::H256,
    ) -> web3::types::H256 {
        evm::RuntimeBaseBackend::storage(self, address, index)
    }

    fn deleted(&self, address: web3::types::H160) -> bool {
        false
    }

    fn is_cold(&self, address: web3::types::H160, index: Option<web3::types::H256>) -> bool {
        false
    }

    fn is_hot(&self, address: web3::types::H160, index: Option<web3::types::H256>) -> bool {
        !self.is_cold(address, index)
    }

    fn mark_hot(&mut self, address: web3::types::H160, index: Option<web3::types::H256>) {}

    fn set_storage(
        &mut self,
        address: web3::types::H160,
        index: web3::types::H256,
        value: web3::types::H256,
    ) -> Result<(), evm::ExitError> {
        self.storages
            .entry(address)
            .and_modify(|storage| {
                storage.insert(index, value);
            })
            .or_insert_with(|| {
                let mut storage = HashMap::new();
                storage.insert(index, value);
                storage
            });
        Ok(())
    }

    fn log(&mut self, log: evm::Log) -> Result<(), evm::ExitError> {
        self.logs.push(log);
        Ok(())
    }

    fn mark_delete(&mut self, address: web3::types::H160) {}

    fn reset_storage(&mut self, address: web3::types::H160) {}

    fn set_code(
        &mut self,
        address: web3::types::H160,
        code: Vec<u8>,
    ) -> Result<(), evm::ExitError> {
        self.codes.insert(address, code);
        Ok(())
    }

    fn reset_balance(&mut self, address: web3::types::H160) {
        self.balances.insert(address, web3::types::U256::zero());
    }

    fn deposit(&mut self, target: web3::types::H160, value: web3::types::U256) {
        self.balances
            .entry(target)
            .and_modify(|balance| *balance += value)
            .or_insert(value);
    }

    fn withdrawal(
        &mut self,
        source: web3::types::H160,
        value: web3::types::U256,
    ) -> Result<(), evm::ExitError> {
        let balance = self
            .balances
            .get_mut(&source)
            .ok_or(evm::ExitError::Exception(evm::ExitException::OutOfFund))?;
        if *balance < value {
            return Err(evm::ExitError::Exception(evm::ExitException::OutOfFund));
        }
        *balance -= value;
        Ok(())
    }

    fn transfer(&mut self, transfer: evm::Transfer) -> Result<(), evm::ExitError> {
        self.withdrawal(transfer.source, transfer.value)?;
        self.deposit(transfer.target, transfer.value);
        Ok(())
    }

    fn inc_nonce(&mut self, address: web3::types::H160) -> Result<(), evm::ExitError> {
        *self
            .nonces
            .entry(address)
            .or_insert_with(web3::types::U256::zero) += web3::types::U256::one();
        Ok(())
    }
}

impl evm::TransactionalBackend for Runtime {
    fn push_substate(&mut self) {}

    fn pop_substate(&mut self, strategy: evm::MergeStrategy) {}
}
