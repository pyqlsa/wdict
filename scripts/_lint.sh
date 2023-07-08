#!/usr/bin/env bash
set -euo pipefail
IFS=$' \t\n'

cd "$(dirname "${0}")"/..

result=$(cargo-fmt fmt --all -- --check)
if [[ ${result} ]]; then
  echo "The following files are not formatted according to 'cargo-fmt'"
  echo "${result}"
  exit 1
fi

cargo check

shopt -s globstar nullglob
for file in ./scripts/**; do
  if [[ "${file}" =~ .*\.sh$ ]]; then
    shellcheck --severity=info "${file}"
  fi
done
