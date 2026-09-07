#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

if [[ -n "${CARGO:-}" ]]; then
  CARGO_BIN="$CARGO"
elif command -v cargo >/dev/null 2>&1; then
  CARGO_BIN="$(command -v cargo)"
elif [[ -x /home/irom/.cargo/bin/cargo ]]; then
  CARGO_BIN="/home/irom/.cargo/bin/cargo"
else
  echo "error: cargo is required (install Rust or set CARGO=/path/to/cargo)" >&2
  exit 2
fi
# pnpm scripts invoke `cargo` by name; make the same toolchain available to
# those checks when Rust was installed through rustup in a non-login shell.
export PATH="$(dirname -- "$CARGO_BIN"):$PATH"

CORPUS_DIR="${FLIGHTLENS_CORPUS:-/home/irom/fpv_cli_dumps/backups}"
CORPUS_ARGS=("$CORPUS_DIR")
if [[ "${FLIGHTLENS_CORPUS_STRICT:-0}" == "1" ]]; then
  CORPUS_ARGS+=(--strict)
fi

echo "== Rust formatting =="
"$CARGO_BIN" fmt --all -- --check

echo "== Rust core tests =="
"$CARGO_BIN" test -p flightlens-core

echo "== Generated bindings =="
pnpm bindings:check

echo "== TypeScript =="
pnpm typecheck

echo "== Frontend tests =="
pnpm test

echo "== Renderer behavior =="
pnpm test:ui

echo "== Real-backup corpus coverage =="
"$CARGO_BIN" run --quiet -p flightlens-core --bin corpus_check -- "${CORPUS_ARGS[@]}"

echo "All FlightLens checks passed."
