# zkSync Era: The EraVM Compiler Integration Test Framework

[![Logo](eraLogo.svg)](https://zksync.io/)

zkSync Era is a layer 2 rollup that uses zero-knowledge proofs to scale Ethereum without compromising on security
or decentralization. As it's EVM-compatible (with Solidity/Vyper), 99% of Ethereum projects can redeploy without
needing to refactor or re-audit any code. zkSync Era also uses an LLVM-based compiler that will eventually enable
developers to write smart contracts in popular languages such as C++ and Rust.

The `compiler-tester` integration test framework runs tests for Matter Labs compilers which target the EraVM,
for supported languages listed below. It compiles source code via external API calls,
e.g. to [Inkwell](https://thedan64.github.io/inkwell/inkwell/index.html). In software quality assurance jargon,
this makes it a whitebox testing framework.

The `compiler-tester` repository includes the Compiler Tests Collection repository as a submodule.

By default, the Tester SHOULD run the entire Collection in all possible combinations of compiler versions and settings,
but it MAY omit some subset of the combinations for the sake of saving time, e.g. when only front-end changes have been
made, and there is no point in running tests in all LLVM optimization modes.

## Building

1. Install some tools system-wide:  
   1.a. `apt install cmake ninja-build clang-13 lld-13 parallel pkg-config` on a Debian-based Linux, with optional `musl-tools` if you need a `musl` build  
   1.b. `pacman -S cmake ninja clang lld parallel` on an Arch-based Linux  
   1.c. On MacOS, install the [HomeBrew](https://brew.sh) package manager (being careful to install it as the appropriate user), then `brew install cmake ninja coreutils parallel`. Install your choice of a recent LLVM/[Clang](https://clang.llvm.org) compiler, e.g. via [Xcode](https://developer.apple.com/xcode/), [Apple’s Command Line Tools](https://developer.apple.com/library/archive/technotes/tn2339/_index.html), or your preferred package manager.  
   1.d. Their equivalents with other package managers  

2. [Install Rust](https://www.rust-lang.org/tools/install).

3. Check out or clone the appropriate branch of this repository:  
   3.a. If you have not cloned this repository yet:  
   ```
   git clone <THIS_REPO_URL> --recursive
   ```
   3.b. If you have already cloned this repository:  
   ```
   git submodule update --init --recursive --remote
   ```

4. Pull, build, or specify the path to your LLVM framework build:  
   4.a. If you have not cloned the LLVM repository yet:  
   ```
   cargo install compiler-llvm-builder
   zkevm-llvm clone && zkevm-llvm build
   ```
   4.b. If you have already cloned the LLVM repository:  
   ```
   cargo install compiler-llvm-builder
   zkevm-llvm checkout
   git -C './llvm/' pull
   zkevm-llvm build
   ```
   4.c. If you would like to use your local LLVM build:
   ```
   export LLVM_SYS_150_PREFIX='<ABSOLUTE_PATH_TO_YOUR_LOCAL_LLVM_BUILD>'
   ```

5. Build [zksolc](https://github.com/matter-labs/era-compiler-solidity) and [zkvyper](https://github.com/matter-labs/era-compiler-vyper) compilers and add the binaries to `$PATH`, or use the `--zksolc` or `--zkvyper` options to specify their paths.

6. Build the Tester with `cargo build --release`.

7. Run the tests using [the examples below](#usage).

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
- EVM assembly from Yul (`y`)
- EVM assembly pure (`E`)
- Vyper LLL (`V`)

### Compiler versions

- `>=0.8` for compiling Solidity via Yul
- `>=0.8.13` for compiling Solidity via EVM assembly from Yul
- [0.4.10; latest] for compiling Solidity via EVM assembly
- [0.3.3, 0.3.9] for compiling Vyper via LLL IR

### Compiler pipelines

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
<<<<<<< HEAD
- Solidity compiler version (`0.8.24`)
=======
- Solidity compiler version (`0.8.23`)
>>>>>>> d020375 (Final sync with the private repo)

Output:

- failed and invalid tests only (absence of `-v`)
- the compiler debug data to the `./debug/` directory (`-D`)
- the VM trace data to the `./trace/` directory (`-T`)

```bash
cargo run --release --bin compiler-tester -- -DT \
	--path='tests/solidity/simple/default.sol' \
<<<<<<< HEAD
	--mode='Y+M3B3 0.8.24' \
=======
	--mode='Y+M3B3 0.8.23' \
>>>>>>> d020375 (Final sync with the private repo)
	--zksolc '../era-compiler-solidity/target/release/zksolc'
```

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

## Tracing

If you run the tester with `-T` flag, JSON trace files will be written to the `./trace/` directory.
The trace files can be used with our [custom zkSync EraVM assembly tracer](https://staging-scan-v2.zksync.dev/tools/debugger) for debugging and research purposes.

## Benchmarking

1. Change the LLVM branch to the base in the `LLVM.lock` file at the repository root, checkout and build it:
```
zkevm-llvm checkout && zkevm-llvm build
```

2. Run the Tester with the desired filters and the output JSON path:
```
./target/release/compiler-tester \
	--path='tests/solidity/simple/default.sol' \
<<<<<<< HEAD
	--mode='Y+M^B3 0.8.24' \
=======
	--mode='Y+M^B3 0.8.23' \
>>>>>>> d020375 (Final sync with the private repo)
	--benchmark='reference.json'
```

3. Change the LLVM branch to your patch in the `LLVM.lock` file at the repository root, checkout and build it:
```
zkevm-llvm checkout && zkevm-llvm build
```

4. Run the Tester with the desired filters and the output JSON path:
```
./target/release/compiler-tester \
	--path='tests/solidity/simple/default.sol' \
<<<<<<< HEAD
	--mode='Y+M^B3 0.8.24' \
=======
	--mode='Y+M^B3 0.8.23' \
>>>>>>> d020375 (Final sync with the private repo)
	--benchmark='candidate.json'
```

5. Run the benchmark analyzer on the two JSONs:
```
cargo run --release --bin benchmark-analyzer -- --reference reference.json --candidate candidate.json
```

After you make any changes in LLVM, you only need to repeat steps 2-3 to update the working branch benchmark data.

## Troubleshooting

- If you get a “failed to authenticate when downloading repository… if the git CLI succeeds then net.git-fetch-with-cli may help here” error,
then prepending the `cargo` command with `CARGO_NET_GIT_FETCH_WITH_CLI=true`
may help.
- On MacOS, `git config --global credential.helper osxkeychain` followed by cloning a repository manually with a personal access token may help.
- Unset any LLVM-related environment variables you may have set, especially `LLVM_SYS_<version>_PREFIX` (see e.g. [https://crates.io/crates/llvm-sys](https://crates.io/crates/llvm-sys) and [https://llvm.org/docs/GettingStarted.html#local-llvm-configuration](https://llvm.org/docs/GettingStarted.html#local-llvm-configuration)). To make sure: `set | grep LLVM`.

## License

The Compiler Tester is distributed under the terms of either

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Resources

[zkSync Era compiler toolchain documentation](https://era.zksync.io/docs/api/compiler-toolchain)

## Official Links

- [Website](https://zksync.io/)
- [GitHub](https://github.com/matter-labs)
- [Twitter](https://twitter.com/zksync)
- [Twitter for Devs](https://twitter.com/zkSyncDevs)
- [Discord](https://join.zksync.dev/)

## Disclaimer

zkSync Era has been through extensive testing and audits, and although it is live, it is still in alpha state and
will undergo further audits and bug bounty programs. We would love to hear our community's thoughts and suggestions
about it!
It's important to note that forking it now could potentially lead to missing important
security updates, critical features, and performance improvements.
