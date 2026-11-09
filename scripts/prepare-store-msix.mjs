#!/usr/bin/env node
/**
 * Prepara el payload y el AppxManifest.xml del MSIX para Microsoft Store.
 *
 * La identidad no se puede inventar: `MSIX_IDENTITY_NAME` y `MSIX_PUBLISHER`
 * tienen que ser exactamente los valores de Partner Center →
 * Product management → Product identity. El publisher por defecto es el ID de
 * la cuenta; el nombre del paquete lo asigna la Store y no puede contener
 * espacios.
 *
 * Este script solo prepara `target/msix/stage`. El empaquetado final lo hace
 * `MakeAppx.exe` en `.github/workflows/store-msix.yml`. El paquete sale sin
 * firmar a propósito: Microsoft Store aplica su identidad y firma al publicar.
 */
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEFAULT_IDENTITY_NAME = "wulflo.StudyPlannerMax";
const DEFAULT_PUBLISHER = "CN=7F51EF66-8407-4798-A8F4-3C4A407DA359";
const DEFAULT_DISPLAY_NAME = "Study Planner Max";
const DEFAULT_PUBLISHER_DISPLAY_NAME = "wulflo";
const STAGING_DIR = resolve(ROOT, "target", "msix", "stage");
const ASSETS = [
  "StoreLogo.png",
  "Square44x44Logo.png",
  "Square150x150Logo.png",
  "Square310x310Logo.png",
];
const RESERVED_NAMES = new Set([
  ".",
  "..",
  "con",
  "prn",
  "aux",
  "nul",
  "com1",
  "com2",
  "com3",
  "com4",
  "com5",
  "com6",
  "com7",
  "com8",
  "com9",
  "lpt1",
  "lpt2",
  "lpt3",
  "lpt4",
  "lpt5",
  "lpt6",
  "lpt7",
  "lpt8",
  "lpt9",
]);

function fail(message) {
  console.error(`\nERROR: ${message}\n`);
  process.exit(1);
}

