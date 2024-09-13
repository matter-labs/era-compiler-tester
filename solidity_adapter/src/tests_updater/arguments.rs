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
    /// Source directory of changed tests.
    #[structopt(
        default_value = "solidity/test/libsolidity/semanticTests",
        short = "s",
        long = "source"
    )]
    pub source: PathBuf,

    /// Path of the tests' index.
    #[structopt(short = "i", long = "index")]
    pub index: PathBuf,

    /// Destination directory for tests to be updated.
    #[structopt(short = "d", long = "destination")]
    pub destination: PathBuf,

    /// Whether to only update the index, and do not touch the files.
    #[structopt(long = "index-only")]
    pub index_only: bool,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
