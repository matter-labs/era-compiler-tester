//!
//! The benchmark representation.
//!

pub mod format;
pub mod group;

use std::collections::BTreeMap;
use std::path::PathBuf;

use format::IBenchmarkSerializer;
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

    /// The EVM interpreter cycles group identifier.
    pub const EVM_INTERPRETER_GROUP_NAME_CYCLES: &'static str = "EVMInterpreter M3B3";

    /// The EVM opcodes to test.
    pub const EVM_OPCODES: [&'static str; 135] = [
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
        "CALLDATALOAD",
        "CALLDATASIZE",
        "CALLDATACOPY",
        "CODESIZE",
        "CODECOPY",
        "GASPRICE",
        "EXTCODESIZE",
        "EXTCODECOPY",
        "RETURNDATASIZE",
        "RETURNDATACOPY",
        "EXTCODEHASH",
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
        "PUSH0",
        "PUSH1",
        "PUSH2",
        "PUSH4",
        "PUSH5",
        "PUSH6",
        "PUSH7",
        "PUSH8",
        "PUSH9",
        "PUSH10",
        "PUSH11",
        "PUSH12",
        "PUSH13",
        "PUSH14",
        "PUSH15",
        "PUSH16",
        "PUSH17",
        "PUSH18",
        "PUSH19",
        "PUSH20",
        "PUSH21",
        "PUSH22",
        "PUSH23",
        "PUSH24",
        "PUSH25",
        "PUSH26",
        "PUSH27",
        "PUSH28",
        "PUSH29",
        "PUSH30",
        "PUSH31",
        "PUSH32",
        "DUP1",
        "DUP2",
        "DUP3",
        "DUP4",
        "DUP5",
        "DUP6",
        "DUP7",
        "DUP8",
        "DUP9",
        "DUP10",
        "DUP11",
        "DUP12",
        "DUP13",
        "DUP14",
        "DUP15",
        "DUP16",
        "SWAP1",
        "SWAP2",
        "SWAP3",
        "SWAP4",
        "SWAP5",
        "SWAP6",
        "SWAP7",
        "SWAP8",
        "SWAP9",
        "SWAP10",
        "SWAP11",
        "SWAP12",
        "SWAP13",
        "SWAP14",
        "SWAP15",
        "SWAP16",
        "CALL",
        "STATICCALL",
        "DELEGATECALL",
        "CREATE",
        "CREATE2",
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
            if group_name.starts_with(Self::EVM_INTERPRETER_GROUP_NAME_CYCLES) {
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
    /// Writes the benchmark results to a file using a provided serializer.
    ///
    pub fn write_to_file(
        self,
        path: PathBuf,
        serializer: impl IBenchmarkSerializer,
    ) -> anyhow::Result<()> {
        let contents = serializer.serialize_to_string(&self).expect("Always valid");
        std::fs::write(path.as_path(), contents)
            .map_err(|error| anyhow::anyhow!("Benchmark file {path:?} reading: {error}"))?;
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
