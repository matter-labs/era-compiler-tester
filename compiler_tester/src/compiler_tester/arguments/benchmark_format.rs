/// Output format for benchmark data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BenchmarkFormat {
    #[default]
    Json,
    Csv,
}

impl std::str::FromStr for BenchmarkFormat {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            string => anyhow::bail!(
                "Unknown benchmark format `{}`. Supported formats: {}",
                string,
                vec![Self::Json, Self::Csv]
                    .into_iter()
                    .map(|element| element.to_string().to_lowercase())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl std::fmt::Display for BenchmarkFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            BenchmarkFormat::Json => "json",
            BenchmarkFormat::Csv => "csv",
        };
        f.write_str(repr)
    }
}
