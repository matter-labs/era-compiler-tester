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

            let mut deploy_code = build.deploy_build.to_owned();
            deploy_code.extend_from_slice(build.runtime_build.as_slice());

            instances.insert(
                name.clone(),
                Instance::evm(
                    name,
                    Some(address),
                    false,
                    true,
                    deploy_code,
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

            let mut deploy_code = main_contract_build.deploy_build.to_owned();
            deploy_code.extend_from_slice(main_contract_build.runtime_build.as_slice());

            instances.insert(
                "Test".to_owned(),
                Instance::evm(
                    self.last_contract.to_owned(),
                    main_address,
                    true,
                    false,
                    deploy_code,
                ),
            );
        } else {
            for (instance, path) in contracts.iter() {
                let build = self.builds.get(path.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("{} not found in the compiler build artifacts", path)
                })?;
                let is_main = path.as_str() == self.last_contract.as_str();

                let mut deploy_code = build.deploy_build.to_owned();
                deploy_code.extend_from_slice(build.runtime_build.as_slice());

                instances.insert(
                    instance.to_owned(),
                    Instance::evm(
                        path.to_owned(),
                        None,
                        is_main,
                        false,
                        deploy_code,
                    ),
                );
            }
        }

        Ok(instances)
    }
}
