//!
//! The coverage watcher binary.
//!

pub(crate) mod arguments;

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use self::arguments::Arguments;

use coverage_watcher::TestsDirectory;
use coverage_watcher::TestsSet;

///
/// The application entry point.
///
fn main() -> anyhow::Result<()> {
    let arguments = Arguments::new();

    let mut data = String::new();
    File::open("coverage.yaml")
        .expect("Failed to open ignore file")
        .read_to_string(&mut data)
        .expect("Failed to read ignore file");

    let ignore_file: coverage_watcher::IgnoreFileEntity =
        serde_yaml::from_str(data.as_str()).expect("Invalid ignore file");

    let missed = TestsSet::get_missed_for_groups(
        vec![
            vec![
                TestsSet {
                    name: "solidity/simple".to_string(),
                    directories: vec![TestsDirectory {
                        path: PathBuf::from("tests/solidity/simple"),
                        extension: era_compiler_common::EXTENSION_SOLIDITY.to_string(),
                        flatten: false,
                    }],
                },
                TestsSet {
                    name: "vyper/simple".to_string(),
                    directories: vec![TestsDirectory {
                        path: PathBuf::from("tests/vyper/simple"),
                        extension: era_compiler_common::EXTENSION_VYPER.to_string(),
                        flatten: false,
                    }],
                },
            ],
            vec![
                TestsSet {
                    name: "solidity/complex".to_string(),
                    directories: vec![TestsDirectory {
                        path: PathBuf::from("tests/solidity/complex"),
                        extension: era_compiler_common::EXTENSION_JSON.to_string(),
                        flatten: false,
                    }],
                },
                TestsSet {
                    name: "vyper/complex".to_string(),
                    directories: vec![TestsDirectory {
                        path: PathBuf::from("tests/vyper/complex"),
                        extension: era_compiler_common::EXTENSION_JSON.to_string(),
                        flatten: false,
                    }],
                },
            ],
            vec![
                TestsSet {
                    name: "solidity/external".to_string(),
                    directories: vec![TestsDirectory {
                        path: PathBuf::from("solidity/test/libsolidity/semanticTests"),
                        extension: era_compiler_common::EXTENSION_SOLIDITY.to_string(),
                        flatten: false,
                    }],
                },
                TestsSet {
                    name: "vyper/external".to_string(),
                    directories: vec![
                        TestsDirectory {
                            path: PathBuf::from("tests/vyper/external"),
                            extension: era_compiler_common::EXTENSION_VYPER.to_string(),
                            flatten: false,
                        },
                        TestsDirectory {
                            path: PathBuf::from("tests/vyper/complex/external"),
                            extension: era_compiler_common::EXTENSION_JSON.to_string(),
                            flatten: true,
                        },
                    ],
                },
            ],
        ],
        &ignore_file,
    )?;

    let mut result = String::new();
    for (name, tests) in missed {
        if tests.is_empty() {
            continue;
        }
        result.push_str(name.as_str());
        result.push('\n');
        for test in tests.into_iter() {
            result.push_str(test.as_str());
            result.push('\n');
        }
        result.push('\n');
    }

    match arguments.output {
        Some(path) => {
            let mut file = File::create(path.as_path()).expect("Failed to create output file");
            file.write_all(result.as_bytes())
                .expect("Failed to write result");
        }
        None => println!(
            "{}",
            if result.is_empty() {
                "No missing tests found".to_owned()
            } else {
                result
            }
        ),
    }

    Ok(())
}
