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
    /// Input files.
    #[structopt(long)]
    pub input_paths: Vec<PathBuf>,

    /// Benchmark output format: `json`, `csv`, or `json-lnt`.
    /// Using `json-lnt` requires providing the path to a JSON file describing the
    /// benchmarking context via `--benchmark-context`.
    #[structopt(long = "benchmark-format", default_value_t = benchmark_analyzer::OutputFormat::Json)]
    pub benchmark_format: benchmark_analyzer::OutputFormat,

    /// Benchmark context to pass additional data.
    #[structopt(long = "benchmark-context")]
    pub benchmark_context: PathBuf,

    /// Output files.
    #[structopt(long)]
    pub output_path: PathBuf,
}

impl Arguments {
    ///
    /// Validates the command line arguments.
    ///
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.input_paths.is_empty() {
            anyhow::bail!("No input files provided. Use `--input-paths` to specify input files.");
        }
        Ok(())
    }
}
