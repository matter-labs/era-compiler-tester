//!
//! The Matter Labs compiler test metadata case.
//!

pub mod input;

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::str::FromStr;

use serde::Deserialize;

use crate::compilers::mode::Mode;
use crate::vm::AddressPredictorIterator;

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
    /// Validates deployer calls, adds libraries deployer calls, contracts deployer calls if they are not present.
    ///
    pub fn normalize_deployer_calls(
        &mut self,
        instances: &BTreeSet<String>,
        libraries: &[String],
    ) -> anyhow::Result<()> {
        let mut instances = instances.clone();
        for (index, input) in self.inputs.iter().enumerate() {
            let instance = &input.instance;
            if !input.method.eq("#deployer") {
                continue;
            };

            if libraries.contains(instance) {
                anyhow::bail!("Deployer call {} for library, note: libraries deployer calls generating automatically", index);
            }

            if !instances.remove(instance) {
                anyhow::bail!(
                    "Input {} is a second deployer call for the same instance or instance is invalid",
                    index
                );
            }
        }

        let mut inputs = Vec::with_capacity(libraries.len() + instances.len() + self.inputs.len());

        for instance in libraries.iter() {
            inputs.push(Input::empty_deployer_call(instance.clone()));
        }

        for instance in instances {
            if !libraries.contains(&instance) {
                inputs.push(Input::empty_deployer_call(instance.clone()));
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
    pub fn instance_addresses<API>(
        &self,
        libraries: &BTreeSet<String>,
        address_predictor: &mut API,
        mode: &Mode,
    ) -> anyhow::Result<HashMap<String, web3::types::Address>>
    where
        API: AddressPredictorIterator,
    {
        let mut instances_addresses = HashMap::new();
        for (index, input) in self.inputs.iter().enumerate() {
            if !input.method.eq("#deployer") {
                continue;
            }
            let instance = &input.instance;
            if libraries.contains(instance) {
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
            let caller = web3::types::Address::from_str(input.caller.as_str())
                .map_err(|error| anyhow::anyhow!("Input #{}: invalid caller: {}", index, error))?;
            instances_addresses.insert(instance.to_string(), address_predictor.next(&caller, true));
        }
        Ok(instances_addresses)
    }
}
