//!
//! Converts `[InputIdentifier]` to the representation used by the benchmark.
//!

use crate::test::case::input::identifier::InputIdentifier;

impl From<InputIdentifier> for benchmark_analyzer::Input {
    ///
    /// Converts `[InputIdentifier]` to the representation used by the benchmark.
    ///
    fn from(val: InputIdentifier) -> Self {
        match val {
            InputIdentifier::Deployer {
                contract_identifier,
            } => benchmark_analyzer::Input::Deployer {
                contract_identifier,
            },
            InputIdentifier::Runtime { input_index, name } => {
                benchmark_analyzer::Input::Runtime { input_index, name }
            }
            InputIdentifier::StorageEmpty { input_index } => {
                benchmark_analyzer::Input::StorageEmpty { input_index }
            }
            InputIdentifier::Balance { input_index } => {
                benchmark_analyzer::Input::Balance { input_index }
            }
            InputIdentifier::Fallback { input_index } => {
                benchmark_analyzer::Input::Fallback { input_index }
            }
        }
    }
}
