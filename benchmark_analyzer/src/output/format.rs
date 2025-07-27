//!
//! Output benchmark format.
//!

///
/// Output benchmark format.
///
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Format {
    #[default]
    /// Unstable JSON format, corresponds to the inner data model of benchmark analyzer.
    Json,
    /// CSV format.
    Csv,
    /// JSON format compatible with LNT.
    JsonLNT,
    /// Excel spreadsheet format.
    Xlsx,
}

impl std::str::FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "json-lnt" => Ok(Self::JsonLNT),
            "csv" => Ok(Self::Csv),
            "xlsx" => Ok(Self::Xlsx),
            string => anyhow::bail!(
                "Unknown benchmark format `{string}`. Supported formats: {}",
                vec![Self::Json, Self::JsonLNT, Self::Csv, Self::Xlsx]
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
            Format::Json => write!(f, "json"),
            Format::JsonLNT => write!(f, "json-lnt"),
            Format::Csv => write!(f, "csv"),
            Format::Xlsx => write!(f, "xlsx"),
        }
    }
}
