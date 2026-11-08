#!/usr/bin/env bash
# Verificación completa de Study Planner. No requiere red si las dependencias ya están instaladas.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> Frontend: typecheck + lint + tests + build"
npm run check

echo "==> Rust: formato"
cargo fmt --all -- --check

echo "==> Rust: clippy (core, store, transfer)"
cargo clippy -p study-planner-core -p study-planner-store -p study-planner-transfer --all-targets -- -D warnings

echo "==> Rust: clippy (host Tauri)"
cargo clippy -p study-planner --all-targets -- -D warnings

echo "==> Rust: tests (core, store, transfer)"
cargo test -p study-planner-core -p study-planner-store -p study-planner-transfer

echo "==> Rust: tests (host Tauri / bóveda)"
cargo test -p study-planner

if [[ "${SKIP_TAURI_CHECK:-0}" != "1" ]]; then
  echo "==> Rust: check del host Tauri"
  cargo check -p study-planner
else
  echo "==> Rust: check del host Tauri omitido (SKIP_TAURI_CHECK=1)"
fi

echo "==> OK: verificación completa"
