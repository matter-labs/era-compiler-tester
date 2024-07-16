//!
//! The test case.
//!

pub mod input;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::RandomState;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use revm::db::states::plain_account::PlainStorage;
use revm::db::EmptyDBTyped;
use revm::primitives::Address;
use revm::primitives::FixedBytes;
use revm::primitives::B256;
use revm::primitives::U256;
use solidity_adapter::test::params::evm_version;

use crate::compilers::mode::Mode;
use crate::directories::matter_labs::test::metadata::case::Case as MatterLabsTestCase;
use crate::summary::Summary;
use crate::target;
use crate::test::instance::Instance;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::evm::input::build::Build;
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
        target: &target::Target,
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
                        Input::try_from_ethereum(input, &instances, last_source, &caller, target)
                            .map_err(|error| {
                                anyhow::anyhow!("Failed to proccess input #{index}: {error}")
                            })?
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
    /// Runs the case on REVM.
    ///
    pub fn run_revm(
        self,
        summary: Arc<Mutex<Summary>>,
        mode: &Mode,
        test_name: String,
        test_group: Option<String>,
        evm_builds: HashMap<String, Build, RandomState>,
        evm_version: Option<evm_version::EVMVersion>,
    ) {
        let name = if let Some(case_name) = self.name {
            format!("{test_name}::{case_name}")
        } else {
            test_name
        };

        let mut vm = create_initial_revm();
        for (index, input) in self.inputs.into_iter().enumerate() {
            vm = input.run_revm(
                summary.clone(),
                vm,
                mode.clone(),
                test_group.clone(),
                name.clone(),
                index,
                &evm_builds,
                evm_version,
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

pub fn create_initial_revm() -> revm::Evm<'static, (), revm::db::State<EmptyDBTyped<std::convert::Infallible>>> {
    let mut cache = revm::CacheState::new(false);
    // Precompile 0x01 needs to have its code hash
    let acc_info = revm::primitives::AccountInfo {
        balance: U256::from(1_u64),
        code_hash: FixedBytes::from_str(
            "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
        )
        .unwrap(),
        code: None,
        nonce: 1,
    };

    cache.insert_account_with_storage(
        Address::from_word(FixedBytes::from(U256::from(1_u64))),
        acc_info,
        PlainStorage::default(),
    );

    // Account 0x00 needs to have its code hash on 0
    let acc_info_zero = revm::primitives::AccountInfo {
        balance: U256::from(0_u64),
        code_hash: FixedBytes::from(U256::ZERO),
        code: None,
        nonce: 1,
    };

    cache.insert_account_with_storage(
        Address::from_word(FixedBytes::from(U256::ZERO)),
        acc_info_zero,
        PlainStorage::default(),
    );

    let mut state = revm::db::State::builder()
        .with_cached_prestate(cache)
        .with_bundle_update()
        .build();

    // Blocks 0 and 1 need to have their hashes set (revm by default just uses the keccak of the number)
    state.block_hashes.insert(
        1,
        B256::from_str("0x3737373737373737373737373737373737373737373737373737373737373737")
            .unwrap(),
    );
    state.block_hashes.insert(
        0,
        B256::from_str("0x3737373737373737373737373737373737373737373737373737373737373737")
            .unwrap(),
    );

    revm::Evm::builder().with_db(state).build()
}
