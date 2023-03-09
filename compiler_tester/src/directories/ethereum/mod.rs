//!
//! The Ethereum tests directory.
//!

pub mod test;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::compilers::Compiler;
use crate::filters::Filters;
use crate::Summary;

use super::TestsDirectory;

use self::test::EthereumTest;

///
/// The Ethereum tests directory.
///
pub struct EthereumDirectory {}

impl EthereumDirectory {
    ///
    /// The index file name.
    ///
    const INDEX_NAME: &'static str = "index.yaml";
}

impl<C> TestsDirectory<C> for EthereumDirectory
where
    C: Compiler,
{
    type Test = EthereumTest<C>;

    fn all_tests(
        directory_path: &Path,
        _extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        filters: &Filters,
    ) -> anyhow::Result<Vec<Self::Test>> {
        let mut index_path = directory_path.to_path_buf();
        index_path.push(Self::INDEX_NAME);
        let index_data = std::fs::read_to_string(index_path)?;
        let index: solidity_adapter::FSEntity = serde_yaml::from_str(index_data.as_str())?;
        let tests = index
            .into_enabled_list(directory_path)
            .into_iter()
            .filter_map(|test| {
                EthereumTest::new(test, summary.clone(), debug_config.clone(), filters)
            })
            .collect();

        Ok(tests)
    }

    fn single_test(
        directory_path: &Path,
        test_path: &Path,
        _extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        debug_config: Option<compiler_llvm_context::DebugConfig>,
        filters: &Filters,
    ) -> anyhow::Result<Option<Self::Test>> {
        let mut index_path = directory_path.to_path_buf();
        index_path.push(Self::INDEX_NAME);
        let index_data = std::fs::read_to_string(index_path.as_path())?;
        let index: solidity_adapter::FSEntity = serde_yaml::from_str(index_data.as_str())?;
        index
            .into_enabled_test(directory_path, test_path)
            .ok_or_else(|| anyhow::anyhow!("Test not found"))
            .map(|test| EthereumTest::new(test, summary, debug_config, filters))
    }
}
