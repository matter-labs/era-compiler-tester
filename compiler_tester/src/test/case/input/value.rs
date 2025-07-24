//!
//! The compiler test value.
//!

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::Serialize;
use serde::Serializer;

use crate::test::instance::Instance;
use crate::vm::eravm::system_context::SystemContext;

///
/// The compiler test value.
///
#[derive(Debug, Clone)]
pub enum Value {
    /// Any value (used for expected data).
    Any,
    /// The certain value.
    Known(web3::types::U256),
}

impl Value {
    ///
    /// Unwrap certain value as reference.
    ///
    /// # Panics
    ///
    /// Will panic if the value is any.
    ///
    pub fn unwrap_certain_as_ref(&self) -> &web3::types::U256 {
        match self {
            Self::Known(value) => value,
            Self::Any => panic!("Value is unknown"),
        }
    }

    ///
    /// Try convert from Matter Labs compiler test metadata value.
    ///
    pub fn try_from_matter_labs(
        value: String,
        instances: &BTreeMap<String, Instance>,
        target: era_compiler_common::Target,
    ) -> anyhow::Result<Self> {
        if value == "*" {
            return Ok(Self::Any);
        }

        let value = if let Some(instance) = value.strip_suffix(".address") {
            web3::types::U256::from_big_endian(
                instances
                    .get(instance)
                    .ok_or_else(|| anyhow::anyhow!("Instance `{}` not found", instance))?
                    .address()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Instance `{}` was not successfully deployed", instance)
                    })?
                    .as_bytes(),
            )
        } else if let Some(value) = value.strip_prefix('-') {
            let value = web3::types::U256::from_dec_str(value)
                .map_err(|error| anyhow::anyhow!("Invalid decimal literal after `-`: {}", error))?;
            if value > web3::types::U256::one() << 255u8 {
                anyhow::bail!("Decimal literal after `-` is too big");
            }
            let value = value
                .checked_sub(web3::types::U256::one())
                .ok_or_else(|| anyhow::anyhow!("`-0` is invalid literal"))?;
            web3::types::U256::max_value()
                .checked_sub(value)
                .expect("Always valid")
        } else if let Some(value) = value.strip_prefix("0x") {
            web3::types::U256::from_str(value)
                .map_err(|error| anyhow::anyhow!("Invalid hexadecimal literal: {}", error))?
        } else if value == "$CHAIN_ID" {
            match target {
                era_compiler_common::Target::EraVM => {
                    web3::types::U256::from(SystemContext::CHAIND_ID_ERAVM)
                }
                era_compiler_common::Target::EVM => {
                    web3::types::U256::from(SystemContext::CHAIND_ID_EVM)
                }
            }
        } else if value == "$GAS_LIMIT" {
            match target {
                era_compiler_common::Target::EraVM => {
                    web3::types::U256::from(SystemContext::BLOCK_GAS_LIMIT_ERAVM)
                }
                era_compiler_common::Target::EVM => {
                    web3::types::U256::from(SystemContext::BLOCK_GAS_LIMIT_EVM)
                }
            }
        } else if value == "$COINBASE" {
            match target {
                era_compiler_common::Target::EraVM => web3::types::U256::from_str_radix(
                    SystemContext::COIN_BASE_ERAVM,
                    era_compiler_common::BASE_HEXADECIMAL,
                ),
                era_compiler_common::Target::EVM => web3::types::U256::from_str_radix(
                    SystemContext::COIN_BASE_EVM,
                    era_compiler_common::BASE_HEXADECIMAL,
                ),
            }
            .expect("Always valid")
        } else if value == "$DIFFICULTY" {
            web3::types::U256::from_str_radix(
                SystemContext::BLOCK_DIFFICULTY_POST_PARIS,
                era_compiler_common::BASE_HEXADECIMAL,
            )
            .expect("Always valid")
        } else if value.starts_with("$BLOCK_HASH") {
            let offset: u64 = value
                .split(':')
                .next_back()
                .and_then(|value| value.parse().ok())
                .unwrap_or_default();
            let mut hash =
                web3::types::U256::from_str(SystemContext::ZERO_BLOCK_HASH).expect("Always valid");
            hash += web3::types::U256::from(offset);
            hash
        } else if value == "$BLOCK_NUMBER" {
            web3::types::U256::from(SystemContext::CURRENT_BLOCK_NUMBER)
        } else if value == "$BLOCK_TIMESTAMP" {
            match target {
                era_compiler_common::Target::EraVM => {
                    web3::types::U256::from(SystemContext::CURRENT_BLOCK_TIMESTAMP_ERAVM)
                }
                era_compiler_common::Target::EVM => {
                    web3::types::U256::from(SystemContext::CURRENT_BLOCK_TIMESTAMP_EVM)
                }
            }
        } else {
            web3::types::U256::from_dec_str(value.as_str())
                .map_err(|error| anyhow::anyhow!("Invalid decimal literal: {}", error))?
        };

        Ok(Self::Known(value))
    }

    ///
    /// Try convert into vec of self from vec of Matter Labs compiler test metadata values.
    ///
    pub fn try_from_vec_matter_labs(
        values: Vec<String>,
        instances: &BTreeMap<String, Instance>,
        target: era_compiler_common::Target,
    ) -> anyhow::Result<Vec<Self>> {
        values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                Self::try_from_matter_labs(value, instances, target)
                    .map_err(|error| anyhow::anyhow!("Value {} is invalid: {}", index, error))
            })
            .collect::<anyhow::Result<Vec<Self>>>()
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value_str = match self {
            Value::Known(value) => format!("0x{}", crate::utils::u256_as_string(value)),
            Value::Any => "*".to_string(),
        };
        serializer.serialize_str(&value_str)
    }
}
