//!
//! The EraVM contract build.
//!

use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

///
/// The EraVM contract build.
///
#[derive(Debug, Clone)]
pub struct Build {
    /// The contract assembly.
    pub assembly: zkevm_assembly::Assembly,
    /// The bytecode hash.
    pub bytecode_hash: web3::types::U256,
}

impl Build {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(assembly: zkevm_assembly::Assembly) -> anyhow::Result<Self> {
        let bytecode: Vec<[u8; compiler_common::BYTE_LENGTH_FIELD]> = assembly
            .clone()
            .compile_to_bytecode()
            .map_err(|error| anyhow::anyhow!("Compiling to bytecode error: {}", error))?;
        let bytecode_hash =
            zkevm_assembly::zkevm_opcode_defs::bytecode_to_code_hash(bytecode.as_slice())
                .map(hex::encode)
                .map_err(|_error| anyhow::anyhow!("Bytecode hash computation error"))?;
        let bytecode_hash = web3::types::U256::from_str(bytecode_hash.as_str())
            .map_err(|error| anyhow::anyhow!("Contract hash is invalid: {}", error))?;

        Ok(Self {
            assembly,
            bytecode_hash,
        })
    }

    ///
    /// A shortcut constructor.
    ///
    pub fn new_with_hash(
        assembly: zkevm_assembly::Assembly,
        bytecode_hash: String,
    ) -> anyhow::Result<Self> {
        let bytecode_hash = web3::types::U256::from_str(bytecode_hash.as_str())
            .map_err(|error| anyhow::anyhow!("Bytecode hash is invalid: {}", error))?;

        Ok(Self {
            assembly,
            bytecode_hash,
        })
    }
}

///
/// The helper struct for Build serialization/deserialization.
///
#[derive(Serialize, Deserialize)]
struct BuildHelper {
    /// The bytecode hash.
    bytecode_hash: web3::types::U256,
    /// The contract assembly string.
    assembly: String,
    /// The contract metadata hash.
    metadata_hash: Option<[u8; compiler_common::BYTE_LENGTH_FIELD]>,
}

impl Serialize for Build {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        BuildHelper {
            bytecode_hash: self.bytecode_hash,
            assembly: self.assembly.assembly_code.clone(),
            metadata_hash: self.assembly.metadata_hash,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Build {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).and_then(|helper: BuildHelper| {
            Ok(Self {
                bytecode_hash: helper.bytecode_hash,
                assembly: zkevm_assembly::Assembly::from_string(
                    helper.assembly,
                    helper.metadata_hash,
                )
                .map_err(|error| {
                    serde::de::Error::custom(format!("Assembly deserialization error: {error}"))
                })?,
            })
        })
    }
}
