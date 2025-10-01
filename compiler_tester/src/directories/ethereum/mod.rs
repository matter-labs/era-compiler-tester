//!
//! The Ethereum tests directory.
//!

pub mod test;

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::directories::Collection;
use crate::environment::Environment;
use crate::filters::Filters;
use crate::summary::Summary;

use self::test::EthereumTest;

///
/// The Ethereum tests directory.
///
pub struct EthereumDirectory;

impl EthereumDirectory {
    ///
    /// The upstream test index file name.
    ///
    /// This version if the index used for the EVM Interpreter environment.
    ///
    const INDEX_NAME_UPSTREAM_EVM_INTERPRETER: &'static str = "ethereum_evm_interpreter.yaml";

    ///
    /// The upstream test index file name.
    ///
    /// This version if the index used for the REVM environment.
    ///
    const INDEX_NAME_UPSTREAM_REVM: &'static str = "ethereum_revm.yaml";

    ///
    /// The ZKsync test index file name.
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
        target: benchmark_converter::Target,
        environment: Environment,
        directory_path: &Path,
        _extension: &'static str,
        summary: Arc<Mutex<Summary>>,
        filters: &Filters,
    ) -> anyhow::Result<Vec<Self::Test>> {
        let index_path = match (target, environment) {
            (benchmark_converter::Target::EraVM, _) => {
                let mut index_path = directory_path.to_path_buf();
                index_path.push(Self::INDEX_NAME_ZKSYNC);
                index_path
            }
            (benchmark_converter::Target::EVM, Environment::EVMInterpreter) => {
                PathBuf::from(Self::INDEX_NAME_UPSTREAM_EVM_INTERPRETER)
            }
            (benchmark_converter::Target::EVM, Environment::REVM) => {
                PathBuf::from(Self::INDEX_NAME_UPSTREAM_REVM)
            }
            (target, environment) => anyhow::bail!(
                "Unsupported target/environment combination: {target:?}/{environment:?}"
            ),
        };

        Ok(Self::read_index(index_path.as_path())?
            .into_enabled_list(directory_path)
            .into_iter()
            .filter_map(|test| EthereumTest::new(test, summary.clone(), filters))
            .collect())
    }
}
