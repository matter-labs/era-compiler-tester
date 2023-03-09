//!
//! The compiler test value.
//!

use std::collections::HashMap;
use std::str::FromStr;

use serde::Serialize;
use serde::Serializer;

use crate::test::instance::Instance;

///
/// The compiler test value.
///
#[derive(Debug, Clone)]
pub enum Value {
    /// Any value (used for expected data).
    Any,
    /// The certain value.
    Certain(web3::types::U256),
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
            Self::Certain(value) => value,
            Self::Any => panic!("Value in any"),
        }
    }

    ///
    /// Try convert from Matter Labs compiler test metadata value.
    ///
    pub fn try_from_matter_labs(
        value: &str,
        instances: &HashMap<String, Instance>,
    ) -> anyhow::Result<Self> {
        if value == "*" {
            return Ok(Self::Any);
        }

        let value = if let Some(instance) = value.strip_suffix(".address") {
            web3::types::U256::from_big_endian(
                instances
                    .get(instance)
                    .ok_or_else(|| anyhow::anyhow!("Instance `{}` not found", instance))?
                    .address
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
        } else {
            web3::types::U256::from_dec_str(value)
                .map_err(|error| anyhow::anyhow!("Invalid decimal literal: {}", error))?
        };
        Ok(Self::Certain(value))
    }

    ///
    /// Try convert into vec of self from vec of Matter Labs compiler test metadata values.
    ///
    pub fn try_from_vec_matter_labs(
        values: &[String],
        instances: &HashMap<String, Instance>,
    ) -> anyhow::Result<Vec<Self>> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                Self::try_from_matter_labs(value, instances)
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
            Value::Certain(value) => format!("0x{}", crate::utils::u256_as_string(value)),
            Value::Any => "*".to_string(),
        };
        serializer.serialize_str(&value_str)
    }
}
