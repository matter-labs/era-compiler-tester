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
    arguments.validate()?;

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
    for input in arguments.input_paths.into_iter() {
        let project = input
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid input file name"))?
            .to_string_lossy();
        let project = project
            .strip_suffix(format!(".{}", era_compiler_common::EXTENSION_JSON).as_str())
            .unwrap_or(project.as_ref());

        let foundry_report = benchmark_analyzer::FoundryReport::try_from(input.as_path())?;
        benchmark.extend_with_foundry(project, foundry_report)?;
    }
    benchmark.metadata.end = Utc::now();

    let output: benchmark_analyzer::Output = (benchmark, arguments.benchmark_format).try_into()?;
    output.write_to_file(arguments.output_path)?;

    Ok(())
}
