//!
//! The tests updater's arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

///
/// The tests updater's arguments.
///
#[derive(Debug, StructOpt)]
#[structopt(
    name = "tests-updater",
    about = "ZKsync toolchain test updater for Ethereum Solidity tests"
)]
pub struct Arguments {
    /// Path of the tests' index.
    #[structopt(
        default_value = "tests/solidity/ethereum/index.yaml",
        short = "i",
        long = "index"
    )]
    pub index: PathBuf,

    /// Source directory of changed tests.
    #[structopt(
        default_value = "solidity/test/libsolidity/semanticTests",
        short = "s",
        long = "source"
    )]
    pub source: PathBuf,

    /// Destination directory for tests to be updated.
    #[structopt(
        default_value = "tests/solidity/ethereum",
        short = "d",
        long = "destination"
    )]
    pub destination: PathBuf,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
