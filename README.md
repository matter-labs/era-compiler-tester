# Integration Test Framework for LLVM-based Compilers

[![Logo](eraLogo.svg)](https://zksync.io/)

ZKsync Era is a layer 2 rollup that uses zero-knowledge proofs to scale Ethereum without compromising on security
or decentralization. As it's EVM-compatible (with Solidity/Vyper), 99% of Ethereum projects can redeploy without
needing to refactor or re-audit any code. ZKsync Era also uses an LLVM-based compiler that will eventually enable
developers to write smart contracts in popular languages such as C++ and Rust.

The `era-compiler-tester` integration test framework runs tests for Matter Labs compilers which target the EraVM,
for supported languages listed below. It compiles source code via external API calls,
e.g. to [Inkwell](https://thedan64.github.io/inkwell/inkwell/index.html). In software quality assurance jargon,
this makes it a whitebox testing framework.

The `era-compiler-tester` repository includes the Compiler Tests Collection repository as a submodule.

By default, the Tester SHOULD run the entire Collection in all possible combinations of compiler versions and settings,
but it MAY omit some subset of the combinations for the sake of saving time, e.g. when only front-end changes have been
made, and there is no point in running tests in all LLVM optimization modes.



## Building

<details>
<summary>1. Install the system prerequisites.</summary>

   * Linux (Debian):

      Install the following packages:
      ```shell
      apt install cmake ninja-build curl git libssl-dev pkg-config clang lld
      ```
   * Linux (Arch):

      Install the following packages:
      ```shell
      pacman -Syu which cmake ninja curl git pkg-config clang lld
      ```

   * MacOS:

      * Install the [HomeBrew](https://brew.sh) package manager.
      * Install the following packages:

         ```shell
         brew install cmake ninja coreutils
         ```

      * Install your choice of a recent LLVM/[Clang](https://clang.llvm.org) compiler, e.g. via [Xcode](https://developer.apple.com/xcode/), [Apple’s Command Line Tools](https://developer.apple.com/library/archive/technotes/tn2339/_index.html), or your preferred package manager.
</details>

<details>
<summary>2. Install Rust.</summary>

   * Follow the latest [official instructions](https://www.rust-lang.org/tools/install:
      ```shell
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
      . ${HOME}/.cargo/env
      ```

      > Currently we are not pinned to any specific version of Rust, so just install the latest stable build for your   platform.
</details>

<details>
<summary>3. Checkout or clone the repository.</summary>

   * If you have not cloned this repository yet:
      ```shell
      git clone https://github.com/matter-labs/era-compiler-tester.git --recursive
      ```

   * If you have already cloned this repository:
      ```shell
      git submodule update --init --recursive --remote
      ```

</details>

<details>
<summary>4. Build ZKsync LLVM framework.</summary>

   * Install the builder using `cargo`:
      ```shell
      cargo install compiler-llvm-builder
      ```

      > The builder is not the ZKsync LLVM framework itself, but a tool that clones its repository and runs a sequence of build commands. By default it is installed in `~/.cargo/bin/`, which is recommended to be added to your `$PATH`.

   * Clone and build the ZKsync LLVM framework using the `zksync-llvm` tool:
      ```shell
      zksync-llvm clone
      zksync-llvm build
      ```

   * If you have already cloned the LLVM repository:
      ```shell
      zksync-llvm checkout
      zksync-llvm build
      ```

   * If you would like to use your local LLVM build:
      ```shell
      export LLVM_SYS_191_PREFIX='<ABSOLUTE_PATH_TO_YOUR_LOCAL_LLVM_BUILD>'
      ```

</details>

<details>
<summary>5. Build compiler executables.</summary>

   * Build [zksolc](https://github.com/matter-labs/era-compiler-solidity) and [zkvyper](https://github.com/matter-labs/era-compiler-vyper) compilers and add the binaries to `$PATH`, or use the `--zksolc` or `--zkvyper` options to specify their paths.

</details>

<details>
<summary>6. Build the main application.</summary>

   * Build era-compiler-tester with `cargo`:
      ```shell
      cargo build --release
      ```

</details>

When the build succeeds, you can run the tests using [the examples below](#usage).



## GitHub Actions

The `era-compiler-tester` is integrated into the GitHub Actions workflows of the following projects:

* [era-compiler-llvm](https://github.com/matter-labs/era-compiler-llvm)
* [era-solidity](https://github.com/matter-labs/era-solidity/)

To allow testing custom FE and VM changes in Pull Requests (PRs) of these repositories, two additional tags are supported:
* `era-compiler-llvm-test`
* `era-solidity-test`

If these tags exist, the tester from these tags will be used by the workflows instead of the default `main` branch.

When testing is done, these tags should be removed.



## What is supported

### Languages

- Solidity
- Yul
- Vyper
- LLVM IR
- EraVM assembly

### Optimizers

- LLVM middle-end optimizer (levels 0 to 3, s, z, e.g. `M0`, `Mz` etc.)
- LLVM back-end optimizer (levels 0 and 3, i.e. `B0` and `B3`)
- `solc` optimizer (`-` or `+`)
- `vyper` optimizer (`-` or `+`)

### Solidity codegens

- Yul pure (`Y`)
- EVM assembly from Yul (`I`)
- EVM assembly pure (`E`)
- Vyper LLL (`V`)

### Compiler versions

- `>=0.8` for compiling Solidity via Yul
- `>=0.8.13` for compiling Solidity via EVM assembly from Yul
- [0.4.10; latest] for compiling Solidity via EVM assembly
- [0.3.3, 0.3.9] for compiling Vyper via LLL IR

### Compiler codegens

Currently only relevant for the Solidity compiler, where you can choose the IR:

- Yul (preferred for Solidity ≥0.8)
- EVM (supports Solidity ≥0.4)

### Wildcards

Most of the specifiers support wildcards `*` (any), `^` ('3' and 'z').
With no mode argument, iterates over all option combinations (approximately 800).


## Usage

Each command assumes you are at the root of the `compiler-tester` repository.

### Generic command

```bash
cargo run --release --bin compiler-tester -- [-v] [-D] [-T[T]] \
	[--path="${PATH}"]* \
	[--mode="${MODE}"]*
```

There are more rarely used options, which you may check out with `./target/release/compiler-tester --help`.

### Example 1

Run a simple Solidity test, dumping Yul, unoptimized and optimized LLVM IR, and EraVM assembly to the specified directory.

Use:

- Yul as the Solidity IR (`Y`)
- Yul optimizations enabled (`+`)
- level 3 optimizations in LLVM middle-end (`M3`)
- level 3 optimizations in LLVM back-end (`B3`)
- Solidity compiler version (`0.8.26`)

Output:

- failed and invalid tests only (absence of `-v`)
- the compiler debug data to the `./debug/` directory (`-D`)
- the VM trace data to the `./trace/` directory (`-T`)

```bash
cargo run --release --bin compiler-tester -- -DT \
	--path='tests/solidity/simple/default.sol' \
	--mode='Y+M3B3 0.8.26' \
	--zksolc '../era-compiler-solidity/target/release/zksolc'
```

Modes are insensitive to spaces, therefore options such as `'Y+M3B3 0.8.26'` and `'Y +  M3B3     0.8.26'` are equivalent.

### Example 2

Run all simple Yul tests. This currently runs about three hundred tests and takes about eight minutes.

Use:

- level 1 optimizations in LLVM middle-end (`M1`)
- level 2 optimizations in LLVM back-end (`B2`)

Output:

- all tests, passed and failed (`-v`)
- the VM trace data to the `./trace/` directory (`-T`)

```bash
cargo run --release --bin compiler-tester -- -vT \
	--path='tests/yul/' \
	--mode='M1B2'
```

### Example 3

Run all tests (currently about three million) in all modes.
This takes a few hours on the CI server, and probably much longer on your personal machine.

```bash
cargo run --release --bin compiler-tester -- \
	--zksolc '../era-compiler-solidity/target/release/zksolc' \
	--zkvyper '../era-compiler-vyper/target/release/zkvyper'
```



## Benchmarking

1. Change the LLVM branch to the base in the `LLVM.lock` file at the repository root, checkout and build it:
```
zksync-llvm checkout && zksync-llvm build
```

2. Run the Tester with the desired filters and the output JSON path:
```
./target/release/compiler-tester \
	--path='tests/solidity/simple/default.sol' \
	--mode='Y+M^B3 0.8.26' \
	--benchmark='reference.json'
```

3. Change the LLVM branch to your patch in the `LLVM.lock` file at the repository root, checkout and build it:
```
zksync-llvm checkout && zksync-llvm build
```

4. Run the Tester with the desired filters and the output JSON path:
```
./target/release/compiler-tester \
	--path='tests/solidity/simple/default.sol' \
	--mode='Y+M^B3 0.8.26' \
	--benchmark='candidate.json'
```

5. Run the benchmark analyzer on the two JSONs:
```
cargo run --release --bin benchmark-analyzer -- --reference reference.json --candidate candidate.json
```

After you make any changes in LLVM, you only need to repeat steps 2-3 to update the working branch benchmark data.

### Comparing results 

By default, benchmark analyzer compares tests from groups with the same name, which means that every test should be compiled with the same codegen and optimizations. 
To compare two groups with different names, use the options `--query-reference` and `--query-candidate`. Then, use benchmark analyzer:

```shell
cargo run --release --bin benchmark-analyzer -- --reference reference.json --candidate candidate.json --query-reference "M0B0" --query-candidate "M3B3"
```

The queries are regular expressions, and the group name, codegen, and
optimization options are matched against it.



### Report formats

Use the parameter `--benchmark-format` of compiler tester to select the output format: `json` (default), `csv`, or `json-lnt`.

If `json-lnt` format is selected:

1. The benchmark report will consist of multiple files. They will be placed in the directory provided via the `--output` argument.
2. It is mandatory to pass a JSON file with additional information using `--benchmark-context`. Here is a minimal example:

```json
{
    "machine": "some_machine",
    "target": "some_target",
    "toolchain": "some_solc_type"
}
```


## Troubleshooting

- Unset any LLVM-related environment variables you may have set, especially `LLVM_SYS_<version>_PREFIX` (see e.g. [https://crates.io/crates/llvm-sys](https://crates.io/crates/llvm-sys) and [https://llvm.org/docs/GettingStarted.html#local-llvm-configuration](https://llvm.org/docs/GettingStarted.html#local-llvm-configuration)). To make sure: `set | grep LLVM`.



## License

The Compiler Tester is distributed under the terms of either

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.



## Resources

[ZKsync Era compiler toolchain documentation](https://docs.zksync.io/zk-stack/components/compiler/toolchain)



## Official Links

- [Website](https://zksync.io/)
- [GitHub](https://github.com/matter-labs)
- [Twitter](https://twitter.com/zksync)
- [Twitter for Devs](https://twitter.com/ZKsyncDevs)
- [Discord](https://join.zksync.dev/)



## Disclaimer

ZKsync Era has been through extensive testing and audits, and although it is live, it is still in alpha state and
will undergo further audits and bug bounty programs. We would love to hear our community's thoughts and suggestions
about it!
It's important to note that forking it now could potentially lead to missing important
security updates, critical features, and performance improvements.
