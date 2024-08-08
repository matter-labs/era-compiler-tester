//!
//! The compiler test outcome event.
//!

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Serialize;

use crate::directories::matter_labs::test::metadata::case::input::expected::variant::extended::event::Event as MatterLabsTestExpectedEvent;
use crate::test::instance::Instance;
use crate::test::case::input::value::Value;

///
/// The compiler test outcome event.
///
#[derive(Debug, Serialize, Clone)]
pub struct Event {
    /// The event address.
    address: Option<web3::types::Address>,
    /// The event topics.
    topics: Vec<Value>,
    /// The event values.
    values: Vec<Value>,
}

impl Event {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        address: Option<web3::types::Address>,
        topics: Vec<Value>,
        values: Vec<Value>,
    ) -> Self {
        Self {
            address,
            topics,
            values,
        }
    }

    ///
    /// Try convert from Matter Labs compiler test metadata expected event.
    ///
    pub fn try_from_matter_labs(
        event: MatterLabsTestExpectedEvent,
        instances: &BTreeMap<String, Instance>,
    ) -> anyhow::Result<Self> {
        let topics = Value::try_from_vec_matter_labs(event.topics, instances)
            .map_err(|error| anyhow::anyhow!("Invalid topics: {}", error))?;
        let values = Value::try_from_vec_matter_labs(event.values, instances)
            .map_err(|error| anyhow::anyhow!("Invalid values: {}", error))?;

        let address = match event.address {
            Some(address) => Some(
                if let Some(instance) = address.strip_suffix(".address") {
                    instances
                        .get(instance)
                        .ok_or_else(|| anyhow::anyhow!("Instance `{}` not found", instance))?
                        .address()
                        .copied()
                        .ok_or_else(|| {
                            anyhow::anyhow!("Instance `{}` was not successfully deployed", instance)
                        })
                } else {
                    web3::types::Address::from_str(address.as_str())
                        .map_err(|error| anyhow::anyhow!("Invalid address literal: {}", error))
                }
                .map_err(|error| anyhow::anyhow!("Invalid event address `{address}`: {error}"))?,
            ),
            None => None,
        };

        Ok(Self {
            address,
            topics,
            values,
        })
    }

    ///
    /// Convert from Ethereum compiler test metadata expected event.
    ///
    pub fn from_ethereum(
        event: &solidity_adapter::Event,
        contract_address: &web3::types::Address,
    ) -> Self {
        let topics = event
            .topics
            .iter()
            .map(|topic| {
                let mut topic_str = crate::utils::u256_as_string(topic);
                topic_str = topic_str.replace(
                    solidity_adapter::DEFAULT_CONTRACT_ADDRESS,
                    &crate::utils::address_as_string(contract_address),
                );
                Value::Certain(
                    web3::types::U256::from_str(&topic_str)
                        .expect("Solidity adapter default contract address constant is invalid"),
                )
            })
            .collect();

        let values = event
            .expected
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

        Self {
            // The address is ignored, as Ethereum tests expect other addresses
            address: None,
            topics,
            values,
        }
    }
}

impl From<zkevm_tester::events::SolidityLikeEvent> for Event {
    fn from(event: zkevm_tester::events::SolidityLikeEvent) -> Self {
        let mut topics: Vec<Value> = event
            .topics
            .into_iter()
            .map(|topic| Value::Certain(web3::types::U256::from_big_endian(topic.as_slice())))
            .collect();

        // Event are written by the system contract, and the first topic is the `msg.sender`
        let address = crate::utils::u256_to_address(topics.remove(0).unwrap_certain_as_ref());

        let values: Vec<Value> = event
            .data
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

        Self {
            address: Some(address),
            topics,
            values,
        }
    }
}

impl From<evm::Log> for Event {
    fn from(log: evm::Log) -> Self {
        let address = log.address;
        let topics = log
            .topics
            .into_iter()
            .map(|topic| Value::Certain(crate::utils::h256_to_u256(&topic)))
            .collect();
        let values: Vec<Value> = log
            .data
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
        Self {
            address: None,
            topics,
            values,
        }
    }
}

impl PartialEq<Self> for Event {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(address1), Some(address2)) = (self.address, other.address) {
            if address1 != address2 {
                return false;
            }
        };

        if self.topics.len() != other.topics.len() {
            return false;
        }
        if self.values.len() != other.values.len() {
            return false;
        }

        for index in 0..self.topics.len() {
            if let (Value::Certain(value1), Value::Certain(value2)) =
                (&self.topics[index], &other.topics[index])
            {
                if value1 != value2 {
                    return false;
                }
            }
        }

        for index in 0..self.values.len() {
            if let (Value::Certain(value1), Value::Certain(value2)) =
                (&self.values[index], &other.values[index])
            {
                if value1 != value2 {
                    return false;
                }
            }
        }

        true
    }
}
