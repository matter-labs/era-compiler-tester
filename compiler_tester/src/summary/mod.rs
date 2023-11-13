//!
//! The compiler tester summary.
//!

pub mod element;

use std::sync::Arc;
use std::sync::Mutex;

use colored::Colorize;

use crate::compilers::mode::Mode;
use crate::test::case::input::output::Output;

use self::element::outcome::passed_variant::PassedVariant;
use self::element::outcome::Outcome;
use self::element::Element;

///
/// The compiler tester summary.
///
#[derive(Debug)]
pub struct Summary {
    /// The summary elements.
    elements: Vec<Element>,
    /// The output verbosity.
    verbosity: bool,
    /// Whether the output is suppressed.
    quiet: bool,
    /// The passed tests counter.
    passed: usize,
    /// The failed tests counter.
    failed: usize,
    /// The invalid tests counter.
    invalid: usize,
    /// The ignored tests counter.
    ignored: usize,
}

impl Summary {
    /// The elements vector default capacity.
    pub const ELEMENTS_INITIAL_CAPACITY: usize = 65536;

    ///
    /// A shortcut constructor.
    ///
    pub fn new(verbosity: bool, quiet: bool) -> Self {
        Self {
            elements: Vec::with_capacity(Self::ELEMENTS_INITIAL_CAPACITY),
            verbosity,
            quiet,
            passed: 0,
            failed: 0,
            invalid: 0,
            ignored: 0,
        }
    }

    ///
    /// Whether the test run has been successful.
    ///
    pub fn is_successful(&self) -> bool {
        for element in self.elements.iter() {
            match element.outcome {
                Outcome::Passed { .. } => continue,
                Outcome::Failed { .. } => return false,
                Outcome::Invalid { .. } => return false,
                Outcome::Ignored => continue,
            }
        }

        true
    }

    ///
    /// Returns the benchmark structure.
    ///
    pub fn benchmark(&self) -> benchmark_analyzer::Benchmark {
        let mut benchmark = benchmark_analyzer::Benchmark::default();
        benchmark.groups.insert(
            format!(
                "{} {}",
                benchmark_analyzer::BENCHMARK_ALL_GROUP_NAME,
                compiler_llvm_context::OptimizerSettings::cycles(),
            ),
            benchmark_analyzer::BenchmarkGroup::default(),
        );
        benchmark.groups.insert(
            format!(
                "{} {}",
                benchmark_analyzer::BENCHMARK_ALL_GROUP_NAME,
                compiler_llvm_context::OptimizerSettings::size(),
            ),
            benchmark_analyzer::BenchmarkGroup::default(),
        );

        for element in self.elements.iter() {
            let (size, cycles, ergs, group) = match &element.outcome {
                Outcome::Passed {
                    variant: PassedVariant::Deploy { size, cycles, ergs },
                    group,
                } => (Some(*size), *cycles, *ergs, group.clone()),
                Outcome::Passed {
                    variant: PassedVariant::Runtime { cycles, ergs },
                    group,
                } => (None, *cycles, *ergs, group.clone()),
                _ => continue,
            };

            let key = format!(
                "{:24} {}",
                element
                    .mode
                    .as_ref()
                    .map(|mode| mode.to_string())
                    .unwrap_or_default(),
                element.name
            );
            let mode = element
                .mode
                .as_ref()
                .and_then(|mode| mode.llvm_optimizer_settings().cloned())
                .unwrap_or(compiler_llvm_context::OptimizerSettings::none());

            let benchmark_element = benchmark_analyzer::BenchmarkElement::new(size, cycles, ergs);
            if let Some(group) = group {
                benchmark
                    .groups
                    .entry(format!("{} {}", group, mode))
                    .or_default()
                    .elements
                    .insert(key.clone(), benchmark_element.clone());
            }

            benchmark
                .groups
                .get_mut(
                    format!("{} {}", benchmark_analyzer::BENCHMARK_ALL_GROUP_NAME, mode).as_str(),
                )
                .expect("Always exists")
                .elements
                .insert(key, benchmark_element);
        }
        benchmark
    }

    ///
    /// Wraps data into a synchronized shared reference.
    ///
    pub fn wrap(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }

    ///
    /// Extracts the data from the synchronized shared reference.
    ///
    pub fn unwrap_arc(summary: Arc<Mutex<Self>>) -> Self {
        Arc::try_unwrap(summary)
            .expect("Last shared reference")
            .into_inner()
            .expect("Last shared reference")
    }

