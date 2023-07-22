#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

cd "$(dirname "${0}")"/..

nix develop --command bash -c "RUST_BACKTRACE=1 cargo test"
