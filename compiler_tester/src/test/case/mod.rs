//!
//! The test case.
//!

pub mod input;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::directories::matter_labs::test::metadata::case::Case as MatterLabsTestCase;
use crate::summary::Summary;
use crate::test::instance::Instance;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::evm::EVM;

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
        case: MatterLabsTestCase,
        mode: &Mode,
        instances: &BTreeMap<String, Instance>,
        method_identifiers: &Option<BTreeMap<String, BTreeMap<String, u32>>>,
    ) -> anyhow::Result<Self> {
        let mut inputs = Vec::with_capacity(case.inputs.len());

        for (index, input) in case.inputs.into_iter().enumerate() {
            let input = Input::try_from_matter_labs(input, mode, instances, method_identifiers)
                .map_err(|error| anyhow::anyhow!("Input #{} is invalid: {}", index, error))?;
            inputs.push(input);
        }

        Ok(Self::new(Some(case.name), inputs))
    }

    ///
    /// Try convert from Ethereum compiler test metadata case.
    ///
    pub fn try_from_ethereum(
        case: &[solidity_adapter::FunctionCall],
        instances: BTreeMap<String, Instance>,
        last_source: &str,
    ) -> anyhow::Result<Self> {
        let mut inputs = Vec::with_capacity(case.len());
        let mut caller = solidity_adapter::account_address(solidity_adapter::DEFAULT_ACCOUNT_INDEX);

        for (index, input) in case.iter().enumerate() {
            match input {
                solidity_adapter::FunctionCall::Account { input, .. } => {
                    caller = solidity_adapter::account_address(*input);
                }
                input => {
                    if let Some(input) =
                        Input::try_from_ethereum(input, &instances, last_source, &caller).map_err(
                            |error| anyhow::anyhow!("Failed to proccess input #{index}: {error}"),
                        )?
                    {
                        inputs.push(input);
                    }
                }
            }
        }

        Ok(Self::new(None, inputs))
    }

    ///
    /// Runs the case on EraVM.
    ///
    pub fn run_eravm<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        mut vm: EraVM,
        mode: &Mode,
        test_name: String,
        test_group: Option<String>,
    ) where
        D: EraVMDeployer,
    {
        let name = if let Some(case_name) = self.name {
            format!("{test_name}::{case_name}")
        } else {
            test_name
        };

        for (index, input) in self.inputs.into_iter().enumerate() {
            input.run_eravm::<_, M>(
                summary.clone(),
                &mut vm,
                mode.to_owned(),
                &mut D::new(),
                test_group.clone(),
                name.clone(),
                index,
            )
        }
    }

    ///
    /// Runs the case on EVM emulator.
    ///
    pub fn run_evm_emulator(
        self,
        summary: Arc<Mutex<Summary>>,
        mut vm: EVM,
        mode: &Mode,
        test_name: String,
        test_group: Option<String>,
    ) {
        let name = if let Some(case_name) = self.name {
            format!("{test_name}::{case_name}")
        } else {
            test_name
        };

        for (index, input) in self.inputs.into_iter().enumerate() {
            input.run_evm_emulator(
                summary.clone(),
                &mut vm,
                mode.clone(),
                test_group.clone(),
                name.clone(),
                index,
            )
        }
    }

    ///
    /// Runs the case on EVM interpreter.
    ///
    pub fn run_evm_interpreter<D, const M: bool>(
        self,
        summary: Arc<Mutex<Summary>>,
        mut vm: EraVM,
        mode: &Mode,
        test_name: String,
        test_group: Option<String>,
    ) where
        D: EraVMDeployer,
    {
        let name = if let Some(case_name) = self.name {
            format!("{test_name}::{case_name}")
        } else {
            test_name
        };

        for (index, input) in self.inputs.into_iter().enumerate() {
            input.run_evm_interpreter::<_, M>(
                summary.clone(),
                &mut vm,
                mode.clone(),
                &mut D::new(),
                test_group.clone(),
                name.clone(),
                index,
            )
        }
    }
}
