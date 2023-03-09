//!
//! The test input calldata.
//!

use std::collections::HashMap;

use crate::directories::matter_labs::test::metadata::case::input::calldata::Calldata as MatterLabsTestInputCalldata;
use crate::test::case::input::value::Value;
use crate::test::instance::Instance;

///
/// The test input calldata.
///
#[derive(Debug, Clone, Default)]
pub struct Calldata {
    /// The inner calldata bytes.
    pub inner: Vec<u8>,
}

impl Calldata {
    ///
    /// Try convert from Matter Labs compiler test storage data.
    ///
    pub fn try_from_matter_labs(
        calldata: &MatterLabsTestInputCalldata,
        instances: &HashMap<String, Instance>,
    ) -> anyhow::Result<Self> {
        let calldata = match calldata {
            MatterLabsTestInputCalldata::Value(value) => {
                let hex = value.strip_prefix("0x").ok_or_else(|| {
                    anyhow::anyhow!("Invalid calldata value, expected hex starting with `0x`")
                })?;

                hex::decode(hex)
                    .map_err(|err| anyhow::anyhow!("Invalid calldata hex value: {}", err))?
            }
            MatterLabsTestInputCalldata::List(values) => {
                let calldata_vec = Value::try_from_vec_matter_labs(values, instances)
                    .map_err(|err| anyhow::anyhow!("Invalid calldata: {}", err))?;
                let mut calldata = Vec::with_capacity(values.len());
                for value in calldata_vec {
                    let value = match value {
                        Value::Certain(value) => value,
                        Value::Any => anyhow::bail!("* not allowed in calldata"),
                    };
                    let mut bytes = [0u8; compiler_common::BYTE_LENGTH_FIELD];
                    value.to_big_endian(&mut bytes);
                    calldata.extend(bytes);
                }
                calldata
            }
        };
        Ok(Self { inner: calldata })
    }

    ///
    /// Insert the selector at the beginning of the calldata.
    ///
    pub fn add_selector(&mut self, selector: u32) {
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
