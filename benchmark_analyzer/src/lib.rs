//!
//! The benchmark analyzer library.
//!

pub mod analysis;
pub mod format;
pub mod model;
pub mod results;
pub mod util;

pub use self::format::csv::Csv as CsvSerializer;
pub use self::format::json::native::Json as JsonNativeSerializer;

pub use self::model::benchmark::test::codegen::versioned::executable::run::Run;
pub use self::model::benchmark::test::codegen::versioned::executable::Executable;
pub use self::model::benchmark::test::codegen::versioned::VersionedGroup;
pub use self::model::benchmark::test::codegen::CodegenGroup;
pub use self::model::benchmark::test::input::Input;
pub use self::model::benchmark::test::selector::Selector as TestSelector;
pub use self::model::benchmark::test::Test;
pub use self::model::benchmark::write_to_file;
pub use self::model::benchmark::Benchmark;

// Metadata for various parts of the model
pub use self::model::benchmark::metadata::BenchmarkVersion;
pub use self::model::benchmark::metadata::Metadata as BenchmarkMetadata;
pub use self::model::benchmark::test::codegen::versioned::executable::metadata::Metadata as ExecutableMetadata;
pub use self::model::benchmark::test::metadata::Metadata as TestMetadata;

pub use self::results::group::Group as ResultsGroup;

pub use self::model::evm_interpreter::GROUP_NAME as TEST_GROUP_EVM_INTERPRETER;
