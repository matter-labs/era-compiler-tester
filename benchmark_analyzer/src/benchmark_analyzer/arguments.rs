//!
//! The benchmark analyzer arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

///
/// The benchmark analyzer arguments.
///
#[derive(Debug, StructOpt)]
#[structopt(name = "benchmark-analyzer", about = "The zkEVM benchmark analyzer")]
pub struct Arguments {
    /// The reference build benchmark.
    #[structopt(long = "reference")]
    pub reference: PathBuf,

    /// The candidate build benchmark.
    #[structopt(long = "candidate")]
    pub candidate: PathBuf,

    /// The output file. If unset, the result is printed to `stdout`.
    #[structopt(short = "o", long = "output-file")]
    pub output_path: Option<PathBuf>,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
