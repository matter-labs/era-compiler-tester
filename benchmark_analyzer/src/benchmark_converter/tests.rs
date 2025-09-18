//!
//! Tests for the benchmark converter.
//!

#![cfg(test)]

use chrono::Utc;

#[test]
fn convert() {
    let context = benchmark_analyzer::BenchmarkContext {
        machine: "default".to_owned(),
        toolchain: "solx".to_owned(),
        compiler_version: semver::Version::new(1, 0, 0).to_string(),
        llvm_version: semver::Version::new(19, 1, 0).to_string(),
        target: benchmark_analyzer::Target::EVM,

        codegen: Some("Y+".to_owned()),
        optimization: Some("M3B3".to_owned()),
    };
    let metadata = benchmark_analyzer::BenchmarkMetadata {
        start: Utc::now(),
        end: Utc::now(),
        context: Some(context),
    };

    let foundry_report_1 = r#"
    { "toolchain": "solx", "project": "ProjectX", "data": [ {
        "contract": "src/test/utils/mocks/MockAuthority.sol:MockAuthority",
        "deployment": { "gas": 111281, "size": 406 },
        "functions": {
            "canCall(address,address,bytes4)": {
                "calls": 5654,
                "min": 462,
                "mean": 462,
                "median": 462,
                "max": 462
            },
            "allowance(address,address)": {
                "calls": 1801,
                "min": 753,
                "mean": 753,
                "median": 753,
                "max": 753
            }
        }
    }, {
        "contract": "src/test/utils/mocks/MockAuthorityHarder.sol:MockAuthorityHarder",
        "deployment": { "gas": 111281, "size": 406 },
        "functions": {
            "canCall(address,address)": {
                "calls": 5654,
                "min": 462,
                "mean": 462,
                "median": 462,
                "max": 462
            },
            "allowance(address,address)": {
                "calls": 1801,
                "min": 753,
                "mean": 753,
                "median": 753,
                "max": 753
            }
        }
    } ] }"#;
    let input_1 = serde_json::from_str::<benchmark_analyzer::InputReport>(foundry_report_1)
        .expect("Failed to parse foundry report");

    let foundry_report_2 = r#"
    { "toolchain": "solx", "project": "ProjectY", "data": [ {
        "contract": "src/test/StrangeFile.sol:StrangeContract",
        "deployment": { "gas": 99999, "size": 1111 },
        "functions": {
            "anotherCall": {
                "calls": 15,
                "mean": 9000
            },
            "yetAnotherCall": {
                "calls": 5,
                "mean": 999
            }
        }
    }, {
        "contract": "src/test/OrdinaryFile.sol:OrdinaryContract",
        "deployment": { "gas": 99999, "size": 1111 },
        "functions": {
            "ordinaryCall": {
                "calls": 15,
                "mean": 9000
            },
            "evenMoreOrdinaryCall": {
                "calls": 5,
                "mean": 999
            }
        }
    } ] }"#;
    let input_2 = serde_json::from_str::<benchmark_analyzer::InputReport>(foundry_report_2)
        .expect("Failed to parse foundry report");

    let mut benchmark = benchmark_analyzer::Benchmark::new(metadata);
    benchmark
        .extend(input_1)
        .expect("Failed to extend a benchmark report with a Foundry report");
    benchmark
        .extend(input_2)
        .expect("Failed to extend a benchmark report with a Foundry report");

    let output: benchmark_analyzer::Output = (
        benchmark,
        benchmark_analyzer::InputSource::Tooling,
        benchmark_analyzer::OutputFormat::JsonLNT,
    )
        .try_into()
        .expect("Failed to convert a benchmark report to output");
    let contents = match output {
        benchmark_analyzer::Output::SingleFile(file) => file,
        benchmark_analyzer::Output::MultipleFiles(mut files) => files.remove(0).content,
        _ => unimplemented!(),
    };

    eprintln!("Contents: {contents}");
    assert!(contents.contains("ProjectX::src/test/utils/mocks/MockAuthority.sol:MockAuthority[#deployer:mocks/MockAuthority.sol:MockAuthority]"));
    assert!(contents.contains("ProjectX::src/test/utils/mocks/MockAuthorityHarder.sol:MockAuthorityHarder[#deployer:mocks/MockAuthorityHarder.sol:MockAuthorityHarder]"));
    assert!(contents.contains("ProjectX::src/test/utils/mocks/MockAuthority.sol:MockAuthority[allowance(address,address):1]"));
    assert!(contents.contains("ProjectX::src/test/utils/mocks/MockAuthority.sol:MockAuthority[canCall(address,address,bytes4):2]"));
    assert!(contents.contains("ProjectX::src/test/utils/mocks/MockAuthorityHarder.sol:MockAuthorityHarder[allowance(address,address):1]"));
    assert!(contents.contains("ProjectX::src/test/utils/mocks/MockAuthorityHarder.sol:MockAuthorityHarder[canCall(address,address):2]"));

    assert!(contents
        .contains("ProjectY::src/test/StrangeFile.sol:StrangeContract[#deployer:test/StrangeFile.sol:StrangeContract]"));
    assert!(contents
        .contains("ProjectY::src/test/OrdinaryFile.sol:OrdinaryContract[#deployer:test/OrdinaryFile.sol:OrdinaryContract]"));
    assert!(contents.contains("ProjectY::src/test/StrangeFile.sol:StrangeContract[anotherCall:1]"));
    assert!(
        contents.contains("ProjectY::src/test/StrangeFile.sol:StrangeContract[yetAnotherCall:2]")
    );
    assert!(contents
        .contains("ProjectY::src/test/OrdinaryFile.sol:OrdinaryContract[evenMoreOrdinaryCall:1]"));
    assert!(
        contents.contains("ProjectY::src/test/OrdinaryFile.sol:OrdinaryContract[ordinaryCall:2]")
    );
}
