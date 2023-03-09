//!
//! The coverage watcher arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

///
/// The coverage watcher arguments.
///
#[derive(Debug, StructOpt)]
#[structopt(name = "coverage-watcher", about = "The tests coverage watcher")]
pub struct Arguments {
    /// The missed tests output file path.
    #[structopt(short = "o", long = "output")]
    pub output: Option<PathBuf>,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
