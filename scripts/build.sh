#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

cd "$(dirname "${0}")"/..

if [ "${1-}" == "release" ]; then
  cargo build --release
else
  cargo build
fi
