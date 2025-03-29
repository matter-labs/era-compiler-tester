//!
//! The compiler tester arguments.
//!

use std::path::PathBuf;

use clap::Parser;

///
/// The compiler tester arguments.
///
#[derive(Debug, Parser)]
#[command(about, long_about = None)]
pub struct Arguments {
    /// The logging level.
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppresses the output completely.
    #[arg(short, long)]
    pub quiet: bool,

    /// Saves all IRs produced by compilers to `./debug/` directory.
    #[arg(short = 'D', long)]
    pub debug: bool,

    /// Runs tests only in modes that contain any string from the specified ones.
    #[arg(short, long)]
    pub mode: Vec<String>,

    /// Runs only tests whose name contains any string from the specified ones.
    #[arg(short, long)]
    pub path: Vec<String>,

    /// Runs only tests from the specified groups.
    #[structopt(short, long)]
    pub group: Vec<String>,

    /// The benchmark output path, if requested.
    #[structopt(short, long)]
    pub benchmark: Option<PathBuf>,

    /// The benchmark output format: `json`, `csv`, or `json-lnt`.
    /// Using `json-lnt` requires providing the path to a JSON file describing the
    /// benchmarking context via `--benchmark-context`.
    #[structopt(long = "benchmark-format", default_value_t = compiler_tester::BenchmarkFormat::Json)]
    pub benchmark_format: compiler_tester::BenchmarkFormat,

    /// The benchmark context to pass additional data to backends.
    #[structopt(long = compiler_tester::ARGUMENT_BENCHMARK_CONTEXT)]
    pub benchmark_context: Option<PathBuf>,

    /// Sets the number of threads, which execute the tests concurrently.
    #[structopt(short, long)]
    pub threads: Option<usize>,

    /// Whether to dump the debug data for system contracts.
    #[structopt(long)]
    pub dump_system: bool,

    /// Whether the deployer should be disabled.
    #[structopt(long)]
    pub disable_deployer: bool,

    /// Whether the msg.value simulator should be disabled.
    #[structopt(long)]
    pub disable_value_simulator: bool,

    /// Path to the `zksolc` executable.
    /// Is set to `zksolc` by default.
    #[structopt(long)]
    pub zksolc: Option<PathBuf>,

    /// Path to the `zkvyper` executable.
    /// Is set to `zkvyper` by default.
    #[structopt(long)]
    pub zkvyper: Option<PathBuf>,

    /// Specify the compiler toolchain.
    /// Available arguments: `ir-llvm`, `solc`, `solc-llvm`.
    /// The default for `EraVM` target is `ir-llvm`.
    /// The default for `EVM` target is `solc`.
    #[structopt(long)]
    pub toolchain: Option<compiler_tester::Toolchain>,

    /// Specify the target architecture.
    /// Available arguments: `eravm`, `evm`.
    #[structopt(long)]
    pub target: era_compiler_common::Target,

    /// Specify the environment to run tests on.
    /// Available arguments: `zk_evm`, `FastVM`, `EVMInterpreter`, `REVM`.
    /// The default for `EraVM` target is `zk_evm`.
    /// The default for `EVM` target is `EVMInterpreter`.
    #[structopt(long)]
    pub environment: Option<compiler_tester::Environment>,

    /// Choose between `build` to compile tests only without running, and `run` to compile and run.
    #[structopt(long, default_value_t = compiler_tester::Workflow::BuildAndRun)]
    pub workflow: compiler_tester::Workflow,

    /// Path to the default `solc` executables download configuration file.
    #[structopt(long)]
    pub solc_bin_config_path: Option<PathBuf>,

    /// Path to the default `vyper` executables download configuration file.
    #[structopt(long)]
    pub vyper_bin_config_path: Option<PathBuf>,

    /// Whether to load the system contracts builds from the specified file.
    #[structopt(long)]
    pub load_system_contracts: Option<PathBuf>,

    /// Whether to save the system contracts builds to the specified file.
    #[structopt(long)]
    pub save_system_contracts: Option<PathBuf>,

    /// Sets the `verify each` option in LLVM.
    #[structopt(long)]
    pub llvm_verify_each: bool,

    /// Sets the `debug logging` option in LLVM.
    #[structopt(long)]
    pub llvm_debug_logging: bool,
}

impl Arguments {
    ///
    /// Validate the arguments passed from user, checking invariants that are not
    /// expressed in the type system.
    ///
    pub fn validate(arguments: Self) -> anyhow::Result<Self> {
        match (&arguments.benchmark_format, &arguments.benchmark_context) {
            (BenchmarkFormat::JsonLNT, None) =>
                anyhow::bail!("Generation of LNT-compatible benchmark results in JSON format requires passing a valid context in the argument `--{ARGUMENT_BENCHMARK_CONTEXT}` to compiler tester.")
            ,
            (BenchmarkFormat::JsonLNT, Some(_)) => (),
            (_, Some(_)) =>
                anyhow::bail!("Only LNT backend in JSON format supports passing a valid context in the argument `--{ARGUMENT_BENCHMARK_CONTEXT}` to compiler tester.")
            ,
            _ => (),
        }

        Ok(arguments)
    }
}
