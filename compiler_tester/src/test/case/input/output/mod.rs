//!
//! The compiler test outcome data.
//!

pub mod event;

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Serialize;

use crate::compilers::mode::Mode;
use crate::directories::matter_labs::test::metadata::case::input::expected::variant::Variant as MatterLabsTestExpectedVariant;
use crate::directories::matter_labs::test::metadata::case::input::expected::Expected as MatterLabsTestExpected;
use crate::test::case::input::value::Value;
use crate::test::instance::Instance;
use crate::vm::evm::output::Output as EVMOutput;

use self::event::Event;

///
/// The compiler test outcome data.
///
#[derive(Debug, Default, Serialize, Clone)]
pub struct Output {
    /// The return data values.
    pub return_data: Vec<Value>,
    /// Whether an exception is thrown,
    pub exception: bool,
    /// The emitted events.
    pub events: Vec<Event>,
}

impl Output {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(return_data: Vec<Value>, exception: bool, events: Vec<Event>) -> Self {
        Self {
            return_data,
            exception,
            events,
        }
    }

    ///
    /// Try convert from Matter Labs compiler test metadata expected.
    ///
    pub fn try_from_matter_labs_expected(
        expected: MatterLabsTestExpected,
        mode: &Mode,
        instances: &BTreeMap<String, Instance>,
    ) -> anyhow::Result<Self> {
        let variants = match expected {
            MatterLabsTestExpected::Single(variant) => vec![variant],
            MatterLabsTestExpected::Multiple(variants) => variants.into_iter().collect(),
        };
        let variant = variants
            .into_iter()
            .find(|variant| {
                let version = match variant {
                    MatterLabsTestExpectedVariant::Simple(_) => None,
                    MatterLabsTestExpectedVariant::Extended(inner) => {
                        inner.compiler_version.as_ref()
                    }
                };
                match version {
                    Some(version) => mode.check_version(version),
                    None => true,
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Version not covered"))?;

        let (return_data, exception, events) = match variant {
            MatterLabsTestExpectedVariant::Simple(return_data) => (return_data, false, Vec::new()),
            MatterLabsTestExpectedVariant::Extended(expected) => {
                let return_data = expected.return_data;
                let exception = expected.exception;
                let events = expected
                    .events
                    .into_iter()
                    .enumerate()
                    .map(|(index, event)| {
                        Event::try_from_matter_labs(event, instances).map_err(|error| {
                            anyhow::anyhow!("Event #{} is invalid: {}", index, error)
                        })
                    })
                    .collect::<anyhow::Result<Vec<Event>>>()
                    .map_err(|error| anyhow::anyhow!("Invalid events: {}", error))?;
                (return_data, exception, events)
            }
        };
        let return_data = Value::try_from_vec_matter_labs(return_data, instances)
            .map_err(|error| anyhow::anyhow!("Invalid return data: {error}"))?;

        Ok(Self {
            return_data,
            exception,
            events,
        })
    }

    ///
    /// Convert from Ethereum compiler test metadata expected.
    ///
    pub fn from_ethereum_expected(
        expected: &[web3::types::U256],
        exception: bool,
        events: &[solidity_adapter::Event],
        contract_address: &web3::types::Address,
    ) -> Self {
        let return_data = expected
            .iter()
            .map(|value| {
                let mut value_str = crate::utils::u256_as_string(value);
                value_str = value_str.replace(
                    solidity_adapter::DEFAULT_CONTRACT_ADDRESS,
                    &crate::utils::address_as_string(contract_address),
                );
                Value::Certain(
                    web3::types::U256::from_str(&value_str)
                        .expect("Solidity adapter default contract address constant is invalid"),
                )
            })
            .collect();

        let events = events
            .iter()
            .map(|event| Event::from_ethereum(event, contract_address))
            .collect();

        Self {
            return_data,
            exception,
            events,
        }
    }
}

impl From<web3::types::U256> for Output {
    fn from(value: web3::types::U256) -> Self {
        Self {
            return_data: vec![Value::Certain(value)],
            exception: false,
            events: vec![],
        }
    }
}

impl From<bool> for Output {
    fn from(value: bool) -> Self {
        let value = if value {
            web3::types::U256::one()
        } else {
            web3::types::U256::zero()
        };
        value.into()
    }
}

impl From<zkevm_tester::compiler_tests::VmSnapshot> for Output {
    fn from(snapshot: zkevm_tester::compiler_tests::VmSnapshot) -> Self {
        let events = snapshot
            .events
            .into_iter()
            .filter(|event| {
                let first_topic = event.topics.first().expect("Always exists");
                let address = crate::utils::bytes32_to_address(first_topic);
                address
                    >= web3::types::Address::from_low_u64_be(
                        zkevm_opcode_defs::ADDRESS_UNRESTRICTED_SPACE,
                    )
            })
            .map(Event::from)
            .collect();

        match snapshot.execution_result {
            zkevm_tester::compiler_tests::VmExecutionResult::Ok(return_data) => {
                let return_data = return_data
                    .chunks(era_compiler_common::BYTE_LENGTH_FIELD)
                    .map(|word| {
                        let value = if word.len() != era_compiler_common::BYTE_LENGTH_FIELD {
                            let mut word_padded = word.to_vec();
                            word_padded.extend(vec![
                                0u8;
                                era_compiler_common::BYTE_LENGTH_FIELD
                                    - word.len()
                            ]);
                            web3::types::U256::from_big_endian(word_padded.as_slice())
                        } else {
                            web3::types::U256::from_big_endian(word)
                        };
                        Value::Certain(value)
                    })
                    .collect();

                Self {
                    return_data,
                    exception: false,
                    events,
                }
            }
            zkevm_tester::compiler_tests::VmExecutionResult::Revert(return_data) => {
                let return_data = return_data
                    .chunks(era_compiler_common::BYTE_LENGTH_FIELD)
                    .map(|word| {
                        let value = if word.len() != era_compiler_common::BYTE_LENGTH_FIELD {
                            let mut word_padded = word.to_vec();
                            word_padded.extend(vec![
                                0u8;
                                era_compiler_common::BYTE_LENGTH_FIELD
                                    - word.len()
                            ]);
                            web3::types::U256::from_big_endian(word_padded.as_slice())
                        } else {
                            web3::types::U256::from_big_endian(word)
                        };
                        Value::Certain(value)
                    })
                    .collect();

                Self {
                    return_data,
                    exception: true,
                    events,
                }
            }
            zkevm_tester::compiler_tests::VmExecutionResult::Panic => Self {
                return_data: vec![],
                exception: true,
                events,
            },
            zkevm_tester::compiler_tests::VmExecutionResult::MostLikelyDidNotFinish { .. } => {
                Self {
                    return_data: vec![],
                    exception: true,
                    events,
                }
            }
        }
    }
}

impl From<EVMOutput> for Output {
    fn from(output: EVMOutput) -> Self {
        let return_data = output
            .return_data
            .chunks(era_compiler_common::BYTE_LENGTH_FIELD)
            .map(|word| {
                let value = if word.len() != era_compiler_common::BYTE_LENGTH_FIELD {
                    let mut word_padded = word.to_vec();
                    word_padded.extend(vec![
                        0u8;
                        era_compiler_common::BYTE_LENGTH_FIELD - word.len()
                    ]);
                    web3::types::U256::from_big_endian(word_padded.as_slice())
                } else {
                    web3::types::U256::from_big_endian(word)
                };
                Value::Certain(value)
            })
            .collect();

        let events = output.logs.into_iter().map(Event::from).collect();

        Self {
            return_data,
            exception: output.exception,
            events,
        }
    }
}

impl PartialEq<Self> for Output {
    fn eq(&self, other: &Self) -> bool {
        if self.exception != other.exception {
            return false;
        }
        if self.events.len() != other.events.len() {
            return false;
        }
        if self.return_data.len() != other.return_data.len() {
            return false;
        }

        for index in 0..self.return_data.len() {
            if let (Value::Certain(value_1), Value::Certain(value_2)) =
                (&self.return_data[index], &other.return_data[index])
            {
                if value_1 != value_2 {
                    return false;
                }
            }
        }

        for index in 0..self.events.len() {
            if self.events[index] != other.events[index] {
                return false;
            }
        }

        true
    }
}
