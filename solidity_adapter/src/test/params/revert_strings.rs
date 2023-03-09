//!
//! revertStrings param values.
//!

///
/// revertStrings param values.
///
#[derive(Debug, PartialEq, Eq)]
pub enum RevertStrings {
    /// `debug` in the metadata.
    Debug,
    /// not specified in the metadata.
    Default,
}

impl TryFrom<&str> for RevertStrings {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "debug" => RevertStrings::Debug,
            word => anyhow::bail!(
                r#"Expected "debug" as revertStrings value, found: {}"#,
                word
            ),
        })
    }
}
