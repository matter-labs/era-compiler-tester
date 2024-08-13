//!
//! The benchmark analyzer arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

///
/// The benchmark analyzer arguments.
///
#[derive(Debug, StructOpt)]
#[structopt(name = "benchmark-analyzer", about = "ZKsync toolchain benchmark analyzer")]
pub struct Arguments {
    /// The reference build benchmark.
    #[structopt(long = "reference", default_value = "reference.json")]
    pub reference: PathBuf,

    /// The candidate build benchmark.
    #[structopt(long = "candidate", default_value = "candidate.json")]
    pub candidate: PathBuf,

    /// The output file. If unset, the result is printed to `stdout`.
    #[structopt(short = "o", long = "output-file")]
    pub output_path: Option<PathBuf>,

    /// Maximum number of results displayed in a group.
    #[structopt(long = "group-max", default_value = "100")]
    pub group_max: usize,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
