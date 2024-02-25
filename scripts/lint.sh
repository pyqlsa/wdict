#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

workDir="${1-"$(dirname "${0}")"/..}"

pushd "${workDir}"
nix flake check --all-systems
popd
