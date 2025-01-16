//!
//! The benchmark analyzer binary.
//!

pub(crate) mod arguments;

use std::io::Write;

use clap::Parser;

use self::arguments::Arguments;
use benchmark_analyzer::ResultsGroup;

///
/// The application entry point.
///
fn main() -> anyhow::Result<()> {
    let arguments = Arguments::try_parse()?;

    let reference_benchmark = benchmark_analyzer::Benchmark::try_from(arguments.reference)?;
    let candidate_benchmark = benchmark_analyzer::Benchmark::try_from(arguments.candidate)?;

    let groups_results = if let (Some(reference_query), Some(candidate_query)) =
        (arguments.query_reference, arguments.query_candidate)
    {
        // If the user provides regular expressions to select groups for
        // comparison, the analyzer will compare all groups with the same names,
        // plus all pairs of groups matching regular expressions
        // [regex_reference] and [regex_candidate].

        let regex_reference =
            regex::Regex::new(&reference_query).expect("Invalid reference query regexp");
        let regex_candidate =
            regex::Regex::new(&candidate_query).expect("Invalid candidate query regexp");

        benchmark_analyzer::analysis::compare(
            &reference_benchmark,
            &candidate_benchmark,
            |g1: &ResultsGroup<'_>, g2: &ResultsGroup<'_>| {
                g1.regex_matches(&regex_reference) && g2.regex_matches(&regex_candidate)
            },
        )
    } else {
        // If the user did not provide regular expressions to select groups for
        // comparison, the analyzer will compare only the groups with the same
        // names.
        benchmark_analyzer::analysis::compare(
            &reference_benchmark,
            &candidate_benchmark,
            |_: &ResultsGroup<'_>, _: &ResultsGroup<'_>| false,
        )
    };

    match arguments.output_file {
        Some(output_path) => {
            let mut file = std::fs::File::create(output_path)?;
            for (group_name, mut results) in groups_results.into_iter() {
                results.sort_worst();
                results.print_worst_results(arguments.group_max, &group_name.to_string());
                results.write_all(&mut file, &group_name.to_string())?;
                writeln!(file)?;
                println!();
                println!();
            }
        }
        None => {
            let mut stdout = std::io::stdout();
            for (group_name, mut results) in groups_results.into_iter() {
                results.sort_worst();
                results.print_worst_results(arguments.group_max, &group_name.to_string());
                results.write_all(&mut stdout, &group_name.to_string())?;
                writeln!(stdout)?;
                println!();
                println!();
            }
        }
    }

    Ok(())
}
