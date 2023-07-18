//!
//! The buildable compiler test trait.
//!

pub mod ethereum;
pub mod matter_labs;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::mode::Mode;
use crate::compilers::Compiler;
use crate::filters::Filters;
use crate::summary::Summary;
use crate::test::Test;

///
/// The buildable compiler test trait.
///
pub trait Buildable: Send + Sync + 'static {
    ///
    /// Builds the test.
    ///
    fn build(
        &self,
        mode: Mode,
        compiler: Arc<dyn Compiler>,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
    ) -> Option<Test>;
}

///
/// The compiler tests directory trait.
///
pub trait TestsDirectory {
    ///
    /// The test type.
    ///
    type Test: Buildable;

    ///
    /// Returns all directory tests.
    ///
    fn all_tests(
        directory_path: &Path,
        extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Vec<Self::Test>>;

    ///
    /// Returns a single test.
    ///
    fn single_test(
        directory_path: &Path,
        test_path: &Path,
        extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Option<Self::Test>>;
}
