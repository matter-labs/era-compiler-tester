//!
//! The compiler tester binary.
//!

pub(crate) mod arguments;

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use colored::Colorize;

use self::arguments::Arguments;

/// The rayon worker stack size.
const RAYON_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

///
/// The application entry point.
///
fn main() {
    match main_inner(Arguments::new()) {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("{error:?}");
            std::process::exit(1)
        }
    }
}

///
/// The entry point wrapper used for proper error handling.
///
fn main_inner(arguments: Arguments) -> anyhow::Result<()> {
    println!(
        "    {} {} v{} (LLVM build {})",
        "Starting".bright_green().bold(),
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_VERSION"),
        inkwell::support::get_commit_id().to_string(),
    );

    let target = match arguments.target {
        Some(target) => era_compiler_llvm_context::Target::from_str(target.as_str())?,
        None => era_compiler_llvm_context::Target::EraVM,
    };

    inkwell::support::enable_llvm_pretty_stack_trace();
    era_compiler_llvm_context::initialize_target(target);
    compiler_tester::LLVMOptions::initialize(
        arguments.llvm_verify_each,
        arguments.llvm_debug_logging,
    )?;
    era_compiler_solidity::EXECUTABLE
        .set(
            arguments
                .zksolc
                .unwrap_or_else(|| PathBuf::from(era_compiler_solidity::DEFAULT_EXECUTABLE_NAME)),
        )
        .expect("Always valid");
    era_compiler_vyper::EXECUTABLE
        .set(
            arguments
                .zkvyper
                .unwrap_or_else(|| PathBuf::from(era_compiler_vyper::DEFAULT_EXECUTABLE_NAME)),
        )
        .expect("Always valid");

    let debug_config = if arguments.debug {
        std::fs::create_dir_all(compiler_tester::DEBUG_DIRECTORY)?;
        Some(era_compiler_llvm_context::DebugConfig::new(
            PathBuf::from_str(compiler_tester::DEBUG_DIRECTORY)?,
        ))
    } else {
        None
    };

    if arguments.trace > 0 {
        std::fs::create_dir_all(compiler_tester::TRACE_DIRECTORY)?;
    }

    let mut thread_pool_builder = rayon::ThreadPoolBuilder::new();
    if let Some(threads) = arguments.threads {
        thread_pool_builder = thread_pool_builder.num_threads(threads);
    }
    thread_pool_builder
        .stack_size(RAYON_WORKER_STACK_SIZE)
        .build_global()
        .expect("Thread pool configuration failure");

    let summary = compiler_tester::Summary::new(arguments.verbosity, arguments.quiet).wrap();

    let filters = compiler_tester::Filters::new(arguments.paths, arguments.modes, arguments.groups);

    let compiler_tester = compiler_tester::CompilerTester::new(
        summary.clone(),
        filters,
        debug_config.clone(),
        arguments.workflow,
    )?;

    let binary_download_config_paths = vec![
        arguments
            .solc_bin_config_path
            .unwrap_or_else(|| PathBuf::from("./configs/solc-bin-default.json")),
        arguments
            .vyper_bin_config_path
            .unwrap_or_else(|| PathBuf::from("./configs/vyper-bin-default.json")),
    ];

    let run_time_start = Instant::now();
    println!(
        "     {} tests with {} worker threads",
        "Running".bright_green().bold(),
        rayon::current_num_threads(),
    );

    match target {
        era_compiler_llvm_context::Target::EraVM => {
            zkevm_tester::runners::compiler_tests::set_tracing_mode(
                zkevm_tester::runners::compiler_tests::VmTracingOptions::from_u64(
                    arguments.trace as u64,
                ),
            );
            zkevm_assembly::set_encoding_mode(zkevm_assembly::RunningVmEncodingMode::Testing);

            let system_contract_debug_config = if arguments.dump_system {
                debug_config
            } else {
                None
            };
            let vm = compiler_tester::EraVM::new(
                binary_download_config_paths,
                PathBuf::from("./configs/solc-bin-system-contracts.json"),
                system_contract_debug_config,
                arguments.system_contracts_load_path,
                arguments.system_contracts_save_path,
            )?;

            match (
                arguments.disable_deployer,
                arguments.disable_value_simulator,
            ) {
                (true, true) => {
                    compiler_tester.run_eravm::<compiler_tester::EraVMNativeDeployer, false>(vm)?
                }
                (true, false) => {
                    compiler_tester.run_eravm::<compiler_tester::EraVMNativeDeployer, true>(vm)?
                }
                (false, true) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMSystemContractDeployer, false>(vm)?,
                (false, false) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMSystemContractDeployer, true>(vm)?,
            }
        }
        era_compiler_llvm_context::Target::EVM => {
            compiler_tester::EVM::download(binary_download_config_paths)?;
            compiler_tester.run_evm()?;
        }
    }

    let summary = compiler_tester::Summary::unwrap_arc(summary);
    print!("{summary}");
    println!(
        "    {} running tests in {}m{:02}s",
        "Finished".bright_green().bold(),
        run_time_start.elapsed().as_secs() / 60,
        run_time_start.elapsed().as_secs() % 60,
    );

    if let Some(path) = arguments.benchmark {
        let benchmark = summary.benchmark()?;
        benchmark.write_to_file(path)?;
    }

    if !summary.is_successful() {
        anyhow::bail!("");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manually() {
        zkevm_tester::runners::compiler_tests::set_tracing_mode(
            zkevm_tester::runners::compiler_tests::VmTracingOptions::ManualVerbose,
        );

        let arguments = Arguments {
            verbosity: false,
            quiet: false,
            debug: false,
            trace: 2,
            modes: vec!["Y+M3B3 0.8.25".to_owned()],
            paths: vec!["./tests/solidity/simple/default.sol".to_owned()],
            groups: vec![],
            benchmark: None,
            threads: Some(1),
            dump_system: false,
            disable_deployer: false,
            disable_value_simulator: false,
            zksolc: Some(PathBuf::from(
                era_compiler_solidity::DEFAULT_EXECUTABLE_NAME,
            )),
            zkvyper: Some(PathBuf::from(era_compiler_vyper::DEFAULT_EXECUTABLE_NAME)),
            target: Some(era_compiler_llvm_context::Target::EraVM.to_string()),
            solc_bin_config_path: Some(PathBuf::from("./configs/solc-bin-default.json")),
            vyper_bin_config_path: Some(PathBuf::from("./configs/vyper-bin-default.json")),
            system_contracts_load_path: None,
            system_contracts_save_path: None,
            llvm_verify_each: false,
            llvm_debug_logging: false,
            workflow: compiler_tester::Workflow::BuildAndRun,
        };

        main_inner(arguments).expect("Manual testing failed");
    }
}
