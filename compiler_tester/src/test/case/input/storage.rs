//!
//! The test input storage data.
//!

use std::collections::HashMap;
use std::str::FromStr;

use crate::directories::matter_labs::test::metadata::case::input::storage::Storage as MatterLabsTestContractStorage;
use crate::test::case::input::value::Value;
use crate::test::instance::Instance;

///
/// The test input storage data.
///
#[derive(Debug, Clone, Default)]
pub struct Storage {
    /// The inner storage hashmap data.
    pub inner: HashMap<zkevm_tester::runners::compiler_tests::StorageKey, web3::types::H256>,
}

impl Storage {
    ///
    /// Try convert from Matter Labs compiler test storage data.
    ///
    pub fn try_from_matter_labs(
        storage: &HashMap<String, MatterLabsTestContractStorage>,
        instances: &HashMap<String, Instance>,
    ) -> anyhow::Result<Self> {
        let mut result_storage = HashMap::new();

        for (address, contract_storage) in storage.iter() {
            let address = if let Some(instance) = address.strip_suffix(".address") {
                instances
                    .get(instance)
                    .ok_or_else(|| anyhow::anyhow!("Instance `{}` not found", instance))?
                    .address
                    .ok_or_else(|| {
                        anyhow::anyhow!("Instance `{}` is not successfully deployed", instance)
                    })
            } else {
                web3::types::Address::from_str(address)
                    .map_err(|error| anyhow::anyhow!("Invalid address literal: {}", error))
            }
            .map_err(|error| anyhow::anyhow!("Invalid storage address: {}", error))?;

            let contract_storage = match contract_storage {
                MatterLabsTestContractStorage::List(list) => list
                    .iter()
                    .enumerate()
                    .map(|(key, value)| (key.to_string(), value.clone()))
                    .collect(),
                MatterLabsTestContractStorage::Map(map) => map.clone(),
            };
            for (key, value) in contract_storage.into_iter() {
                let key = match Value::try_from_matter_labs(key.as_str(), instances)
                    .map_err(|error| anyhow::anyhow!("Invalid storage key: {}", error))?
                {
                    Value::Certain(value) => value,
                    Value::Any => anyhow::bail!("Storage key can not be `*`"),
                };
                let key = zkevm_tester::runners::compiler_tests::StorageKey { address, key };

                let value = match Value::try_from_matter_labs(value.as_str(), instances)
                    .map_err(|error| anyhow::anyhow!("Invalid storage value: {}", error))?
                {
                    Value::Certain(value) => value,
                    Value::Any => anyhow::bail!("Storage value can not be `*`"),
                };

                let mut value_bytes = [0u8; compiler_common::BYTE_LENGTH_FIELD];
                value.to_big_endian(value_bytes.as_mut_slice());
                let value = web3::types::H256::from(value_bytes);

                result_storage.insert(key, value);
            }
        }

        Ok(Self {
            inner: result_storage,
        })
    }
}
