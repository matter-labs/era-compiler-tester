//!
//! The compiler downloader binary config.
//!

pub mod protocol;

use serde::Deserialize;

use self::protocol::Protocol;

///
/// The compiler downloader binary config.
///
#[derive(Debug, Deserialize)]
pub struct Binary {
    /// Whether downloading the binary is enabled.
    pub is_enabled: bool,
    /// The downloading protocol.
    pub protocol: Protocol,
    /// The downloaded data source.
    pub source: String,
    /// The downloaded binary file destination.
    pub destination: String,
}
