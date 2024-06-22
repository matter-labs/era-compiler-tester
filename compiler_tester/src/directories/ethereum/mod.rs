//!
//! The Ethereum tests directory.
//!

pub mod test;

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::directories::Collection;
use crate::filters::Filters;
use crate::summary::Summary;
use crate::target::Target;

use self::test::EthereumTest;

///
/// The Ethereum tests directory.
///
pub struct EthereumDirectory;

impl EthereumDirectory {
    ///
    /// The upstream index file path.
    /// 
    /// Must be appended to the tests directory.
    ///
    const INDEX_NAME_UPSTREAM: &'static str = "ethereum.yaml";

    ///
    /// The ZKsync index file name.
    /// 
    /// Should refer to a file in the tester repository root.
    ///
    const INDEX_NAME_ZKSYNC: &'static str = "index.yaml";

    ///
    /// Reads the Ethereum test index.
    ///
    pub fn read_index(index_path: &Path) -> anyhow::Result<solidity_adapter::FSEntity> {
        let index_data = std::fs::read_to_string(index_path)?;
        let index: solidity_adapter::FSEntity = serde_yaml::from_str(index_data.as_str())?;
        Ok(index)
    }
}

impl Collection for EthereumDirectory {
    type Test = EthereumTest;

    fn read_all(
        target: Target,
        directory_path: &Path,
        _extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Vec<Self::Test>> {
        let index_path = match target {
            Target::EraVM => {
                let mut index_path = directory_path.to_path_buf();
                index_path.push(Self::INDEX_NAME_ZKSYNC);
                index_path
            },
            Target::EVMInterpreter | Target::EVM => {
                PathBuf::from(Self::INDEX_NAME_UPSTREAM)
            }
        };

        Ok(Self::read_index(index_path.as_path())?
            .into_enabled_list(directory_path)
            .into_iter()
            .filter_map(|test| EthereumTest::new(test, summary.clone(), filters))
            .collect())
    }
}
