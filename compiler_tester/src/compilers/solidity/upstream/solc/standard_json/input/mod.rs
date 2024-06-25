//!
//! The `solc --standard-json` input.
//!

pub mod language;
pub mod settings;
pub mod source;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use serde::Serialize;
use settings::debug::DebugOpt;

use self::settings::optimizer::Optimizer as SolcStandardJsonInputSettingsOptimizer;
use self::settings::selection::Selection as SolcStandardJsonInputSettingsSelection;

use self::language::Language;
use self::settings::Settings;
use self::source::Source;

///
/// The `solc --standard-json` input.
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    /// The input language.
    pub language: Language,
    /// The input source code files hashmap.
    pub sources: BTreeMap<String, Source>,
    /// The compiler settings.
    pub settings: Settings,
}

impl Input {
    ///
    /// A shortcut constructor from source code.
    ///
    /// Only for the integration test purposes.
    ///
    pub fn try_from_sources(
        language: Language,
        evm_version: Option<era_compiler_common::EVMVersion>,
        sources: BTreeMap<String, String>,
        libraries: BTreeMap<String, BTreeMap<String, String>>,
        remappings: Option<BTreeSet<String>>,
        output_selection: SolcStandardJsonInputSettingsSelection,
        optimizer: SolcStandardJsonInputSettingsOptimizer,
        via_ir: bool,
    ) -> anyhow::Result<Self> {
        let sources = sources
            .into_par_iter()
            .map(|(path, content)| (path, Source::from(content)))
            .collect();

        Ok(Self {
            language,
            sources,
            settings: Settings::new(
                evm_version,
                libraries,
                remappings,
                output_selection,
                via_ir,
                optimizer,
                DebugOpt::new("debug".to_owned())
            ),
        })
    }
}
