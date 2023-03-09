//!
//! The compiler downloader binary download protocol.
//!

use serde::Deserialize;

///
/// The compiler downloader binary download protocol.
///
#[derive(Debug, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    /// The local file copy.
    #[serde(rename = "file")]
    File,
    /// Download via HTTPS.
    #[serde(rename = "https")]
    HTTPS,
    /// Use the solc-bin JSON list.
    #[serde(rename = "solc-bin-list")]
    SolcBinList,
}
