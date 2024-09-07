#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${0}")"/..

nix develop --command bash -c "RUST_BACKTRACE=1 cargo test"