    ///
    /// Adds an invalid outcome.
    ///
    pub fn invalid<S>(summary: Arc<Mutex<Self>>, mode: Option<Mode>, name: String, error: S)
    where
        S: ToString,
    {
        let element = Element::new(mode, name, Outcome::invalid(error));
        summary.lock().expect("Sync").push_element(element);
    }

    ///
    /// Adds a failed outcome.
    ///
    pub fn failed(
        summary: Arc<Mutex<Self>>,
        mode: Mode,
        name: String,
        expected: Output,
        found: Output,
        calldata: Vec<u8>,
    ) {
        let element = Element::new(Some(mode), name, Outcome::failed(expected, found, calldata));
        summary.lock().expect("Sync").push_element(element);
    }

    ///
    /// Adds an ignored outcome.
    ///
    pub fn ignored(summary: Arc<Mutex<Self>>, name: String) {
        let element = Element::new(None, name, Outcome::ignored());
        summary.lock().expect("Sync").push_element(element);
    }

    ///
    /// Adds a passed contract deploy outcome.
    ///
    pub fn passed_deploy(
        summary: Arc<Mutex<Self>>,
        mode: Mode,
        name: String,
        group: Option<String>,
        size: usize,
        cycles: usize,
        ergs: u32,
    ) {
        let passed_variant = PassedVariant::Deploy { size, cycles, ergs };
        Self::passed(summary, mode, name, group, passed_variant);
    }

    ///
    /// Adds a passed contract call outcome.
    ///
    pub fn passed_runtime(
        summary: Arc<Mutex<Self>>,
        mode: Mode,
        name: String,
        group: Option<String>,
        cycles: usize,
        ergs: u32,
    ) {
        let passed_variant = PassedVariant::Runtime { cycles, ergs };
        Self::passed(summary, mode, name, group, passed_variant);
    }

    ///
    /// Adds a passed special function call outcome.
    ///
    pub fn passed_special(
        summary: Arc<Mutex<Self>>,
        mode: Mode,
        name: String,
        group: Option<String>,
    ) {
        let passed_variant = PassedVariant::Special;
        Self::passed(summary, mode, name, group, passed_variant);
    }

    ///
    /// Adds a passed outcome.
    ///
    fn passed(
        summary: Arc<Mutex<Self>>,
        mode: Mode,
        name: String,
        group: Option<String>,
        passed_variant: PassedVariant,
    ) {
        let element = Element::new(Some(mode), name, Outcome::passed(group, passed_variant));
        summary.lock().expect("Sync").push_element(element);
    }

    ///
    /// Pushes an element to the summary, printing it.
    ///
    fn push_element(&mut self, element: Element) {
        if let Some(string) = element.print(self.verbosity) {
            println!("{string}");
        }

        let is_executed = match element.outcome {
            Outcome::Passed { .. } => {
                self.passed += 1;
                true
            }
            Outcome::Failed { .. } => {
                self.failed += 1;
                true
            }
            Outcome::Invalid { .. } => {
                self.invalid += 1;
                true
            }
            Outcome::Ignored => {
                self.ignored += 1;
                false
            }
        };

        if is_executed {
            let milestone = if self.verbosity {
                usize::pow(10, 3)
            } else {
                usize::pow(10, 5)
            };

            if (self.passed + self.failed + self.invalid) % milestone == 0 {
                println!("{self}");
            }
        }

        self.elements.push(element);
    }
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.quiet {
            return Ok(());
        }

        writeln!(
            f,
            "╔═══════════════════╡ INTEGRATION TESTING ╞════════════════════╗"
        )?;
        writeln!(
            f,
            "║                                                              ║"
        )?;
        writeln!(
            f,
            "║     {:7}                                   {:10}     ║",
            "PASSED".green(),
            self.passed.to_string().green(),
        )?;
        writeln!(
            f,
            "║     {:7}                                   {:10}     ║",
            "FAILED".bright_red(),
            self.failed.to_string().bright_red(),
        )?;
        writeln!(
            f,
            "║     {:7}                                   {:10}     ║",
            "INVALID".red(),
            self.invalid.to_string().red(),
        )?;
        writeln!(
            f,
            "║     {:7}                                   {:10}     ║",
            "IGNORED".bright_black(),
            self.ignored.to_string().bright_black(),
        )?;
        writeln!(
            f,
            "║               {:10} TESTS MILESTONE                     ║",
            self.passed + self.failed + self.invalid,
        )?;
        writeln!(
            f,
            "╚══════════════════════════════════════════════════════════════╝"
        )?;

        Ok(())
    }
}
