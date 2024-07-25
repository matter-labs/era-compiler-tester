//!
//! The EraVM compiler input.
//!

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::test::instance::Instance;

///
/// The EraVM compiler input.
///
#[derive(Debug, Clone)]
pub struct Input {
    /// The contract builds.
    pub builds: HashMap<String, era_compiler_llvm_context::EraVMBuild>,
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
        builds: HashMap<String, era_compiler_llvm_context::EraVMBuild>,
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
        main_address: web3::types::Address,
    ) -> anyhow::Result<BTreeMap<String, Instance>> {
        let mut instances = BTreeMap::new();

        for (name, address) in library_addresses.into_iter() {
            let build = self.builds.get(name.as_str()).ok_or_else(|| {
                anyhow::anyhow!("Library `{}` not found in the build artifacts", name)
            })?;
            let code_hash = web3::types::U256::from_big_endian(build.bytecode_hash.as_slice());

            instances.insert(
                name.clone(),
                Instance::eravm(name, Some(address), false, true, code_hash),
            );
        }

        if contracts.is_empty() {
            let main_contract_build =
                self.builds
                    .get(self.last_contract.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Main contract not found in the compiler build artifacts")
                    })?;
            let code_hash =
                web3::types::U256::from_big_endian(main_contract_build.bytecode_hash.as_slice());

            instances.insert(
                "Test".to_owned(),
                Instance::eravm(
                    self.last_contract.to_owned(),
                    Some(main_address),
                    true,
                    false,
                    code_hash,
                ),
            );
        } else {
            for (instance, path) in contracts.iter() {
                let build = self.builds.get(path.as_str()).ok_or_else(|| {
                    anyhow::anyhow!("{} not found in the compiler build artifacts", path)
                })?;
                let code_hash = web3::types::U256::from_big_endian(build.bytecode_hash.as_slice());

                instances.insert(
                    instance.to_owned(),
                    Instance::eravm(path.to_owned(), None, false, false, code_hash),
                );
            }
        }

        Ok(instances)
    }
}
