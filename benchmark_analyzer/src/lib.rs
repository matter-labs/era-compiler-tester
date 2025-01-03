//!
//! The benchmark analyzer library.
//!

pub mod analysis;
pub mod model;
pub mod output;
pub mod results;
pub mod util;

pub use crate::output::format::csv::Csv as CsvSerializer;
pub use crate::output::format::json::lnt::JsonLNT as JsonLNTSerializer;
pub use crate::output::format::json::native::Json as JsonNativeSerializer;

pub use crate::model::benchmark::test::codegen::versioned::executable::run::Run;
pub use crate::model::benchmark::test::codegen::versioned::executable::Executable;
pub use crate::model::benchmark::test::codegen::versioned::VersionedGroup;
pub use crate::model::benchmark::test::codegen::CodegenGroup;
pub use crate::model::benchmark::test::input::Input;
pub use crate::model::benchmark::test::selector::Selector as TestSelector;
pub use crate::model::benchmark::test::Test;
pub use crate::model::benchmark::write_to_file;
pub use crate::model::benchmark::Benchmark;
pub use crate::model::context::validate_context;
pub use crate::model::context::Context as BenchmarkContext;

// Metadata for various parts of the model
pub use crate::model::benchmark::metadata::BenchmarkVersion;
pub use crate::model::benchmark::metadata::Metadata as BenchmarkMetadata;
pub use crate::model::benchmark::test::codegen::versioned::executable::metadata::Metadata as ExecutableMetadata;
pub use crate::model::benchmark::test::metadata::Metadata as TestMetadata;

pub use crate::results::group::Group as ResultsGroup;

pub use crate::model::evm_interpreter::GROUP_NAME as TEST_GROUP_EVM_INTERPRETER;
