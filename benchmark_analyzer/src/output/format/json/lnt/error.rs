//!
//! Errors occuring during generation of LNT-compatible JSON files.
//!

///
/// Errors occuring during generation of LNT-compatible JSON files.
///
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LntSerializationError {
    ///
    /// No instance of [crate::model::context::Context] is provided.
    ///
    NoContext,
}

impl std::fmt::Display for LntSerializationError {
    #[allow(unreachable_patterns)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LntSerializationError::NoContext => f.write_str("LNT backend requires explicitly passed benchmark context, but no context was provided."),
            _ => f.write_fmt(format_args!("{self:?}")),
        }
    }
}
impl std::error::Error for LntSerializationError {}
