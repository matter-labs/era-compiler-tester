//!
//! The compiler tester arguments.
//!

use std::path::PathBuf;

use structopt::StructOpt;

use compiler_tester::Workflow;

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

    /// Path to the `zksolc` binary.
    /// Is set to `zksolc` by default.
    #[structopt(long = "zksolc")]
    pub zksolc: Option<PathBuf>,

    /// Path to the `zkvyper` binary.
    /// Is set to `zkvyper` by default.
    #[structopt(long = "zkvyper")]
    pub zkvyper: Option<PathBuf>,

    /// Specify the target machine.
    /// Available arguments: `EraVM`, `EVM`, `EVMInterpreter`.
    /// The default is `EraVM`.
    #[structopt(long = "target")]
    pub target: Option<String>,

    /// Use the upstream `solc` compiler.
    #[structopt(long = "use-upstream-solc")]
    pub use_upstream_solc: bool,

    /// Path to the default `solc` binaries download configuration file.
    #[structopt(long = "solc-bin-config-path")]
    pub solc_bin_config_path: Option<PathBuf>,

    /// Path to the default `vyper` binaries download configuration file.
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

    /// Choose between `build` to compile tests only without running them, and `run` to compile and run them.
    #[structopt(long = "workflow", default_value = "run")]
    pub workflow: Workflow,
}

impl Arguments {
    ///
    /// A shortcut constructor.
    ///
    pub fn new() -> Self {
        Self::from_args()
    }
}
