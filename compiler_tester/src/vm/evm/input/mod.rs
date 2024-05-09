//!
//! The EVM compiler input.
//!

pub mod build;

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::test::instance::Instance;

use self::build::Build;

///
/// The EVM compiler input.
///
#[derive(Debug)]
pub struct Input {
    /// The contract builds.
    pub builds: HashMap<String, Build>,
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
        builds: HashMap<String, Build>,
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
                anyhow::anyhow!("Library `{}` not found in the build artifacts", name)
            })?;

            instances.insert(
                name.clone(),
                Instance::evm(
                    name,
                    Some(address),
                    false,
                    true,
                    build.deploy_build.bytecode.to_owned(),
                ),
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
                    main_contract_build.deploy_build.bytecode.to_owned(),
                ),
            );
        } else {
            for (instance, path) in contracts.iter() {
                let build = self.builds.get(path.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("{} not found in the compiler build artifacts", path)
                })?;
                let is_main = path.as_str() == self.last_contract.as_str();
                instances.insert(
                    instance.to_owned(),
                    Instance::evm(
                        path.to_owned(),
                        None,
                        is_main,
                        false,
                        build.deploy_build.bytecode.to_owned(),
                    ),
                );
            }
        }

        Ok(instances)
    }
}
