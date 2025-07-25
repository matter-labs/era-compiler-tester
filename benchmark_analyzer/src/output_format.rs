//!
//! Output format for benchmark data.
//!

///
/// Output format for benchmark data.
///
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OutputFormat {
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

impl std::str::FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "json-lnt" => Ok(Self::JsonLNT),
            "csv" => Ok(Self::Csv),
            "xlsx" => Ok(Self::Xlsx),
            string => anyhow::bail!(
                "Unknown benchmark format `{string}`. Supported formats: {}",
                vec![Self::Json, Self::Csv]
                    .into_iter()
                    .map(|element| element.to_string().to_lowercase())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::JsonLNT => write!(f, "json-lnt"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Xlsx => write!(f, "xlsx"),
        }
    }
}
