#!/usr/bin/env bash
# Fork-local verification for the `blob-shared-key-support` delta.
#
# Run after every rebase onto upstream main, before using the branch:
#
#   ./scripts/fork-verify.sh
#
# The emulator checks are opt-in. Export the emulator key first:
#
#   AZURE_STORAGE_EMULATOR=1 AZURE_STORAGE_ACCESS_KEY=... ./scripts/fork-verify.sh
#
# This script is part of the fork delta. It is not an upstream file.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

packages=(
  typespec_client_core
  azure_core
  azure_storage_common
  azure_storage_blob
  azure_storage_queue
)

crates_dir="sdk/storage/azure_storage_blob/tests/fixtures"
captured_xml="$crates_dir/list_blobs_delimited.xml"
captured_json="$crates_dir/list_blobs_delimited.json"

step() {
  printf '\n== %s\n' "$1"
}

step "fixtures"
# The captured page is the contract for delimiter and BlobPrefix. A rebase
# that drops either one must fail here, not in the consuming application.
missing=0
for file in "$captured_xml" "$captured_json"; do
  if [[ ! -f "$file" ]]; then
    printf 'missing %s\n' "$file" >&2
    missing=1
  fi
done
if [[ "$missing" -ne 0 ]]; then
  if [[ "${FORK_ALLOW_MISSING_FIXTURES:-0}" == "1" ]]; then
    printf 'WARNING: fixtures missing, allowed by FORK_ALLOW_MISSING_FIXTURES=1.\n' >&2
    printf 'The delimiter and BlobPrefix contract is NOT verified.\n' >&2
  else
    printf 'Capture one real delimited listing page first. See %s/README.md.\n' "$crates_dir" >&2
    exit 1
  fi
else
  printf 'captured page present\n'
fi

step "typespec_client_core (%20 encoding)"
cargo test -q -p typespec_client_core --lib

step "azure_core"
cargo test -q -p azure_core --lib

step "azure_storage_common (SharedKey signing)"
cargo test -q -p azure_storage_common

step "azure_storage_blob (parser fixtures)"
cargo test -q -p azure_storage_blob --test list_blobs_parsing

step "azure_storage_blob (fork contract)"
cargo test -q -p azure_storage_blob --test fork_contract

step "azure_storage_blob (doc examples)"
cargo test -q -p azure_storage_blob --doc

if [[ "${AZURE_STORAGE_EMULATOR:-0}" == "1" ]]; then
  step "azure_storage_blob (emulator)"
  if [[ -z "${AZURE_STORAGE_ACCESS_KEY:-}" ]]; then
    printf 'AZURE_STORAGE_ACCESS_KEY is required when AZURE_STORAGE_EMULATOR=1\n' >&2
    exit 1
  fi
  cargo test -q -p azure_storage_blob --test emulator
else
  step "emulator (skipped)"
  printf 'Set AZURE_STORAGE_EMULATOR=1 and AZURE_STORAGE_ACCESS_KEY to run the live checks.\n'
fi

step "fmt"
cargo fmt -p "${packages[@]}" -- --check

step "clippy"
cargo clippy -q -p "${packages[@]}" --all-targets

printf '\nfork verify passed\n'