function xmlEscape(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

/** 0.9.13 -> 0.9.13.0; MSIX exige cuatro componentes numéricos. */
function toMsixVersion(version) {
  const base = version.trim().replace(/^v/i, "").split(/[-+]/, 1)[0];
  const parts = base.split(".");
  if (parts.length < 1 || parts.length > 4) {
    fail(
      `La versión "${version}" no se puede convertir a Major.Minor.Build.Revision.`,
    );
  }
  const numbers = parts.map((part) => Number(part));
  if (numbers.some((n) => !Number.isInteger(n) || n < 0 || n > 65535)) {
    fail(`La versión "${version}" tiene componentes fuera del rango 0-65535.`);
  }
  while (numbers.length < 4) {
    numbers.push(0);
  }
  return numbers.join(".");
}

function validateIdentityName(name) {
  if (name.length < 3 || name.length > 50) {
    fail("MSIX_IDENTITY_NAME debe tener entre 3 y 50 caracteres.");
  }
  if (!/^[A-Za-z0-9.-]+$/.test(name)) {
    fail(
      `MSIX_IDENTITY_NAME="${name}" contiene caracteres inválidos. ` +
        "Solo se permiten letras, números, puntos y guiones (sin espacios).",
    );
  }
  if (name.endsWith(".") || name.toLowerCase().includes(".xn--")) {
    fail(`MSIX_IDENTITY_NAME="${name}" no es un nombre de paquete válido.`);
  }
  if (
    RESERVED_NAMES.has(name.toLowerCase()) ||
    /^(con|prn|aux|nul|com[1-9]|lpt[1-9])\./i.test(name)
  ) {
    fail(`MSIX_IDENTITY_NAME="${name}" usa un nombre reservado por Windows.`);
  }
}

function firstExisting(paths, description) {
  const found = paths.find((candidate) => existsSync(candidate));
  if (!found) {
    fail(
      `No se encontró ${description}. Ejecutá 'npm run tauri build -- --no-bundle' primero.`,
    );
  }
  return found;
}

const identityName = (
  process.env.MSIX_IDENTITY_NAME ?? DEFAULT_IDENTITY_NAME
).trim();
const publisher = (process.env.MSIX_PUBLISHER ?? DEFAULT_PUBLISHER).trim();
const displayName = (
  process.env.MSIX_DISPLAY_NAME ?? DEFAULT_DISPLAY_NAME
).trim();
const publisherDisplayName = (
  process.env.MSIX_PUBLISHER_DISPLAY ?? DEFAULT_PUBLISHER_DISPLAY_NAME
).trim();

if (!displayName) {
  fail(
    "falta MSIX_DISPLAY_NAME. Copiá Package/Properties/DisplayName desde " +
      "Partner Center.",
  );
}

if (!publisherDisplayName) {
  fail(
    "falta MSIX_PUBLISHER_DISPLAY. Copiá " +
      "Package/Properties/PublisherDisplayName desde Partner Center.",
  );
}

if (!identityName) {
  fail(
    "falta MSIX_IDENTITY_NAME. Copiá el valor de Partner Center → " +
      "Product management → Product identity → Package/Identity/Name " +
      "(no es el nombre comercial y no puede tener espacios).",
  );
}
validateIdentityName(identityName);

if (!/^CN=.+/.test(publisher)) {
  fail(
    `MSIX_PUBLISHER="${publisher}" no empieza con CN=. Copiá el valor de ` +
      "Package/Identity/Publisher en Partner Center.",
  );
}

const tauriConfig = JSON.parse(
  readFileSync(resolve(ROOT, "src-tauri", "tauri.conf.json"), "utf8"),
);
if (!tauriConfig.version) {
  fail("src-tauri/tauri.conf.json no declara version");
}
const version = toMsixVersion(tauriConfig.version);

const exeSource = firstExisting(
  [
    resolve(ROOT, "target", "release", "study-planner.exe"),
    resolve(ROOT, "src-tauri", "target", "release", "study-planner.exe"),
  ],
  "target/release/study-planner.exe",
);

const template = readFileSync(
  resolve(ROOT, "packaging", "windows-store", "AppxManifest.template.xml"),
  "utf8",
);

rmSync(STAGING_DIR, { recursive: true, force: true });
mkdirSync(join(STAGING_DIR, "Assets"), { recursive: true });
cpSync(exeSource, join(STAGING_DIR, "study-planner.exe"));

for (const asset of ASSETS) {
  const source = resolve(ROOT, "src-tauri", "icons", asset);
  if (!existsSync(source)) {
    fail(
      `Falta el ícono ${source}. Generá los íconos de Tauri y commiteá el resultado.`,
    );
  }
  cpSync(source, join(STAGING_DIR, "Assets", asset));
}

const manifest = template
  .replaceAll("__IDENTITY_NAME__", xmlEscape(identityName))
  .replaceAll("__PUBLISHER__", xmlEscape(publisher))
  .replaceAll("__VERSION__", version)
  .replaceAll("__DISPLAY_NAME__", xmlEscape(displayName))
  .replaceAll("__PUBLISHER_DISPLAY_NAME__", xmlEscape(publisherDisplayName));

if (/__[A-Z0-9_]+__/.test(manifest)) {
  fail(
    "El AppxManifest.xml generado todavía contiene placeholders sin " +
      "reemplazar. Revisá la plantilla y los valores de identidad.",
  );
}

writeFileSync(join(STAGING_DIR, "AppxManifest.xml"), manifest, "utf8");

console.log(`Payload MSIX preparado: ${STAGING_DIR}`);
console.log(`  Identity:             ${identityName}`);
console.log(`  Publisher:            ${publisher}`);
console.log(`  Version:              ${version}`);
console.log(`  Display:              ${displayName}`);
console.log(`  PublisherDisplayName: ${publisherDisplayName}`);
