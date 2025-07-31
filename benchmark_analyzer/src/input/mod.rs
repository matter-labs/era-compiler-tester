//!
//! Benchmark input format.
//!

pub mod foundry_gas;
pub mod foundry_size;

use std::path::Path;
use std::path::PathBuf;

use self::foundry_gas::FoundryGasReport;
use self::foundry_size::FoundrySizeReport;

///
/// Benchmark input format.
///
#[derive(Debug, serde::Deserialize)]
pub struct Input {
    /// The original report.
    pub data: Report,

    /// Project identifier.
    /// Must be added to the original Foundry report.
    pub project: String,
    /// Optional toolchain identifier.
    /// Must be added to the original Foundry report.
    pub toolchain: String,
}

///
/// Enum representing various benchmark formats from tooling.
///
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Report {
    /// Foundry gas report.
    FoundryGas(FoundryGasReport),
    /// Foundry size report.
    FoundrySize(FoundrySizeReport),
}

impl From<FoundryGasReport> for Report {
    fn from(foundry_gas_report: FoundryGasReport) -> Self {
        Report::FoundryGas(foundry_gas_report)
    }
}

impl From<FoundrySizeReport> for Report {
    fn from(foundry_size_report: FoundrySizeReport) -> Self {
        Report::FoundrySize(foundry_size_report)
    }
}

///
/// Input report reading error.
///
#[derive(Debug, thiserror::Error)]
pub enum InputError {
    /// Error reading the input file.
    #[error("Reading input file {path:?}: {error}")]
    Reading {
        /// The underlying IO error.
        error: std::io::Error,
        /// The path to the input file.
        path: PathBuf,
    },
    /// Error parsing the input file.
    #[error("Parsing input file {path:?}: {error}")]
    Parsing {
        /// The underlying JSON parsing error.
        error: serde_json::Error,
        /// The path to the input file.
        path: PathBuf,
    },
    /// Empty file error.
    #[error("Input file {path:?} is empty")]
    EmptyFile {
        /// The path to the input file.
        path: PathBuf,
    },
}

impl TryFrom<&Path> for Input {
    type Error = InputError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let text = std::fs::read_to_string(path).map_err(|error| InputError::Reading {
            error,
            path: path.to_path_buf(),
        })?;
        if text.is_empty() {
            return Err(InputError::EmptyFile {
                path: path.to_path_buf(),
            });
        }
        let json: Self =
            serde_json::from_str(text.as_str()).map_err(|error| InputError::Parsing {
                error,
                path: path.to_path_buf(),
            })?;
        Ok(json)
    }
}
