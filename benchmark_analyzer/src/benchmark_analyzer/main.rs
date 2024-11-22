//!
//! The benchmark analyzer binary.
//!

pub(crate) mod arguments;

use std::io::Write;

use clap::Parser;

use self::arguments::Arguments;

///
/// The application entry point.
///
fn main() -> anyhow::Result<()> {
    let arguments = Arguments::try_parse()?;

    let reference = benchmark_analyzer::Benchmark::try_from(arguments.reference)?;
    let candidate = benchmark_analyzer::Benchmark::try_from(arguments.candidate)?;

    let groups_results = benchmark_analyzer::Benchmark::compare(&reference, &candidate);

    match arguments.output_file {
        Some(output_path) => {
            let mut file = std::fs::File::create(output_path)?;
            for (group_name, mut results) in groups_results.into_iter() {
                results.sort_worst();
                results.print_worst_results(arguments.group_max, group_name);
                results.write_all(&mut file, group_name)?;
                writeln!(file)?;
                println!();
                println!();
            }
        }
        None => {
            let mut stdout = std::io::stdout();
            for (group_name, mut results) in groups_results.into_iter() {
                results.sort_worst();
                results.print_worst_results(arguments.group_max, group_name);
                results.write_all(&mut stdout, group_name)?;
                writeln!(stdout)?;
                println!();
                println!();
            }
        }
    }

    Ok(())
}
