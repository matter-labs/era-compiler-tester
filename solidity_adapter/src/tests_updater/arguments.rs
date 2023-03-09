//!
//! The tests updater binary arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

///
/// The tests updater binary arguments.
///
#[derive(Debug, StructOpt)]
#[structopt(name = "tests-updater", about = "The Solidity tests updater")]
pub struct Arguments {
    /// The tests index path
    #[structopt(
        default_value = "tests/solidity/ethereum/index.yaml",
        short = "i",
        long = "index"
    )]
    pub index: PathBuf,

    /// The tests update source.
    #[structopt(
        default_value = "solidity/test/libsolidity/semanticTests",
        short = "s",
        long = "source"
    )]
    pub source: PathBuf,

    /// The tests update destination.
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
