//!
//! The test input calldata.
//!

use std::collections::BTreeMap;

use crate::directories::matter_labs::test::metadata::case::input::calldata::Calldata as MatterLabsTestInputCalldata;
use crate::test::case::input::value::Value;
use crate::test::instance::Instance;

///
/// The test input calldata.
///
#[derive(Debug, Clone, Default)]
pub struct Calldata {
    /// The calldata bytes.
    pub inner: Vec<u8>,
}

impl Calldata {
    ///
    /// Try convert from Matter Labs compiler test storage data.
    ///
    pub fn try_from_matter_labs(
        calldata: MatterLabsTestInputCalldata,
        instances: &BTreeMap<String, Instance>,
        target: era_compiler_common::Target,
    ) -> anyhow::Result<Self> {
        let calldata = match calldata {
            MatterLabsTestInputCalldata::Value(value) => {
                let hex = value.strip_prefix("0x").ok_or_else(|| {
                    anyhow::anyhow!("Expected a hexadecimal starting with `0x`, found `{value}`")
                })?;

                hex::decode(hex).map_err(|error| {
                    anyhow::anyhow!("Hexadecimal value `{value}` decoding error: {}", error)
                })?
            }
            MatterLabsTestInputCalldata::List(values) => {
                let mut result = Vec::with_capacity(values.len());
                let calldata = Value::try_from_vec_matter_labs(values, instances, target)?;
                for value in calldata.into_iter() {
                    let value = match value {
                        Value::Certain(value) => value,
                        Value::Any => anyhow::bail!("The `*` wildcard is not allowed in calldata"),
                    };
                    let mut bytes = [0u8; era_compiler_common::BYTE_LENGTH_FIELD];
                    value.to_big_endian(&mut bytes);
                    result.extend(bytes);
                }
                result
            }
        };
        Ok(Self { inner: calldata })
    }

    ///
    /// Pushes a selector to the calldata.
    ///
    pub fn push_selector(&mut self, selector: u32) {
        let mut calldata_with_selector = selector.to_be_bytes().to_vec();
        calldata_with_selector.append(&mut self.inner);
        self.inner = calldata_with_selector;
    }
}

impl From<Vec<u8>> for Calldata {
    fn from(value: Vec<u8>) -> Self {
        Self { inner: value }
    }
}
