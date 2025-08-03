//!
//! The benchmark analyzer arguments.
//!

use std::path::PathBuf;

use clap::Parser;

///
/// The benchmark analyzer arguments.
///
#[derive(Debug, Parser)]
#[command(about, long_about = None, arg_required_else_help = true)]
pub struct Arguments {
    /// Suppresses the terminal output.
    #[arg(short, long)]
    pub quiet: bool,

    /// Input files.
    /// If only one path is provided, it is treated as a directory with JSON files.
    pub input_paths: Vec<PathBuf>,

    /// Benchmark context to pass additional data.
    /// Deprecated: use separate arguments instead.
    #[structopt(long = "benchmark-context")]
    pub benchmark_context: Option<PathBuf>,

    /// Benchmark output format: `json`, `csv`, or `json-lnt`.
    #[structopt(long = "output-format", alias = "benchmark-format", default_value_t = benchmark_analyzer::OutputFormat::Xlsx)]
    pub output_format: benchmark_analyzer::OutputFormat,

    /// Output files.
    #[structopt(long)]
    pub output_path: PathBuf,
}
