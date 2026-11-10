#!/usr/bin/env bash
# Verificación completa de Study Planner. No requiere red si las dependencias ya están instaladas.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> Frontend: typecheck + lint + tests + build"
npm run check

echo "==> Rust: formato"
cargo fmt --all -- --check

echo "==> Rust: clippy (core, store, transfer, updater)"
cargo clippy -p study-planner-core -p study-planner-store -p study-planner-transfer -p study-planner-updater --all-targets -- -D warnings

echo "==> Rust: clippy (host Tauri)"
cargo clippy -p study-planner --all-targets -- -D warnings

echo "==> Rust: tests (core, store, transfer, updater)"
cargo test -p study-planner-core -p study-planner-store -p study-planner-transfer -p study-planner-updater

echo "==> Rust: tests (host Tauri / bóveda)"
cargo test -p study-planner

if [[ "${SKIP_TAURI_CHECK:-0}" != "1" ]]; then
  echo "==> Rust: check del host Tauri"
  cargo check -p study-planner
else
  echo "==> Rust: check del host Tauri omitido (SKIP_TAURI_CHECK=1)"
fi

echo "==> Snap: versión sincronizada"
bash scripts/check-snap-version.sh

echo "==> Auditoría de red, permisos y secretos"
bash scripts/audit-network.sh

echo "==> Auditoría de licencias"
bash scripts/audit-licenses.sh

echo "==> OK: verificación completa"
