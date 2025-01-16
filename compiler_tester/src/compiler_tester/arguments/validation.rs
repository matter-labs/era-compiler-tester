//!
//! Validate the arguments passed from user, checking invariants that are not
//! expressed in the type system.
//!

use super::benchmark_format::BenchmarkFormat;
use super::Arguments;

use super::ARGUMENT_BENCHMARK_CONTEXT;

///
/// Validate the arguments passed from user, checking invariants that are not
/// expressed in the type system.
///
pub fn validate_arguments(arguments: Arguments) -> anyhow::Result<Arguments> {
    match (&arguments.benchmark_format, &arguments.benchmark_context) {
        (BenchmarkFormat::JsonLNT, None) =>
            anyhow::bail!("Generation of LNT-compatible benchmark results in JSON format requires passing a valid context in the argument `--{ARGUMENT_BENCHMARK_CONTEXT}` to compiler tester.")
        ,
        (BenchmarkFormat::JsonLNT, Some(_)) => (),
        (_, Some(_)) =>
            anyhow::bail!("Only LNT backend in JSON format supports passing a valid context in the argument `--{ARGUMENT_BENCHMARK_CONTEXT}` to compiler tester.")
        ,
        _ => (),
    }

    Ok(arguments)
}
