#!/usr/bin/env node
// Empaqueta Study Planner como MSIX para Windows.
//
// Tauri 2.12 no genera MSIX de forma nativa (solo NSIS y MSI), así que este
// script toma el ejecutable ya compilado y lo empaqueta con MakeAppx.exe del
// Windows SDK. Corre en el job de Windows de .github/workflows/release.yml.
//
// El paquete sale SIN firmar a propósito: se sube tal cual al Partner Center y
// Microsoft Store aplica su propia identidad y firma al publicar. No hace falta
// ningún certificado local.
//
// Requisitos:
//   - Windows con el Windows SDK (MakeAppx.exe).
//   - `npm run tauri build` ya ejecutado (necesita target/release/<binario>.exe).
//   - PRIVACY.md en la raíz (obligatoria para publicar en Microsoft Store).
//
// Variables de entorno (opcionales; por defecto usan la identidad reservada en
// el Partner Center de Study Planner):
//   MSIX_PUBLISHER           Publisher del Partner Center (CN=...)
//   MSIX_PUBLISHER_DISPLAY   nombre visible del publicador
//   MSIX_IDENTITY_NAME       Identity Name asignado por el Partner Center
//   MSIX_DISPLAY_NAME        nombre visible del producto en el paquete
//   MSIX_DESCRIPTION         descripción del manifiesto
//   RELEASE_OUT_DIR          por defecto dist-release
//   RELEASE_TAG              por defecto v<versión>
//   GITHUB_REPOSITORY        por defecto wulflo/study-planner
//
// No publica nada: deja el .msix en dist-release y lo suma al SHA256SUMS.
// `MSIX_PUBLISHER` e `MSIX_IDENTITY_NAME` deben coincidir exactamente con los
// valores que muestra el Partner Center para la app reservada; si no, la Store
// rechaza la carga. Los valores por defecto son los de la app reservada:
//   Identity Name:   wulfo.StudyPlannerMax
//   Publisher:       CN=7F51EF66-8407-4798-A8F4-3C4A407DA359
//   PublisherDisplay: wulfo
//   DisplayName:     Study Planner Max (nombre reservado en la Store)
//   PFN:             wulfo.StudyPlannerMax_m5406cz1kxxs8 (derivada de los dos
//                    valores anteriores; no se configura en el manifiesto)
//   Store ID:        9N1B9V99VGHR

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  appendFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

function fail(message) {
  console.error(`ERROR: ${message}`);
  process.exit(1);
}

function log(message) {
  console.log(message);
}

function escapeXml(value) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function archName(arch) {
  return arch === "x64" ? "x86_64" : arch === "arm64" ? "aarch64" : arch;
}

function msixVersion(semver) {
  const [core, prerelease] = semver.split("-");
  const parts = core.split(".").map((part) => Number.parseInt(part, 10));
  while (parts.length < 4) {
    parts.push(0);
  }
  if (
    parts.length !== 4 ||
    parts.some((part) => !Number.isInteger(part) || part < 0)
  ) {
    fail(`la versión ${semver} no es compatible con MSIX`);
  }
  if (prerelease) {
    log(`AVISO: ${semver} tiene prerelease; el MSIX usa ${parts.join(".")}`);
  }
  return parts.join(".");
}

function findSdkTool(name) {
  const candidates = [];
  const roots = [
    "C:\\Program Files (x86)\\Windows Kits\\10\\bin",
    "C:\\Program Files\\Windows Kits\\10\\bin",
  ];
  for (const rootDir of roots) {
    if (!existsSync(rootDir)) {
      continue;
    }
    const pending = [rootDir];
    while (pending.length > 0) {
      const directory = pending.pop();
      let entries;
      try {
        entries = readdirSync(directory, { withFileTypes: true });
      } catch {
        continue;
      }
      for (const entry of entries) {
        const full = path.join(directory, entry.name);
        if (entry.isDirectory()) {
          pending.push(full);
        } else if (
          entry.isFile() &&
          entry.name.toLowerCase() === name.toLowerCase()
        ) {
          candidates.push(full);
        }
      }
    }
  }
  const x64 = candidates.find((candidate) =>
    candidate.toLowerCase().includes(`${path.sep}x64${path.sep}`),
  );
  return x64 ?? candidates[0] ?? null;
}

if (process.platform !== "win32") {
  fail("este script solo corre en Windows");
}

const root = process.cwd();
const config = JSON.parse(
  readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);
const version = config.version;
if (!version) {
  fail("src-tauri/tauri.conf.json no declara version");
}
const displayName = process.env.MSIX_DISPLAY_NAME || "Study Planner Max";

const cargo = readFileSync(path.join(root, "src-tauri", "Cargo.toml"), "utf8");
const packageName = /^name = "([^"]+)"/m.exec(cargo)?.[1];
if (!packageName) {
  fail("no se pudo leer el nombre del paquete de src-tauri/Cargo.toml");
}

if (!existsSync(path.join(root, "PRIVACY.md"))) {
  fail("falta PRIVACY.md; es obligatoria para publicar en Microsoft Store");
}

