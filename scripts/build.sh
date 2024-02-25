#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

cd "$(dirname "${0}")"/..

if [ "${1-}" == "release" ]; then
	nix develop --command bash -c "cargo build --release"
else
	nix develop --command bash -c "cargo build"
fi
