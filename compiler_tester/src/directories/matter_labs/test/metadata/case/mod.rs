//!
//! The Matter Labs compiler test metadata case.
//!

pub mod input;

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Deserialize;

use crate::compilers::mode::Mode;
use crate::target::Target;
use crate::test::instance::Instance;
use crate::vm::address_iterator::AddressIterator;
use crate::vm::eravm::address_iterator::EraVMAddressIterator;
use crate::vm::evm::address_iterator::EVMAddressIterator;

use self::input::expected::Expected;
use self::input::Input;

///
/// The Matter Labs compiler test metadata case.
///
#[derive(Debug, Clone, Deserialize)]
pub struct Case {
    /// The comment to a case.
    pub comment: Option<String>,
    /// The case name.
    pub name: String,
    /// The mode filter.
    pub modes: Option<Vec<String>>,
    /// The case inputs.
    pub inputs: Vec<Input>,
    /// The expected return data.
    pub expected: Expected,
    /// If the test case must be ignored.
    #[serde(default)]
    pub ignore: bool,
    /// Overrides the default number of cycles.
    pub cycles: Option<usize>,
}

impl Case {
    ///
    /// Normalizes the case.
    ///
    pub fn normalize(
        mut self,
        contracts: &BTreeMap<String, String>,
        instances: &BTreeMap<String, Instance>,
        target: Target,
    ) -> anyhow::Result<Self> {
        self.normalize_deployer_calls(contracts, instances, target)?;
        self.normalize_expected();
        Ok(self)
    }

    ///
    /// Validates deployer calls, adds libraries deployer calls, contracts deployer calls if they are not present.
    ///
    pub fn normalize_deployer_calls(
        &mut self,
        contracts: &BTreeMap<String, String>,
        instances: &BTreeMap<String, Instance>,
        target: Target,
    ) -> anyhow::Result<()> {
        let mut contracts = contracts.clone();
        for (index, input) in self.inputs.iter().enumerate() {
            if input.method.as_str() != "#deployer" {
                continue;
            };

            if contracts.remove(input.instance.as_str()).is_none() {
                anyhow::bail!(
                    "Input {} is a second deployer call for the same instance or instance is invalid",
                    index
                );
            }
        }

        let mut inputs = Vec::with_capacity(instances.len() + self.inputs.len());

        for (name, instance) in instances.iter() {
            if instance.is_library() {
                inputs.push(Input::empty_deployer_call(name.to_owned()));
            }
        }

        for contract in contracts.keys() {
            if !instances
                .iter()
                .any(|(filter_name, instance)| filter_name == contract && instance.is_library())
            {
                inputs.push(Input::empty_deployer_call(contract.clone()));
            }
        }

        if let Target::EraVM = target {
            for (name, instance) in instances.iter() {
                if let Instance::EraVM { .. } = instance {
                    continue;
                }

                if name != "Benchmark"
                    && name.split('_').next().unwrap_or_default()
                        != self.name.split('_').next().unwrap_or_default()
                {
                    continue;
                }

                if !instances
                    .iter()
                    .any(|(filter_name, instance)| filter_name == name && instance.is_library())
                {
                    inputs.push(Input::empty_deployer_call(name.to_owned()));
                }
            }
        }

        inputs.append(&mut self.inputs);
        self.inputs = inputs;

        Ok(())
    }

    ///
    /// Copies the final expected data to the last input.
    ///
    pub fn normalize_expected(&mut self) {
        if let Some(input) = self.inputs.last_mut() {
            if input.expected.is_none() {
                input.expected = Some(self.expected.clone());
            }
        }
    }

    ///
    /// Returns all the instances addresses, except libraries.
    ///
    pub fn set_instance_addresses(
        &self,
        instances: &mut BTreeMap<String, Instance>,
        mut eravm_address_iterator: EraVMAddressIterator,
        mut evm_address_iterator: EVMAddressIterator,
        mode: &Mode,
    ) -> anyhow::Result<()> {
        for (index, input) in self.inputs.iter().enumerate() {
            if input.method.as_str() != "#deployer"
                || instances.iter().any(|(name, instance)| {
                    name.as_str() == input.instance.as_str() && instance.is_library()
                })
            {
                continue;
            }

            let exception = match input.expected.as_ref() {
                Some(expected) => expected
                    .exception(mode)
                    .map_err(|error| anyhow::anyhow!("Input #{}: {}", index, error))?,
                None => false,
            };
            if exception {
                continue;
            }

            let caller =
                web3::types::Address::from_str(input.caller.as_str()).map_err(|error| {
                    anyhow::anyhow!(
                        "Input #{} has invalid caller `{}`: {}",
                        index,
                        input.caller.as_str(),
                        error
                    )
                })?;

            match instances.get_mut(input.instance.as_str()) {
                Some(instance @ Instance::EraVM(_)) => {
                    instance.set_address(eravm_address_iterator.next(&caller, true));
                }
                Some(instance @ Instance::EVM(_)) => {
                    instance.set_address(evm_address_iterator.next(&caller, true));
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}
