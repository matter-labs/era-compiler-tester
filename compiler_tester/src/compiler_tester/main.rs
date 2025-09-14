//!
//! The compiler tester executable.
//!

pub(crate) mod arguments;

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use clap::Parser;
use colored::Colorize;

use self::arguments::Arguments;

/// The rayon worker stack size.
const RAYON_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

///
/// The application entry point.
///
fn main() {
    let exit_code = match Arguments::try_parse()
        .map_err(|error| anyhow::anyhow!(error))
        .and_then(main_inner)
    {
        Ok(()) => era_compiler_common::EXIT_CODE_SUCCESS,
        Err(error) => {
            eprintln!("{error:?}");
            era_compiler_common::EXIT_CODE_FAILURE
        }
    };
    unsafe { inkwell::support::shutdown_llvm() };
    std::process::exit(exit_code);
}

///
/// The entry point wrapper used for proper error handling.
///
fn main_inner(arguments: Arguments) -> anyhow::Result<()> {
    let arguments = Arguments::validate(arguments)?;
    println!(
        "    {} {} v{} (LLVM build {})",
        "Starting".bright_green().bold(),
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_VERSION"),
        inkwell::support::get_commit_id().to_string(),
    );

    inkwell::support::enable_llvm_pretty_stack_trace();
    era_compiler_llvm_context::initialize_target();
    solx_codegen_evm::initialize_target();
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

    let mut thread_pool_builder = rayon::ThreadPoolBuilder::new();
    if let Some(threads) = arguments.threads {
        thread_pool_builder = thread_pool_builder.num_threads(threads);
    }
    thread_pool_builder
        .stack_size(RAYON_WORKER_STACK_SIZE)
        .build_global()
        .expect("Thread pool configuration failure");

    let target = arguments.target;
    let toolchain = arguments
        .toolchain
        .unwrap_or(compiler_tester::Toolchain::IrLLVM);
    let environment = match (target, arguments.environment) {
        (
            benchmark_analyzer::Target::EraVM,
            Some(environment @ compiler_tester::Environment::ZkEVM),
        ) => environment,
        (benchmark_analyzer::Target::EraVM, None) => compiler_tester::Environment::ZkEVM,
        (
            benchmark_analyzer::Target::EVM,
            Some(environment @ compiler_tester::Environment::EVMInterpreter),
        ) => environment,
        (
            benchmark_analyzer::Target::EVM,
            Some(environment @ compiler_tester::Environment::REVM),
        ) => environment,
        (benchmark_analyzer::Target::EVM, None) => compiler_tester::Environment::EVMInterpreter,
        (target, Some(environment)) => anyhow::bail!(
            "Target `{target}` and environment `{environment}` combination is not supported"
        ),
    };

    let mut executable_download_config_paths = Vec::with_capacity(2);
    if let Some(path) = match (target, toolchain) {
        (benchmark_analyzer::Target::EVM, compiler_tester::Toolchain::IrLLVM) => None,
        (benchmark_analyzer::Target::EraVM, compiler_tester::Toolchain::IrLLVM) => {
            Some("./configs/solc-bin-default.json")
        }
        (_, compiler_tester::Toolchain::Zksolc) => Some("./configs/solc-bin-default.json"),
        (_, compiler_tester::Toolchain::Solc) => Some("./configs/solc-bin-upstream.json"),
        (_, compiler_tester::Toolchain::SolcLLVM) => Some("./configs/solc-bin-llvm.json"),
    }
    .map(PathBuf::from)
    {
        executable_download_config_paths.push(path);
    }
    executable_download_config_paths.push(
        arguments
            .vyper_bin_config_path
            .unwrap_or_else(|| PathBuf::from("./configs/vyper-bin-default.json")),
    );

    let summary = compiler_tester::Summary::new(arguments.verbose, arguments.quiet).wrap();

    let filters = compiler_tester::Filters::new(arguments.path, arguments.mode, arguments.group);

    let compiler_tester = compiler_tester::CompilerTester::new(
        summary.clone(),
        filters,
        debug_config.clone(),
        arguments.workflow,
    )?;

    let run_time_start = Instant::now();
    println!(
        "     {} tests with {} worker threads",
        "Running".bright_green().bold(),
        rayon::current_num_threads(),
    );

    match environment {
        compiler_tester::Environment::ZkEVM => {
            let system_contracts_debug_config = if arguments.dump_system {
                debug_config
            } else {
                None
            };
            let vm = compiler_tester::EraVM::new(
                executable_download_config_paths,
                PathBuf::from("./configs/solc-bin-system-contracts.json"),
                system_contracts_debug_config,
                arguments.load_system_contracts,
                arguments.save_system_contracts,
                arguments.target,
            )?;

            match (
                arguments.disable_deployer,
                arguments.disable_value_simulator,
            ) {
                (true, true) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMNativeDeployer, false>(vm, toolchain),
                (true, false) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMNativeDeployer, true>(vm, toolchain),
                (false, true) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMSystemContractDeployer, false>(
                        vm, toolchain,
                    ),
                (false, false) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMSystemContractDeployer, true>(vm, toolchain),
            }
        }
        compiler_tester::Environment::EVMInterpreter => {
            let system_contract_debug_config = if arguments.dump_system {
                debug_config
            } else {
                None
            };
            let vm = compiler_tester::EraVM::new(
                executable_download_config_paths,
                PathBuf::from("./configs/solc-bin-system-contracts.json"),
                system_contract_debug_config,
                arguments.load_system_contracts,
                arguments.save_system_contracts,
                arguments.target,
            )?;

            compiler_tester
                .run_evm_interpreter::<compiler_tester::EraVMSystemContractDeployer, true>(
                    vm,
                    toolchain,
                    arguments.solx,
                )
        }
        compiler_tester::Environment::REVM => {
            compiler_tester::REVM::download(executable_download_config_paths)?;
            compiler_tester.run_revm(toolchain, arguments.solx)
        }
    }?;

    let summary = compiler_tester::Summary::unwrap_arc(summary);
    print!("{summary}");
    println!(
        "    {} running tests in {}m{:02}s",
        "Finished".bright_green().bold(),
        run_time_start.elapsed().as_secs() / 60,
        run_time_start.elapsed().as_secs() % 60,
    );

    if let Some(path) = arguments.benchmark {
        let benchmark = summary.benchmark(toolchain)?;
        let output: benchmark_analyzer::Output = (
            benchmark,
            benchmark_analyzer::InputSource::CompilerTester,
            arguments.benchmark_format,
        )
            .try_into()?;
        output.write_to_file(path)?;
    }

    if !summary.is_successful() {
        anyhow::bail!("");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::arguments::Arguments;

    #[test]
    fn test_manually() {
        std::env::set_current_dir("..").expect("Change directory failed");

        let arguments = Arguments {
            verbose: false,
            quiet: false,
            debug: false,
            mode: vec!["Y+M3B3 0.8.30".to_owned()],
            path: vec!["tests/solidity/simple/default.sol".to_owned()],
            group: vec![],
            benchmark: None,
            benchmark_format: benchmark_analyzer::OutputFormat::Json,
            benchmark_context: None,
            threads: Some(1),
            dump_system: false,
            disable_deployer: false,
            disable_value_simulator: false,
            zksolc: None,
            zkvyper: None,
            solx: Some(PathBuf::from("solx")),
            toolchain: Some(compiler_tester::Toolchain::IrLLVM),
            target: benchmark_analyzer::Target::EVM,
            environment: None,
            workflow: compiler_tester::Workflow::BuildAndRun,
            solc_bin_config_path: Some(PathBuf::from("./configs/solc-bin-default.json")),
            vyper_bin_config_path: Some(PathBuf::from("./configs/vyper-bin-default.json")),
            load_system_contracts: Some(PathBuf::from("system-contracts-stable-build")),
            save_system_contracts: None,
            llvm_verify_each: false,
            llvm_debug_logging: false,
        };

        crate::main_inner(arguments).expect("Manual testing failed");
    }
}
