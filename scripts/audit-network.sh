#!/usr/bin/env bash
# Auditoría estática de red, permisos y secretos. No hace requests.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
root="$PWD"
failed=0

fail() {
  echo "FALLO: $*" >&2
  failed=1
}

ok() {
  echo "OK: $*"
}

# --- Capacidades y CSP -------------------------------------------------------
node - "$root" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");

const root = process.argv[2];
const failures = [];
const forbidden = [
  "fs:", "http:", "sql:", "shell:", "opener:", "dialog:",
  "notification:", "updater:", "process:", "os:",
];

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(root, file), "utf8"));
}

const capDir = path.join(root, "src-tauri", "capabilities");
let permissions = [];
for (const name of fs.readdirSync(capDir)) {
  if (!name.endsWith(".json")) continue;
  const cap = readJson(path.join("src-tauri", "capabilities", name));
  permissions = permissions.concat(cap.permissions ?? []);
}
for (const permission of permissions) {
  const id = typeof permission === "string" ? permission : permission.identifier;
  if (forbidden.some((prefix) => id.startsWith(prefix))) {
    failures.push(`permiso prohibido en capabilities: ${id}`);
  }
}
if (!permissions.includes("core:default")) {
  failures.push("capabilities no incluye core:default");
}

const conf = readJson("src-tauri/tauri.conf.json");
if (conf.app?.withGlobalTauri !== false) {
  failures.push("withGlobalTauri debe ser false");
}
if (conf.app?.security?.freezePrototype !== true) {
  failures.push("freezePrototype debe ser true");
}

const csp = conf.app?.security?.csp;
if (typeof csp !== "string") {
  failures.push("falta la CSP de producción");
} else {
  if (/unsafe-eval/i.test(csp)) failures.push("la CSP de producción contiene unsafe-eval");
  if (/https?:\/\//i.test(csp.replace(/http:\/\/ipc\.localhost/g, ""))) {
    failures.push("la CSP de producción contiene orígenes HTTP remotos");
  }
  if (!/default-src\s+'self'/.test(csp)) failures.push("CSP sin default-src 'self'");
  if (!/script-src\s+'self'/.test(csp)) failures.push("CSP sin script-src 'self'");
  if (!/connect-src\s+ipc:\s+http:\/\/ipc\.localhost/.test(csp)) {
    failures.push("connect-src no está limitado al IPC de Tauri");
  }
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`FALLO: ${failure}`);
  process.exit(1);
}
console.log("OK: capacidades, CSP, withGlobalTauri y freezePrototype");
NODE

# --- Plugins de Tauri --------------------------------------------------------
if grep -RInE 'tauri-plugin-(sql|http|fs|shell|opener|notification|updater)' \
  src-tauri/Cargo.toml src-tauri/src 2>/dev/null; then
  fail "plugin de Tauri prohibido en el host"
else
  ok "sin plugins de Tauri que amplíen permisos de red/archivos"
fi

# El diálogo nativo se usa solo desde Rust (`DialogExt`) y no debe llegar al
# frontend: sin permiso `dialog:` ni paquete npm del plugin.
if grep -RInE 'tauri-plugin-dialog' src-tauri/Cargo.toml src-tauri/src >/dev/null 2>&1; then
  if grep -RInE 'dialog:' src-tauri/capabilities >/dev/null 2>&1; then
    fail "el plugin dialog está expuesto al frontend"
  elif grep -RInE '@tauri-apps/plugin-dialog' package.json src >/dev/null 2>&1; then
    fail "el frontend importa el plugin dialog"
  else
    ok "tauri-plugin-dialog solo desde Rust, sin permiso dialog"
  fi
fi

# --- Updater aislado ---------------------------------------------------------
if grep -q 'UPDATER_IMPLEMENTED: bool = false' crates/study-planner-updater/src/lib.rs; then
  if grep -Eq '(reqwest|ureq|hyper|curl|tauri-plugin-updater|tokio)' crates/study-planner-updater/Cargo.toml; then
    fail "study-planner-updater tiene dependencias de red antes de Fase 5"
  else
    ok "study-planner-updater desactivado y sin red"
  fi
else
  echo "AVISO: el updater está marcado como implementado; revisar manualmente su red."
fi

# --- URLs remotas en la UI ---------------------------------------------------
if command -v rg >/dev/null 2>&1; then
  urls="$(rg -n --no-heading -e 'https?://' src index.html public src-tauri/tauri.conf.json 2>/dev/null || true)"
else
  urls="$(grep -RInE 'https?://' src index.html public src-tauri/tauri.conf.json 2>/dev/null || true)"
fi

if [[ -n "$urls" ]]; then
  filtered="$(printf '%s\n' "$urls" | grep -vE \
    'http://www\.w3\.org/|https://schema\.tauri\.app/|http://localhost:1420|ws://localhost:1421|http://ipc\.localhost' || true)"
  if [[ -n "$filtered" ]]; then
    printf '%s\n' "$filtered" >&2
    fail "URL remota no permitida en la UI"
  else
    ok "sin URLs remotas en la UI (solo namespaces y desarrollo local)"
  fi
else
  ok "sin URLs en la UI"
fi

# --- Secretos versionados ----------------------------------------------------
if [[ -d .git ]] && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  tracked="$(git ls-files)"
  secret_hits="$(printf '%s\n' "$tracked" | grep -E \
    '(^|/)(\.env($|\.)|secrets?/|study-planner-keys?/|keys?/)|\.(key|pem|pfx|p12|jks|keystore|minisign|db|sqlite|sqlite3)$|(^|/)minisign\.key$' || true)"
  if [[ -n "$secret_hits" ]]; then
    printf '%s\n' "$secret_hits" >&2
    fail "archivos sensibles versionados"
  else
    ok "sin secretos, claves ni bases versionadas"
  fi
else
  ok "repo git aún no inicializado; revisar secretos después del primer commit"
fi

if [[ "$failed" -ne 0 ]]; then
  echo "==> Auditoría con fallos" >&2
  exit 1
fi

echo "==> OK: auditoría de red y permisos"
