#!/usr/bin/env bash
set -e

preprocess -f era-contracts/system-contracts/contracts/EvmInterpreter.template.yul -d era-contracts/system-contracts/contracts/EvmInterpreterPreprocessed.yul
