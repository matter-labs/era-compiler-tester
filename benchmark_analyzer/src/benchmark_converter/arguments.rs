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
    /// Input files.
    pub input_paths: Vec<PathBuf>,

    /// Benchmark input format: only `foundry`.
    #[structopt(long = "input-format", default_value_t = benchmark_analyzer::InputFormat::Foundry)]
    pub input_format: benchmark_analyzer::InputFormat,

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
