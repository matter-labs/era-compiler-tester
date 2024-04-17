# Solidity Contracts Fuzzing

This is the skeleton for Solidity smart contracts fuzzing based on the [Rust fuzz](https://rust-fuzz.github.io/book/introduction.html) engine.

## Project structure

The project consists of the following directories:

- `fuzz_contracts` - Solidity smart contracts to be fuzzed.
- `fuzz_targets` - fuzzing targets definitions.

### Fuzzing targets

Each fuzzing target is a separate Rust binary crate and defined in the `fuzz_targets` directory. The `Cargo.toml` file in the root directory contains the dependencies and the configuration for the fuzzing engine.

For example, the `simple` fuzzing target is defined in the `fuzz_targets/simple.rs` file. The `Cargo.toml` file contains the following section:

```properties
[[bin]]
name = "simple"
path = "fuzz_targets/simple.rs"
...
```

`cargo fuzz add <target_name>` command can be used to add a new empty fuzzing target.

## Running fuzzing

To run the fuzzing, execute the following command:

```bash
cargo fuzz run <target_name>
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
