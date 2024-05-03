//!
//! The benchmark representation.
//!

pub mod group;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use self::group::results::Results;
use self::group::Group;

///
/// The benchmark representation.
///
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Benchmark {
    /// The benchmark groups.
    pub groups: BTreeMap<String, Group>,
}

impl Benchmark {
    /// The EVM interpreter group identifier.
    pub const EVM_INTERPRETER_GROUP_NAME: &'static str = "EVMInterpreter";

    /// The EVM interpreter group identifier prefix.
    pub const EVM_INTERPRETER_GROUP_PREFIX: &'static str = "EVMInterpreter M3B3";

    /// The EVM opcodes to test.
    pub const EVM_OPCODES: [&'static str; 56] = [
        "ADD",
        "MUL",
        "SUB",
        "DIV",
        "SDIV",
        "MOD",
        "SMOD",
        "ADDMOD",
        "MULMOD",
        "EXP",
        "SIGNEXTEND",
        "LT",
        "GT",
        "SLT",
        "SGT",
        "EQ",
        "ISZERO",
        "AND",
        "OR",
        "XOR",
        "NOT",
        "BYTE",
        "SHL",
        "SHR",
        "SAR",
        "SGT",
        "SHA3",
        "ADDRESS",
        "BALANCE",
        "ORIGIN",
        "CALLER",
        "CALLVALUE",
        "BLOCKHASH",
        "COINBASE",
        "TIMESTAMP",
        "NUMBER",
        "PREVRANDAO",
        "GASLIMIT",
        "CHAINID",
        "SELFBALANCE",
        "BASEFEE",
        "POP",
        "MLOAD",
        "MSTORE",
        "MSTORE8",
        "SLOAD",
        "SSTORE",
        "JUMP",
        "JUMPI",
        "PC",
        "MSIZE",
        "GAS",
        "JUMPDEST",
        "PUSH1",
        "RETURN",
        "REVERT",
    ];

    ///
    /// Compares two benchmarks.
    ///
    pub fn compare<'a>(reference: &'a Self, candidate: &'a Self) -> BTreeMap<&'a str, Results<'a>> {
        let mut results = BTreeMap::new();

        for (group_name, reference_group) in reference.groups.iter() {
            let candidate_group = match candidate.groups.get(group_name) {
                Some(candidate_group) => candidate_group,
                None => continue,
            };

            let mut group_results = Group::compare(reference_group, candidate_group);
            if group_name.starts_with(Self::EVM_INTERPRETER_GROUP_PREFIX) {
                if let (Some(reference_ratios), Some(candidate_ratios)) = (
                    reference
                        .groups
                        .get(group_name.as_str())
                        .map(|group| group.evm_interpreter_ratios()),
                    candidate
                        .groups
                        .get(group_name.as_str())
                        .map(|group| group.evm_interpreter_ratios()),
                ) {
                    group_results.set_evm_interpreter_ratios(reference_ratios, candidate_ratios);
                }
            }
            results.insert(group_name.as_str(), group_results);
        }

        results
    }

    ///
    /// Writes the benchmark to a file.
    ///
    pub fn write_to_file(self, path: PathBuf) -> anyhow::Result<()> {
        let contents = serde_json::to_string(&self).expect("Always valid");
        std::fs::write(path.as_path(), contents)
            .map_err(|error| anyhow::anyhow!("Benchmark file {:?} reading: {}", path, error))?;
        Ok(())
    }
}

impl TryFrom<PathBuf> for Benchmark {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let text = std::fs::read_to_string(path.as_path())
            .map_err(|error| anyhow::anyhow!("Benchmark file {:?} reading: {}", path, error))?;
        let json: Self = serde_json::from_str(text.as_str())
            .map_err(|error| anyhow::anyhow!("Benchmark file {:?} parsing: {}", path, error))?;
        Ok(json)
    }
}
