//!
//! The benchmark analyzer arguments.
//!

use std::path::PathBuf;

use clap::Parser;

///
/// The benchmark analyzer arguments.
///
#[derive(Debug, Parser)]
#[command(about, long_about = None)]
pub struct Arguments {
    /// The reference build benchmark.
    #[structopt(long, default_value = "reference.json")]
    pub reference: PathBuf,

    /// The candidate build benchmark.
    #[structopt(long, default_value = "candidate.json")]
    pub candidate: PathBuf,

    /// The output file. If unset, the result is printed to `stdout`.
    #[structopt(short = 'o', long)]
    pub output_file: Option<PathBuf>,

    /// Maximum number of results displayed in a group.
    #[structopt(long, default_value_t = 100)]
    pub group_max: usize,

    /// Regular expression to select reference group for the comparison.
    #[structopt(long)]
    pub query_reference: Option<String>,

    /// Regular expression to select candidate group for the comparison.
    #[structopt(long)]
    pub query_candidate: Option<String>,
}
