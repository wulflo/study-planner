#!/usr/bin/env bash
# Verifica que la versión del snap coincida con la versión de la app.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

snap_version="$(
  sed -n "s/^version:[[:space:]]*['\"]\{0,1\}\([^'\"]*\)['\"]\{0,1\}[[:space:]]*$/\1/p" \
    snap/snapcraft.yaml | head -n 1
)"
app_version="$(node -p "require('./src-tauri/tauri.conf.json').version")"

if [[ -z "$snap_version" ]]; then
  echo "FALLO: snap/snapcraft.yaml no declara version" >&2
  exit 1
fi

if [[ "$snap_version" != "$app_version" ]]; then
  echo "FALLO: snap version $snap_version != app version $app_version" >&2
  exit 1
fi

echo "OK: snap y app comparten la versión $app_version"
