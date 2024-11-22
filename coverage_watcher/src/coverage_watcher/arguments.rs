//!
//! The coverage watcher arguments.
//!

use std::path::PathBuf;

use clap::Parser;

///
/// The coverage watcher arguments.
///
#[derive(Debug, Parser)]
#[command(about, long_about = None)]
pub struct Arguments {
    /// The missed tests output file path.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}
