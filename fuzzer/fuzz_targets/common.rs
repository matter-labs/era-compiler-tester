use std::{path::PathBuf, sync::Arc};

use compiler_tester::{
    Buildable, EthereumTest, Mode, SolidityCompiler, SolidityMode, Summary, Workflow,
};
use era_compiler_solidity::SolcPipeline;

pub use solidity_adapter::{
    test::function_call::parser::{
        lexical::token::{
            lexeme::literal::boolean::Boolean as LexicalBooleanLiteral,
            lexeme::literal::integer::Integer, location::Location,
        },
        syntax::tree::{
            call::builder::Builder as CallBuilder,
            identifier::Identifier,
            literal::{
                alignment::Alignment, boolean::Literal as BooleanLiteral,
                integer::Literal as IntegerLiteral, Literal,
            },
            r#type::{variant::Variant as TypeVariant, Type},
        },
    },
    EnabledTest, FunctionCall,
};

///
/// Fuzzing case definition
///
pub struct FuzzingCase {
    // The path to the contract to fuzz
    pub contract_path: String,
    // The smart contract function name to fuzz
    pub function_name: String,
    // The function inputs types
    pub input_types: Vec<TypeVariant>,
    // The inputs values
    pub inputs: Vec<Literal>,
    // The expected output
    pub expected_output: Literal,
}

/// Create an integer literal primitive from input data
///
/// # Arguments
///
/// * `data` - The input data
///
/// # Returns
///
/// * `Literal` - The integer literal
///
pub fn integer_literal<T: ToString>(data: T) -> Literal {
    Literal::Integer(IntegerLiteral::new(
        Location::new(),
        Integer::new_decimal(data.to_string(), false),
        Alignment::default(),
    ))
}

/// Create a boolean literal primitive from input data
///
/// # Arguments
///
/// * `data` - boolean input
///
/// # Returns
///
/// * `Literal` - The boolean literal
///
pub fn boolean_literal(data: bool) -> Literal {
    Literal::Boolean(BooleanLiteral::new(
        Location::new(),
        if data {
            LexicalBooleanLiteral::True
        } else {
            LexicalBooleanLiteral::False
        },
        Alignment::default(),
    ))
}

/// Build function call from fuzzing data
///
/// # Arguments
///
/// * `case` - The fuzzing case
///
/// # Returns
///
/// * `FunctionCall` - The function call
///
pub fn build_function_call(case: FuzzingCase) -> anyhow::Result<FunctionCall> {
    // Initialize function call builder
    let mut builder: CallBuilder = CallBuilder::default();
    builder.set_location(Location::new());

    // Set function name
    builder.set_call(Identifier::new(Location::new(), case.function_name.clone()));
    // Set input parameter type
    for ftype in case.input_types.iter() {
        builder.push_types(Type::new(Location::new(), ftype.clone()));
    }

    // Set input parameter value
    for input in case.inputs.iter() {
        builder.push_input(input.clone());
    }

    // Set expected output
    builder.push_expected(case.expected_output.clone());

    // Finalize function call
    let call = builder.finish();
    FunctionCall::try_from(call)
}

/// Generates fuzzing test
///
/// # Arguments
///
/// * `test_path` - The path to the test contract
/// * `data` - The fuzzing data
///
/// # Returns
///
/// * `EthereumTest` - The Ethereum test
pub fn gen_fuzzing_test(case: FuzzingCase) -> anyhow::Result<EthereumTest> {
    let test_path = PathBuf::from(case.contract_path.as_str());

    // Generate Test objects for the fuzzing contract
    let enabled_test = EnabledTest::new(test_path.to_path_buf(), None, None, None);
    let mut test = solidity_adapter::Test::try_from(test_path.as_path())?;
    let fcall = build_function_call(case)?;
    test.calls.push(fcall);
    Ok(EthereumTest {
        identifier: test_path.to_string_lossy().to_string(),
        index_entity: enabled_test,
        test,
    })
}

/// Build and run the test
///
/// # Arguments
///
/// * `test` - The Ethereum test
///
/// # Returns
///
/// * `Summary` - The test summary
pub fn build_and_run(test: EthereumTest) -> anyhow::Result<Summary> {
    // TODO: this should be parametrized
    let solc_version = semver::Version::new(0, 8, 26);
    let mode = Mode::Solidity(SolidityMode::new(
        solc_version,
        SolcPipeline::Yul,
        true,
        true,
        era_compiler_llvm_context::OptimizerSettings::try_from_cli('3')
            .expect("Error: optimization settings incorrect!"),
        false,
        false,
    ));

    // Initialization
    era_compiler_llvm_context::initialize_target(era_compiler_llvm_context::Target::EraVM);
    era_compiler_solidity::EXECUTABLE
        .set(PathBuf::from(
            era_compiler_solidity::DEFAULT_EXECUTABLE_NAME,
        ))
        .expect("Always valid");
    compiler_tester::LLVMOptions::initialize(false, false)?;
    let compiler_tester = compiler_tester::CompilerTester::new(
        compiler_tester::Summary::new(true, false).wrap(),
        compiler_tester::Filters::new(vec![], vec![], vec![]),
        None,
        Workflow::BuildAndRun,
    )?;

    // Compile and run test
    if let Some(test) = test.build_for_eravm(
        mode,
        Arc::new(SolidityCompiler::new()),
        compiler_tester::Target::EraVM,
        compiler_tester.summary.clone(),
        &compiler_tester.filters,
        compiler_tester.debug_config.clone(),
    ) {
        test.run_eravm::<compiler_tester::EraVMSystemContractDeployer, true>(
            compiler_tester.summary.clone(),
            Arc::new(compiler_tester::EraVM::new(
                vec![
                    PathBuf::from("./configs/solc-bin-default.json"),
                    PathBuf::from("./configs/vyper-bin-default.json"),
                ],
                PathBuf::from("./configs/solc-bin-system-contracts.json"),
                None,
                Some(PathBuf::from("system-contracts-stable-build")),
                Some(PathBuf::from("system-contracts-stable-build")),
            )?),
        );
    }

    // Get the results
    let summary = compiler_tester::Summary::unwrap_arc(compiler_tester.summary);
    Ok(summary)
}
