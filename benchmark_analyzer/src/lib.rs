//!
//! The benchmark analyzer library.
//!

#![allow(clippy::large_enum_variant)]
#![allow(clippy::let_and_return)]

pub mod analysis;
pub mod input;
pub mod model;
pub mod output;
pub mod results;
pub mod util;

pub use crate::input::foundry_gas::FoundryGasReport;
pub use crate::input::Input as InputReport;
pub use crate::input::InputError as InputReportError;
pub use crate::model::benchmark::metadata::Metadata as BenchmarkMetadata;
pub use crate::model::benchmark::test::input::Input;
pub use crate::model::benchmark::test::metadata::Metadata as TestMetadata;
pub use crate::model::benchmark::test::selector::Selector as TestSelector;
pub use crate::model::benchmark::test::toolchain::codegen::versioned::executable::metadata::Metadata as ExecutableMetadata;
pub use crate::model::benchmark::test::toolchain::codegen::versioned::executable::run::Run;
pub use crate::model::benchmark::test::toolchain::codegen::versioned::executable::Executable;
pub use crate::model::benchmark::test::toolchain::codegen::versioned::VersionedGroup;
pub use crate::model::benchmark::test::toolchain::codegen::CodegenGroup;
pub use crate::model::benchmark::test::Test;
pub use crate::model::benchmark::Benchmark;
pub use crate::model::context::Context as BenchmarkContext;
pub use crate::model::evm_interpreter::GROUP_NAME as TEST_GROUP_EVM_INTERPRETER;
pub use crate::output::csv::Csv as CsvOutput;
pub use crate::output::format::Format as OutputFormat;
pub use crate::output::json::lnt::JsonLNT as JsonLNTOutput;
pub use crate::output::json::Json as JsonNativeOutput;
pub use crate::output::Output;
pub use crate::results::group::Group as ResultsGroup;
