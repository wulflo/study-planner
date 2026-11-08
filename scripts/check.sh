#!/usr/bin/env bash
# Verificación completa de Omega. No requiere red si las dependencias ya están instaladas.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> Frontend: typecheck + lint + tests + build"
npm run check

echo "==> Rust: formato"
cargo fmt --all -- --check

echo "==> Rust: clippy (core, store, transfer)"
cargo clippy -p omega-core -p omega-store -p omega-transfer --all-targets -- -D warnings

echo "==> Rust: tests (core, store, transfer)"
cargo test -p omega-core -p omega-store -p omega-transfer

if [[ "${SKIP_TAURI_CHECK:-0}" != "1" ]]; then
  echo "==> Rust: check del host Tauri"
  cargo check -p omega
else
  echo "==> Rust: check del host Tauri omitido (SKIP_TAURI_CHECK=1)"
fi

echo "==> OK: verificación completa"
