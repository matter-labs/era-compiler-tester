//!
//! The semantic test instance.
//!

pub mod function_call;
pub mod params;

use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;

use self::function_call::FunctionCall;
use self::params::Params;

///
/// The semantic test instance.
///
#[derive(Debug, PartialEq, Eq)]
pub struct Test {
    /// The source code files.
    pub sources: Vec<(String, String)>,
    /// The test params.
    pub params: Params,
    /// The function calls.
    pub calls: Vec<FunctionCall>,
}

impl TryFrom<&Path> for Test {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let mut file = fs::File::open(path)?;

        let mut data = String::new();
        file.read_to_string(&mut data)
            .map_err(|error| anyhow::anyhow!("Failed to read test file: {}", error))?;

        let comment_start = if path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("Failed to get file extension"))?
            == era_compiler_common::EXTENSION_VYPER
        {
            "# ".to_owned()
        } else {
            "// ".to_owned()
        };

        let sources = process_sources(&data, path)?;

        let (data, function_calls) = data
            .split_once(&format!("{comment_start}----\n"))
            .ok_or_else(|| anyhow::anyhow!("Invalid test format"))?;

        let params = data
            .split_once(&format!("{comment_start}====\n"))
            .map(|parts| parts.1)
            .unwrap_or_default();

        let params = params
            .lines()
            .filter_map(|line| line.strip_prefix(&comment_start))
            .map(|line| {
                let mut line = line.to_owned();
                line.push('\n');
                line
            })
            .collect::<Vec<String>>()
            .join("");

        let params = Params::try_from(params.as_str())
            .map_err(|err| anyhow::anyhow!("Failed to parse params: {}", err))?;

        let function_calls = function_calls
            .lines()
            .filter_map(|line| line.strip_prefix(&comment_start))
            .map(|line| {
                let mut line = line.to_owned();
                line.push('\n');
                line
            })
            .collect::<Vec<String>>()
            .join("");

        let calls = FunctionCall::parse_calls(function_calls.as_str())
            .map_err(|err| anyhow::anyhow!("Failed to parse function calls: {}", err))?;

        Ok(Self {
            sources,
            params,
            calls,
        })
    }
}

///
/// Returns sources.
///
fn process_sources(data: &str, path: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut sources = Vec::new();

    let mut source_name = None;
    let mut source = String::new();

    let regex = Regex::new(r"^==== (.*): (.*) ====$").expect("Always valid");

    for (index, line) in data.lines().enumerate() {
        let captures = match regex.captures(line) {
            Some(captures) => captures,
            None => {
                source.push_str(line);
                source.push('\n');
                continue;
            }
        };

        match captures.get(1).expect("Always exists").as_str() {
            "ExternalSource" => {
                let data = captures.get(2).expect("Always exists").as_str();
                let regex = Regex::new("^([^=]*)=(.*)$").expect("Always valid");
                let (name, relative_path) = match regex.captures(data) {
                    Some(captures) => (
                        captures.get(1).expect("Always exists").as_str().to_owned(),
                        PathBuf::from(captures.get(2).expect("Always exists").as_str()),
                    ),
                    None => (data.to_owned(), PathBuf::from(data)),
                };

                let path = path
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory of file"))?
                    .join(relative_path);

                let mut file = fs::File::open(path)?;

                let mut data = String::new();
                file.read_to_string(&mut data).map_err(|error| {
                    anyhow::anyhow!("Failed to read source code file: {}", error)
                })?;

                sources.push((name, data));
            }
            "Source" => {
                let name = captures.get(2).expect("Always exists").as_str().to_owned();

                if let Some(source_name) = source_name {
                    sources.push((source_name, source));
                    source = String::new();
                }

                // For sources without names
                if !source.is_empty() {
                    sources.push((String::new(), source));
                    source = String::new();
                }

                source_name = Some(name);
            }
            word => anyhow::bail!(
                "Expected \"Source\" or \"ExternalSource\" on line {}, found: {}",
                index + 1,
                word
            ),
        }
    }

    if !source.is_empty() {
        let name = match source_name {
            Some(source_name) => source_name,
            None => path.to_string_lossy().to_string(),
        };
        sources.push((name, source));
    }

    Ok(sources)
}
