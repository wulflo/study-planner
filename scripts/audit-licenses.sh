#!/usr/bin/env bash
# Auditoría estática de licencias del grafo Cargo. No hace requests.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

if ! cargo metadata --offline --format-version 1 > "$tmp"; then
  echo "FALLO: no se pudo leer el grafo de Cargo (¿falta 'cargo fetch --locked'?)" >&2
  exit 1
fi

python3 - "$tmp" <<'PY'
import collections
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    metadata = json.load(handle)

forbidden = ("AGPL", "LGPL", "GPL", "SSPL", "CPAL")
allowed_but_reviewed = {"MPL-2.0"}
blocked = []
mpl = []
families = collections.Counter()

for package in metadata["packages"]:
    license_text = (package.get("license") or "").strip()
    if not license_text:
        blocked.append((package["name"], package["version"], "sin licencia declarada"))
        continue
    families[license_text] += 1
    alternatives = [part.strip() for part in license_text.split(" OR ")]
    usable = [
        part
        for part in alternatives
        if not any(token in part for token in forbidden)
    ]
    if not usable:
        blocked.append((package["name"], package["version"], license_text))
    if any(token in license_text for token in forbidden) and usable:
        # Expresión con alternativa permisiva: se elige la permisiva.
        continue
    if license_text in allowed_but_reviewed:
        mpl.append((package["name"], package["version"]))

print(f"Paquetes Cargo resueltos: {len(metadata['packages'])}")
for license_text, count in families.most_common(8):
    print(f"  {count:4d}  {license_text}")

if mpl:
    print("Aviso: MPL-2.0 revisada y aceptada en ADR-017:")
    for name, version in mpl:
        print(f"  {name} {version}")

if blocked:
    print("FALLO: licencias no permisivas sin decisión registrada:", file=sys.stderr)
    for name, version, license_text in blocked:
        print(f"  {name} {version}: {license_text}", file=sys.stderr)
    sys.exit(1)

print("OK: sin GPL/AGPL/LGPL/SSPL/CPAL en el grafo resuelto")
PY
