//!
//! Input benchmark format.
//!

///
/// Input benchmark format.
///
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Format {
    #[default]
    /// Foundry benchmark report format.
    Foundry,
}

impl std::str::FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "foundry" => Ok(Self::Foundry),
            string => anyhow::bail!(
                "Unknown benchmark format `{string}`. Supported formats: {}",
                vec![Self::Foundry]
                    .into_iter()
                    .map(|element| element.to_string().to_lowercase())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Foundry => write!(f, "foundry"),
        }
    }
}
