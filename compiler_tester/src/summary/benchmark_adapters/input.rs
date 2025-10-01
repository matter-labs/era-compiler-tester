//!
//! Converts `[InputIdentifier]` to the representation used by the benchmark.
//!

use crate::test::case::input::identifier::InputIdentifier;

impl From<InputIdentifier> for benchmark_converter::Input {
    ///
    /// Converts `[InputIdentifier]` to the representation used by the benchmark.
    ///
    fn from(val: InputIdentifier) -> Self {
        match val {
            InputIdentifier::Deployer {
                contract_identifier,
            } => benchmark_converter::Input::Deployer {
                contract_identifier,
            },
            InputIdentifier::Runtime { input_index, name } => {
                benchmark_converter::Input::Runtime { input_index, name }
            }
            InputIdentifier::StorageEmpty { input_index } => {
                benchmark_converter::Input::StorageEmpty { input_index }
            }
            InputIdentifier::Balance { input_index } => {
                benchmark_converter::Input::Balance { input_index }
            }
            InputIdentifier::Fallback { input_index } => {
                benchmark_converter::Input::Fallback { input_index }
            }
        }
    }
}
