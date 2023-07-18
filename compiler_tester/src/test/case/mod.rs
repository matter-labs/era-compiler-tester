//!
//! The test case.
//!

pub mod input;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::deployers::Deployer;
use crate::directories::matter_labs::test::metadata::case::Case as MatterLabsTestCase;
use crate::summary::Summary;
use crate::test::instance::Instance;
use crate::zkevm::zkEVM;

use self::input::Input;

///
/// The test case.
///
#[derive(Debug, Clone)]
pub struct Case {
    /// The case name.
    name: Option<String>,
    /// The case inputs.
    inputs: Vec<Input>,
}

impl Case {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(name: Option<String>, inputs: Vec<Input>) -> Self {
        Self { name, inputs }
    }

    ///
    /// Try convert from Matter Labs compiler test metadata case.
    ///
    pub fn try_from_matter_labs(
        case: &MatterLabsTestCase,
        mode: &Mode,
        instances: &HashMap<String, Instance>,
        method_identifiers: &Option<BTreeMap<String, BTreeMap<String, u32>>>,
    ) -> anyhow::Result<Self> {
        let mut inputs = Vec::with_capacity(case.inputs.capacity());

        for (index, input) in case.inputs.iter().enumerate() {
            let input = Input::try_from_matter_labs(input, mode, instances, method_identifiers)
                .map_err(|error| {
                    anyhow::anyhow!(
                        "Input {}(After adding deployer calls) is invalid: {}",
                        index,
                        error
                    )
                })?;
            inputs.push(input);
        }

        Ok(Self::new(Some(case.name.clone()), inputs))
    }

    ///
    /// Try convert from Ethereum compiler test metadata case.
    ///
    pub fn try_from_ethereum(
        case: &[solidity_adapter::FunctionCall],
        main_contract_instance: &Instance,
        libraries_instances: &HashMap<String, Instance>,
        last_source: &str,
    ) -> anyhow::Result<Self> {
        let mut inputs = Vec::new();
        let mut caller = solidity_adapter::account_address(solidity_adapter::DEFAULT_ACCOUNT_INDEX);

        for (index, input) in case.iter().enumerate() {
            match input {
                solidity_adapter::FunctionCall::Account { input, .. } => {
                    caller = solidity_adapter::account_address(*input);
                }
                input => {
                    if let Some(input) = Input::try_from_ethereum(
                        input,
                        main_contract_instance,
                        libraries_instances,
                        last_source,
                        &caller,
                    )
                    .map_err(|error| {
                        anyhow::anyhow!("Failed to proccess {} input: {}", index, error)
                    })? {
                        inputs.push(input)
                    }
                }
            }
        }

        Ok(Self { name: None, inputs })
    }

    ///
    /// Run the case.
    ///
    pub fn run<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        initial_vm: zkEVM,
        mode: &Mode,
        test_name: String,
        test_group: Option<String>,
    ) where
        D: Deployer,
    {
        let name = if let Some(case_name) = self.name {
            format!("{test_name}::{case_name}")
        } else {
            test_name
        };
        let mut vm = initial_vm;
        let mut deployer = D::new();
        for (index, input) in self.inputs.into_iter().enumerate() {
            input.run::<_, M>(
                summary.clone(),
                &mut vm,
                mode.clone(),
                &mut deployer,
                test_group.clone(),
                name.clone(),
                index,
            )
        }
    }
}