const tag = process.env.RELEASE_TAG ?? `v${version}`;
if (tag.replace(/^v/, "") !== version) {
  fail(`el tag ${tag} no coincide con la versión ${version}`);
}

const repo = process.env.GITHUB_REPOSITORY ?? "wulflo/study-planner";
const outDir = process.env.RELEASE_OUT_DIR ?? "dist-release";
const arch =
  process.arch === "x64" ? "x64" : process.arch === "arm64" ? "arm64" : null;
if (!arch) {
  fail(`arquitectura no soportada: ${process.arch}`);
}

const exeSource = path.join(root, "target", "release", `${packageName}.exe`);
if (!existsSync(exeSource)) {
  fail(`no se encontró ${exeSource}; ejecutá npm run tauri build primero`);
}

const stageDir = path.join(root, "target", "msix", "stage");
rmSync(stageDir, { recursive: true, force: true });
mkdirSync(stageDir, { recursive: true });
cpSync(exeSource, path.join(stageDir, `${packageName}.exe`));

const assetsDir = path.join(stageDir, "Assets");
mkdirSync(assetsDir, { recursive: true });
const iconsDir = path.join(root, "src-tauri", "icons");
const iconFiles = [
  "StoreLogo.png",
  "Square44x44Logo.png",
  "Square150x150Logo.png",
  "Square310x310Logo.png",
  "Square71x71Logo.png",
  "Square89x89Logo.png",
  "Square107x107Logo.png",
  "Square142x142Logo.png",
  "Square284x284Logo.png",
];
for (const name of iconFiles) {
  const source = path.join(iconsDir, name);
  if (!existsSync(source)) {
    fail(`falta el ícono ${source}`);
  }
  cpSync(source, path.join(assetsDir, name));
}

const publisher =
  process.env.MSIX_PUBLISHER || "CN=7F51EF66-8407-4798-A8F4-3C4A407DA359";
const publisherDisplay = process.env.MSIX_PUBLISHER_DISPLAY || "wulfo";
const identityName = process.env.MSIX_IDENTITY_NAME || "wulfo.StudyPlannerMax";
if (process.env.MSIX_PUBLISHER && !/^CN=.+/.test(publisher)) {
  fail(`MSIX_PUBLISHER debe ser un sujeto CN=...; recibido: ${publisher}`);
}
log(`Identity Name:    ${identityName}`);
log(`DisplayName:      ${displayName}`);
log(`Publisher:        ${publisher}`);
log(`PublisherDisplay: ${publisherDisplay}`);
const description =
  process.env.MSIX_DESCRIPTION ??
  "Organización académica personal, estrictamente local.";
const manifest = `<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity
    Name="${escapeXml(identityName)}"
    Version="${msixVersion(version)}"
    Publisher="${escapeXml(publisher)}"
    ProcessorArchitecture="${arch}" />
  <Properties>
    <DisplayName>${escapeXml(displayName)}</DisplayName>
    <PublisherDisplayName>${escapeXml(publisherDisplay)}</PublisherDisplayName>
    <Description>${escapeXml(description)}</Description>
    <Logo>Assets\\StoreLogo.png</Logo>
  </Properties>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.22621.0" />
  </Dependencies>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
  <Applications>
    <Application Id="App" Executable="${packageName}.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="${escapeXml(displayName)}"
        Description="${escapeXml(description)}"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\\Square150x150Logo.png"
        Square44x44Logo="Assets\\Square44x44Logo.png">
        <uap:DefaultTile
          Square310x310Logo="Assets\\Square310x310Logo.png"
          Square71x71Logo="Assets\\Square71x71Logo.png"
          Square89x89Logo="Assets\\Square89x89Logo.png"
          Square107x107Logo="Assets\\Square107x107Logo.png"
          Square142x142Logo="Assets\\Square142x142Logo.png"
          Square284x284Logo="Assets\\Square284x284Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>
</Package>
`;
writeFileSync(path.join(stageDir, "AppxManifest.xml"), manifest);

const makeAppx = findSdkTool("MakeAppx.exe");
if (!makeAppx) {
  fail("no se encontró MakeAppx.exe; instalá el Windows SDK");
}

const assetName = `study-planner_${version}_${arch}.msix`;
const outPath = path.join(root, outDir, assetName);
mkdirSync(outDir, { recursive: true });
const pack = spawnSync(
  makeAppx,
  ["pack", "/d", stageDir, "/p", outPath, "/o"],
  {
    stdio: "inherit",
  },
);
if (pack.status !== 0) {
  fail(`MakeAppx falló con código ${pack.status}`);
}

const bytes = readFileSync(outPath);
const sha256 = createHash("sha256").update(bytes).digest("hex");
const sumsName = `SHA256SUMS-windows-${archName(arch)}.txt`;
const sumsPath = path.join(root, outDir, sumsName);
appendFileSync(sumsPath, `${sha256}  ${assetName}\n`);

log(`MSIX:    ${outPath}`);
log(
  `URL:     https://github.com/${repo}/releases/download/${tag}/${assetName}`,
);
log(`SHA-256: ${sha256}`);
log("Firma:   no local; Microsoft Store firma al publicar");
log(`Sumas:   ${sumsPath}`);
