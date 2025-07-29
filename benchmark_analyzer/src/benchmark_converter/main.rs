//!
//! The benchmark analyzer binary.
//!

pub(crate) mod arguments;
pub(crate) mod tests;

use chrono::Utc;
use clap::Parser;

use self::arguments::Arguments;

///
/// The application entry point.
///
fn main() -> anyhow::Result<()> {
    let arguments = Arguments::try_parse()?;

    let context = match arguments.benchmark_context {
        Some(path) => benchmark_analyzer::BenchmarkContext::try_from(path)?,
        None => benchmark_analyzer::BenchmarkContext::default(),
    };
    let metadata = benchmark_analyzer::BenchmarkMetadata {
        start: Utc::now(),
        end: Utc::now(),
        context: Some(context),
    };

    let mut benchmark = benchmark_analyzer::Benchmark::new(metadata);
    if arguments.input_paths.len() == 1 {
        if !arguments.input_paths[0].is_dir() {
            anyhow::bail!(
                "Expected a directory with JSON files, but got a file: {}",
                arguments.input_paths[0].display()
            );
        }
        let resolution_pattern =
            format!("{}/**/*.json", arguments.input_paths[0].to_string_lossy());
        for path in glob::glob(resolution_pattern.as_str())?.filter_map(Result::ok) {
            let foundry_report = benchmark_analyzer::FoundryReport::try_from(path.as_path())?;
            benchmark.extend_with_foundry(foundry_report)?;
        }
    } else if arguments.input_paths.is_empty() {
        anyhow::bail!("No input files provided. Use `--input-paths` to specify input files.");
    } else {
        for input_path in arguments.input_paths.into_iter() {
            let foundry_report = benchmark_analyzer::FoundryReport::try_from(input_path.as_path())?;
            benchmark.extend_with_foundry(foundry_report)?;
        }
    }
    benchmark.metadata.end = Utc::now();

    let output: benchmark_analyzer::Output = (benchmark, arguments.output_format).try_into()?;
    output.write_to_file(arguments.output_path)?;

    Ok(())
}
