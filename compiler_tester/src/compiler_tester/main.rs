//!
//! The compiler tester executable.
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
    let exit_code = match main_inner(Arguments::new()) {
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
    println!(
        "    {} {} v{} (LLVM build {})",
        "Starting".bright_green().bold(),
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_VERSION"),
        inkwell::support::get_commit_id().to_string(),
    );

    inkwell::support::enable_llvm_pretty_stack_trace();
    for target in [
        era_compiler_common::Target::EraVM,
        era_compiler_common::Target::EVM,
    ]
    .into_iter()
    {
        era_compiler_llvm_context::initialize_target(target);
    }
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

    let summary = compiler_tester::Summary::new(arguments.verbosity, arguments.quiet).wrap();

    let filters = compiler_tester::Filters::new(arguments.paths, arguments.modes, arguments.groups);

    let compiler_tester = compiler_tester::CompilerTester::new(
        summary.clone(),
        filters,
        debug_config.clone(),
        arguments.workflow,
    )?;

    let toolchain = match (arguments.target, arguments.toolchain) {
        (era_compiler_common::Target::EraVM, Some(toolchain)) => toolchain,
        (era_compiler_common::Target::EraVM, None) => compiler_tester::Toolchain::IrLLVM,
        (era_compiler_common::Target::EVM, Some(toolchain)) => toolchain,
        (era_compiler_common::Target::EVM, None) => compiler_tester::Toolchain::Solc,
    };
    let executable_download_config_paths = vec![
        arguments.solc_bin_config_path.unwrap_or_else(|| {
            PathBuf::from(match toolchain {
                compiler_tester::Toolchain::IrLLVM => "./configs/solc-bin-default.json",
                compiler_tester::Toolchain::Solc => "./configs/solc-bin-upstream.json",
                compiler_tester::Toolchain::SolcLLVM => "./configs/solc-bin-llvm.json",
            })
        }),
        arguments
            .vyper_bin_config_path
            .unwrap_or_else(|| PathBuf::from("./configs/vyper-bin-default.json")),
    ];
    let environment = match (arguments.target, arguments.environment) {
        (
            era_compiler_common::Target::EraVM,
            Some(environment @ compiler_tester::Environment::ZkEVM),
        ) => environment,
        (era_compiler_common::Target::EraVM, Some(compiler_tester::Environment::FastVM)) => {
            todo!("FastVM is implemented as a crate feature")
        }
        (era_compiler_common::Target::EraVM, None) => compiler_tester::Environment::ZkEVM,
        (
            era_compiler_common::Target::EVM,
            Some(environment @ compiler_tester::Environment::EVMInterpreter),
        ) => environment,
        (
            era_compiler_common::Target::EVM,
            Some(environment @ compiler_tester::Environment::REVM),
        ) => environment,
        (era_compiler_common::Target::EVM, None) => compiler_tester::Environment::EVMInterpreter,
        (target, Some(environment)) => anyhow::bail!(
            "Target `{target}` and environment `{environment}` combination is not supported"
        ),
    };

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
                arguments.system_contracts_load_path,
                arguments.system_contracts_save_path,
                arguments.target,
            )?;

            match (
                arguments.disable_deployer,
                arguments.disable_value_simulator,
            ) {
                (true, true) => {
                    compiler_tester.run_eravm::<compiler_tester::EraVMNativeDeployer, false>(vm, toolchain)
                }
                (true, false) => {
                    compiler_tester.run_eravm::<compiler_tester::EraVMNativeDeployer, true>(vm, toolchain)
                }
                (false, true) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMSystemContractDeployer, false>(vm, toolchain),
                (false, false) => compiler_tester
                    .run_eravm::<compiler_tester::EraVMSystemContractDeployer, true>(vm, toolchain),
            }
        }
        compiler_tester::Environment::FastVM => todo!(),
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
                arguments.system_contracts_load_path,
                arguments.system_contracts_save_path,
                arguments.target,
            )?;

            compiler_tester
                .run_evm_interpreter::<compiler_tester::EraVMSystemContractDeployer, true>(
                    vm, toolchain,
                )
        }
        compiler_tester::Environment::REVM => {
            compiler_tester::EVM::download(executable_download_config_paths)?;
            compiler_tester.run_revm(toolchain)
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
    use std::path::PathBuf;

    use crate::arguments::Arguments;

    #[test]
    fn test_manually() {
        std::env::set_current_dir("..").expect("Change directory failed");

        let arguments = Arguments {
            verbosity: false,
            quiet: false,
            debug: false,
            modes: vec!["Y+M3B3 0.8.26".to_owned()],
            paths: vec!["tests/solidity/simple/default.sol".to_owned()],
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
            toolchain: Some(compiler_tester::Toolchain::IrLLVM),
            target: era_compiler_common::Target::EraVM,
            environment: None,
            workflow: compiler_tester::Workflow::BuildAndRun,
            solc_bin_config_path: Some(PathBuf::from("./configs/solc-bin-default.json")),
            vyper_bin_config_path: Some(PathBuf::from("./configs/vyper-bin-default.json")),
            system_contracts_load_path: Some(PathBuf::from("system-contracts-stable-build")),
            system_contracts_save_path: None,
            llvm_verify_each: false,
            llvm_debug_logging: false,
        };

        crate::main_inner(arguments).expect("Manual testing failed");
    }
}
