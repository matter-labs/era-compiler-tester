//!
//! The Ethereum tests directory.
//!

pub mod test;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::directories::Collection;
use crate::filters::Filters;
use crate::summary::Summary;

use self::test::EthereumTest;

///
/// The Ethereum tests directory.
///
pub struct EthereumDirectory;

impl EthereumDirectory {
    ///
    /// The index file name.
    ///
    const INDEX_NAME: &'static str = "index.yaml";

    ///
    /// Reads the Ethereum test index.
    ///
    pub fn read_index(directory_path: &Path) -> anyhow::Result<solidity_adapter::FSEntity> {
        let mut index_path = directory_path.to_path_buf();
        index_path.push(Self::INDEX_NAME);
        let index_data = std::fs::read_to_string(index_path)?;
        let index: solidity_adapter::FSEntity = serde_yaml::from_str(index_data.as_str())?;
        Ok(index)
    }
}

impl Collection for EthereumDirectory {
    type Test = EthereumTest;

    fn read_all(
        directory_path: &Path,
        _extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Vec<Self::Test>> {
        Ok(Self::read_index(directory_path)?
            .into_enabled_list(directory_path)
            .into_iter()
            .filter_map(|test| EthereumTest::new(test, summary.clone(), filters))
            .collect())
    }
}
