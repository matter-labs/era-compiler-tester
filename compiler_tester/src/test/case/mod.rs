//!
//! The test case.
//!

pub mod input;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::Infallible;
use std::hash::RandomState;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

use input::revm_type_conversions::web3_u256_to_revm_address;
use input::revm_type_conversions::web3_u256_to_revm_u256;
use revm::db::states::plain_account::PlainStorage;
use revm::db::EmptyDBTyped;
use revm::db::State;
use revm::primitives::Address;
use revm::primitives::FixedBytes;
use revm::primitives::SpecId::SHANGHAI;
use revm::primitives::B256;
use revm::primitives::U256;
use revm::Database;
use revm::DatabaseCommit;
use solidity_adapter::test::params::evm_version;

use crate::compilers::mode::Mode;
use crate::directories::matter_labs::test::metadata::case::Case as MatterLabsTestCase;
use crate::summary::Summary;
use crate::test::instance::Instance;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::system_context::SystemContext;
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
    /// Runs the case on EVM.
    ///
    pub fn run_evm(
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
            input.run_evm(
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

        let mut cache = revm::CacheState::new(false);
        for input in self.inputs.iter() {
            input.add_balance(&mut cache,name.clone());
        }
        let acc_info = revm::primitives::AccountInfo {
            balance: U256::from(1_u64),
            code_hash: FixedBytes::from_str("0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470").unwrap(),
            code: None,
            nonce: 1,
        };

        cache.insert_account_with_storage(Address::from_str("0x0000000000000000000000000000000000000001").unwrap(), acc_info, PlainStorage::default());

        let mut state = revm::db::State::builder().with_cached_prestate(cache).with_bundle_update().build();
        state.block_hashes.insert(1, B256::from_str("0x3737373737373737373737373737373737373737373737373737373737373737").unwrap());
        state.block_hashes.insert(0, B256::from_str("0x3737373737373737373737373737373737373737373737373737373737373737").unwrap());
        let mut vm = revm::Evm::builder().with_db(state).modify_env(|env| {
            let evm_context = SystemContext::get_constants_evm(evm_version);
            env.cfg.chain_id = evm_context.chain_id;
            env.block.number = U256::from(evm_context.block_number);
            let coinbase = web3::types::U256::from_str_radix(evm_context.coinbase,16).unwrap();
            env.block.coinbase = web3_u256_to_revm_address(coinbase);
            env.block.timestamp = U256::from(evm_context.block_timestamp);
            env.block.gas_limit = U256::from(evm_context.block_gas_limit);
            env.block.basefee = U256::from(evm_context.base_fee);
            let block_difficulty = web3::types::U256::from_str_radix(evm_context.block_difficulty,16).unwrap();
            env.block.difficulty = web3_u256_to_revm_u256(block_difficulty);
            env.block.prevrandao = Some(B256::from(env.block.difficulty));
            env.tx.gas_price = U256::from(0xb2d05e00_u32);
            //env.tx.gas_priority_fee = ;
            //env.tx.blob_hashes = ;
            //env.tx.max_fee_per_blob_gas = 
            env.tx.gas_limit = evm_context.block_gas_limit;
            env.tx.access_list = vec![];
        }).build();

        for (index, input) in self.inputs.into_iter().enumerate() {
            vm = input.run_revm(
                summary.clone(),
                vm,
                mode.clone(),
                test_group.clone(),
                name.clone(),
                index,
                &evm_builds,
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
