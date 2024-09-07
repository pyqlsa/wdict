#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${0}")"/..

nix develop --command bash -c "pushd utils/readme && cargo run && popd"
