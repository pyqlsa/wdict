#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

cd "$(dirname "${0}")"/..

if [ "${1-}" == "pub" ]; then
	nix develop --command bash -c "cargo publish"
else
	nix develop --command bash -c "cargo package"
fi
