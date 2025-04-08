//!
//! The EVM compiler input.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::test::instance::Instance;

///
/// The EVM compiler input.
///
#[derive(Debug)]
pub struct Input {
    /// The contract builds.
    pub builds: HashMap<String, Vec<u8>>,
    /// The contracts method identifiers.
    pub method_identifiers: Option<BTreeMap<String, BTreeMap<String, u32>>>,
    /// The last contract name.
    pub last_contract: String,
}

impl Input {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        builds: HashMap<String, Vec<u8>>,
        method_identifiers: Option<BTreeMap<String, BTreeMap<String, u32>>>,
        last_contract: String,
    ) -> Self {
        Self {
            builds,
            method_identifiers,
            last_contract,
        }
    }

    ///
    /// Returns all contract instances.
    ///
    pub fn get_instances(
        &self,
        contracts: &BTreeMap<String, String>,
        library_addresses: BTreeMap<String, web3::types::Address>,
        main_address: Option<web3::types::Address>,
    ) -> anyhow::Result<BTreeMap<String, Instance>> {
        let mut instances = BTreeMap::new();

        for (name, address) in library_addresses.into_iter() {
            let build = self.builds.get(name.as_str()).ok_or_else(|| {
                anyhow::anyhow!("Library `{name}` not found in the build artifacts")
            })?;

            instances.insert(
                name.clone(),
                Instance::evm(name, Some(address), false, true, build.to_owned()),
            );
        }

        if contracts.is_empty() {
            let main_contract_build =
                self.builds
                    .get(self.last_contract.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Main contract not found in the compiler build artifacts")
                    })?;

            instances.insert(
                "Test".to_owned(),
                Instance::evm(
                    self.last_contract.to_owned(),
                    main_address,
                    true,
                    false,
                    main_contract_build.to_owned(),
                ),
            );
        } else {
            for (instance, path) in contracts.iter() {
                let build = self.builds.get(path.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("{path} not found in the compiler build artifacts")
                })?;
                let is_main = path.as_str() == self.last_contract.as_str();

                instances.insert(
                    instance.to_owned(),
                    Instance::evm(path.to_owned(), None, is_main, false, build.to_owned()),
                );
            }
        }

        Ok(instances)
    }
}
