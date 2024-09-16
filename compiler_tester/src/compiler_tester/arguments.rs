//!
//! The compiler tester arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

///
/// The compiler tester arguments.
///
#[derive(Debug, StructOpt)]
#[structopt(
    name = "compiler-tester",
    about = "EraVM Compiler Integration Testing Framework"
)]
pub struct Arguments {
    /// The logging level.
    #[structopt(short = "v", long = "verbose")]
    pub verbosity: bool,

    /// Suppresses the output completely.
    #[structopt(short = "q", long = "quiet")]
    pub quiet: bool,

    /// Saves all IRs produced by compilers to `./debug/` directory.
    #[structopt(short = "D", long = "debug")]
    pub debug: bool,

    /// Runs tests only in modes that contain any string from the specified ones.
    #[structopt(short = "m", long = "mode")]
    pub modes: Vec<String>,

    /// Runs only tests whose name contains any string from the specified ones.
    #[structopt(short = "p", long = "path")]
    pub paths: Vec<String>,

    /// Runs only tests from the specified groups.
    #[structopt(short = "g", long = "group")]
    pub groups: Vec<String>,

    /// The benchmark output path, if requested.
    #[structopt(short = "b", long = "benchmark")]
    pub benchmark: Option<PathBuf>,

    /// Sets the number of threads, which execute the tests concurrently.
    #[structopt(short = "t", long = "threads")]
    pub threads: Option<usize>,

    /// Whether to dump the debug data for system contracts.
    #[structopt(long = "dump-system")]
    pub dump_system: bool,

    /// Whether the deployer should be disabled.
    #[structopt(long = "disable-deployer")]
    pub disable_deployer: bool,

    /// Whether the msg.value simulator should be disabled.
    #[structopt(long = "disable-value-simulator")]
    pub disable_value_simulator: bool,

    /// Path to the `zksolc` executable.
    /// Is set to `zksolc` by default.
    #[structopt(long = "zksolc")]
    pub zksolc: Option<PathBuf>,

    /// Path to the `zkvyper` executable.
    /// Is set to `zkvyper` by default.
    #[structopt(long = "zkvyper")]
    pub zkvyper: Option<PathBuf>,

    /// Specify the compiler toolchain.
    /// Available arguments: `ir-llvm`, `solc`, `solc-llvm`.
    /// The default for `EraVM` target is `ir-llvm`.
    /// The default for `EVM` target is `solc`.
    #[structopt(long = "toolchain")]
    pub toolchain: Option<compiler_tester::Toolchain>,

    /// Specify the target architecture.
    /// Available arguments: `eravm`, `evm`.
    /// The default is `eravm`.
    #[structopt(long = "target", default_value = "eravm")]
    pub target: era_compiler_common::Target,

    /// Specify the environment to run tests on.
    /// Available arguments: `zk_evm`, `FastVM`, `EVMInterpreter`, `REVM`.
    /// The default for `EraVM` target is `zk_evm`.
    /// The default for `EVM` target is `EVMInterpreter`.
    #[structopt(long = "environment")]
    pub environment: Option<compiler_tester::Environment>,

    /// Choose between `build` to compile tests only without running, and `run` to compile and run.
    #[structopt(long = "workflow", default_value = "run")]
    pub workflow: compiler_tester::Workflow,

    /// Path to the default `solc` executables download configuration file.
    #[structopt(long = "solc-bin-config-path")]
    pub solc_bin_config_path: Option<PathBuf>,

    /// Path to the default `vyper` executables download configuration file.
    #[structopt(long = "vyper-bin-config-path")]
    pub vyper_bin_config_path: Option<PathBuf>,

    /// Whether to load the system contracts builds from the specified file.
    #[structopt(long = "load-system-contracts")]
    pub system_contracts_load_path: Option<PathBuf>,

    /// Whether to save the system contracts builds to the specified file.
    #[structopt(long = "save-system-contracts")]
    pub system_contracts_save_path: Option<PathBuf>,

    /// Sets the `verify each` option in LLVM.
    #[structopt(long = "llvm-verify-each")]
    pub llvm_verify_each: bool,

    /// Sets the `debug logging` option in LLVM.
    #[structopt(long = "llvm-debug-logging")]
    pub llvm_debug_logging: bool,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
