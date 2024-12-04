//!
//! Converts `[TestDescription]` to the representation used by the benchmark.
//!

use crate::test::description::TestDescription;
use crate::Mode;

pub fn convert_description(
    description: &TestDescription,
    default_group: &str,
) -> benchmark_analyzer::Metadata {
    let TestDescription {
        group,
        mode,
        selector,
    } = description.clone();
    let selector = selector.into();
    let version = match &mode {
        Some(mode) => mode_version(mode.clone()).map(|m| m.to_string()),
        None => None,
    };
    let mode = mode.map(mode_string).unwrap_or_default();
    let group = group.unwrap_or(default_group.to_string());
    benchmark_analyzer::Metadata {
        selector,
        mode,
        version,
        group,
    }
}

fn mode_version(mode: Mode) -> Option<semver::Version> {
    match mode {
        Mode::Solidity(mode) => Some(mode.solc_version),
        Mode::SolidityUpstream(mode) => Some(mode.solc_version),
        Mode::Yul(_) => None,
        Mode::YulUpstream(mode) => Some(mode.solc_version),
        Mode::Vyper(mode) => Some(mode.vyper_version),
        Mode::LLVM(_) => None,
        Mode::EraVM(_) => None,
    }
}

fn mode_string(mode: Mode) -> Option<String> {
    match mode {
        Mode::Solidity(mode) => Some(mode.repr_without_version()),
        Mode::SolidityUpstream(mode) => Some(mode.repr_without_version()),
        Mode::Yul(_) => None,
        Mode::YulUpstream(mode) => Some(mode.repr_without_version()),
        Mode::Vyper(mode) => Some(mode.repr_without_version()),
        Mode::LLVM(_) => None,
        Mode::EraVM(_) => None,
    }
}
