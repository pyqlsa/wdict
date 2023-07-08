#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

cd "$(dirname "${0}")"

nix develop --command bash -c "./_lint.sh"
