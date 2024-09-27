//!
//! The EraVM dummy deployer implementation.
//!

use std::collections::HashMap;

use web3::contract::tokens::Tokenizable;

use crate::test::case::input::output::Output;
use crate::test::case::input::value::Value;
use crate::vm::address_iterator::AddressIterator;
use crate::vm::eravm::address_iterator::EraVMAddressIterator;
use crate::vm::eravm::deployers::EraVMDeployer;
use crate::vm::eravm::EraVM;
use crate::vm::execution_result::ExecutionResult;

///
/// The EraVM dummy deployer implementation.
///
#[derive(Debug, Clone)]
pub struct DummyDeployer {
    /// The address iterator instance for computing the contracts addresses.
    address_iterator: EraVMAddressIterator,
}

impl DummyDeployer {
    /// The immutables mapping position in contract.
    const IMMUTABLES_MAPPING_POSITION: web3::types::U256 = web3::types::U256::zero();
}

impl EraVMDeployer for DummyDeployer {
    fn new() -> Self {
        Self {
            address_iterator: EraVMAddressIterator::new(),
        }
    }

    fn deploy_eravm<const M: bool>(
        &mut self,
        test_name: String,
        caller: web3::types::Address,
        bytecode_hash: web3::types::U256,
        constructor_calldata: Vec<u8>,
        value: Option<u128>,
        vm: &mut EraVM,
    ) -> anyhow::Result<ExecutionResult> {
        let address = self.address_iterator.next(&caller, false);

        vm.add_deployed_contract(address, bytecode_hash, None);

        let context_u128_value = if let Some(value) = value {
            vm.mint_ether(address, web3::types::U256::from(value));
            value
        } else {
            0
        };

        let result = vm.execute::<M>(
            test_name,
            address,
            caller,
            Some(context_u128_value),
            constructor_calldata,
            Some(zkevm_tester::compiler_tests::VmLaunchOption::ManualCallABI(
                zkevm_tester::compiler_tests::FullABIParams {
                    is_constructor: true,
                    is_system_call: false,
                    r3_value: None,
                    r4_value: None,
                    r5_value: None,
                },
            )),
        )?;

        if result.output.exception {
            if let Some(value) = value {
                vm.burn_ether(address, web3::types::U256::from(value));
            }
            vm.remove_deployed_contract(address);
            return Ok(result);
        }

        self.address_iterator.increment_nonce(&caller);

        Self::set_immutables(address, &result.output.return_data, vm)?;

        let return_data = vec![Value::Certain(web3::types::U256::from_big_endian(
            address.as_bytes(),
        ))];

        Ok(ExecutionResult::new(
            Output::new(return_data, false, result.output.events),
            result.cycles,
            result.ergs,
            result.gas,
        ))
    }

    fn deploy_evm<const M: bool>(
        &mut self,
        _test_name: String,
        _caller: web3::types::Address,
        _init_code: Vec<u8>,
        _constructor_calldata: Vec<u8>,
        _value: Option<u128>,
        _vm: &mut EraVM,
    ) -> anyhow::Result<ExecutionResult> {
        todo!()
    }
}

impl DummyDeployer {
    ///
    /// Writes the contract immutables to a storage.
    ///
    fn set_immutables(
        address: web3::types::Address,
        encoded_data: &[Value],
        vm: &mut EraVM,
    ) -> anyhow::Result<()> {
        let return_data = encoded_data
            .iter()
            .flat_map(|value| {
                let mut bytes = [0u8; era_compiler_common::BYTE_LENGTH_FIELD];
                value.unwrap_certain_as_ref().to_big_endian(&mut bytes);
                bytes
            })
            .collect::<Vec<u8>>();

        let r#type =
            web3::ethabi::ParamType::Array(Box::new(web3::ethabi::ParamType::Tuple(vec![
                web3::ethabi::ParamType::Uint(256),
                web3::ethabi::ParamType::FixedBytes(32),
            ])));
        let mut immutables = web3::ethabi::decode(&[r#type], return_data.as_slice())
            .map_err(|err| anyhow::anyhow!("Failed to decode immutables: {:?}", err))?;

        assert_eq!(immutables.len(), 1);
        let immutables = match immutables.remove(0) {
            web3::ethabi::Token::Array(immutables) => immutables,
            _ => unreachable!(),
        };

        let mut immutables_storage = HashMap::new();
        for immutable in immutables {
            let (immutable_index, immutable_value) = match immutable {
                web3::ethabi::Token::Tuple(elements) => {
                    assert_eq!(elements.len(), 2);
                    let mut elements_iter = elements.into_iter();
                    (
                        elements_iter.next().expect("Always valid"),
                        elements_iter.next().expect("Always valid"),
                    )
                }
                _ => unreachable!(),
            };

            let immutable_index =
                web3::types::U256::from_token(immutable_index).expect("Always valid");
            let immutable_value =
                web3::types::H256::from_token(immutable_value).expect("Always valid");

            let immutable_position = Self::get_position_of_immutable(address, immutable_index);

            let address = web3::types::Address::from_low_u64_be(
                zkevm_opcode_defs::ADDRESS_IMMUTABLE_SIMULATOR.into(),
            );
            let key = immutable_position;

            immutables_storage.insert((address, key), immutable_value);
        }

        vm.populate_storage(immutables_storage);

        Ok(())
    }

    ///
    /// Returns the immutable position in the contract storage.
    ///
    fn get_position_of_immutable(
        address: web3::types::Address,
        index: web3::types::U256,
    ) -> web3::types::U256 {
        let mut key = web3::types::H256::from(address).to_fixed_bytes().to_vec();
        key.extend([0u8; era_compiler_common::BYTE_LENGTH_FIELD]);
        Self::IMMUTABLES_MAPPING_POSITION
            .to_big_endian(&mut key[era_compiler_common::BYTE_LENGTH_FIELD..]);
        let key = web3::signing::keccak256(key.as_slice()).to_vec();

        let mut nested_key = vec![0u8; era_compiler_common::BYTE_LENGTH_FIELD];
        index.to_big_endian(&mut nested_key[..]);
        nested_key.extend(key);
        let nested_key = web3::signing::keccak256(nested_key.as_slice());

        web3::types::U256::from(nested_key)
    }
}
