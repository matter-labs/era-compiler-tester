# Solidity Contracts Fuzzing

This is the skeleton for Solidity smart contracts fuzzing based on the [Rust fuzz](https://rust-fuzz.github.io/book/introduction.html) engine.

## Project structure

The project consists of the following directories:

- `fuzz_contracts` - Solidity smart contracts to be fuzzed.
- `fuzz_targets` - fuzzing targets definitions.

### Fuzzing targets

Each fuzzing target is a separate Rust binary crate and defined in the `fuzz_targets` directory. The `Cargo.toml` file in the fuzzer directory contains the dependencies and the configuration for the fuzzing engine.

For example, the `demo` fuzzing target is defined in the `fuzz_targets/demo.rs` file. The `Cargo.toml` file contains the following section:

```properties
[[bin]]
name = "demo"
path = "fuzz_targets/demo.rs"
...
```

`cargo fuzz add <target_name>` command can be used to add a new empty fuzzing target.

## Running fuzzing

### Prerequisites

1. Follow the build instructions from [README.md](../README.md) to build `era-compiler-tester`.
2. Install the `cargo-fuzz` tool:
```bash
cargo install cargo-fuzz
```
3. Build `zksolc` compiler and make sure that `zksolc` is added in the `PATH`.
4. Install nightly Rust toolchain:
```bash
rustup toolchain install nightly
```

### Executing fuzzing

To execute a fuzzing target, run the following command from the root directory of the project:

```bash
cargo +nightly fuzz run --fuzz-dir fuzzer <target_name>
```

## Supported targets

- [`demo`](./fuzz_contracts/demo/demo.md) - demonstrates the basic fuzzing setup.
- `optimizer_bug` - demonstrates fuzzer finding a bug in the optimizer.

## Current limitations

- The current setup uses the fixed hardcoded version of optimization settings (`Y+M3B3`) and `solc` compiler version (`0.8.24`).
- The current targets are using the simplest contracts and fuzzing strategy that mutates only the function arguments.
- The current setup uses only EraVM as the execution engine as well as `EthereumTest` as the test type.

## Roadmap

- [ ] Add the ability to specify the optimization settings and compiler versions.
- [ ] Support for more complex contracts (real-life use cases).
- [ ] Support on-the-fly fuzzing function generation.
- [ ] Support mutating of the contract source code with Solidity vocabulary.
- [ ] Support CI execution in OSS Fuzz infrastructure.
