//!
//! Converts `[InputIdentifier]` to the representation used by the benchmark.
//!

use crate::test::case::input::identifier::InputIdentifier;

///
/// Converts `[InputIdentifier]` to the representation used by the benchmark.
///
pub fn convert_input(input: InputIdentifier) -> benchmark_analyzer::Input {
    match input {
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
    }
}
