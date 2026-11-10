#!/usr/bin/env node
// Firma y prepara los artefactos de escritorio que no cubre package-release.mjs.
//
// Linux: .deb y .rpm (el AppImage lo maneja package-release.mjs).
// Windows: .exe (NSIS) y .msi.
// macOS: .dmg.
//
// Requiere las variables de firma de Tauri:
//   TAURI_SIGNING_PRIVATE_KEY           contenido de la clave privada
//   TAURI_SIGNING_PRIVATE_KEY_PASSWORD  passphrase
// Opcionales:
//   GITHUB_REPOSITORY   por defecto wulflo/study-planner
//   RELEASE_TAG         por defecto v<versión>
//   RELEASE_OUT_DIR     por defecto dist-release
//   RELEASE_NOTES       notas visibles en la app
//
// No publica nada: deja los artefactos, sus .sig, un SHA256SUMS por plataforma
// y un manifiesto parcial (manifest-<plataforma>-<arquitectura>.json) que el
// job de release fusiona en latest.json.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFile,
  mkdir,
  readdir,
  readFile,
  writeFile,
} from "node:fs/promises";
import path from "node:path";

function fail(message) {
  console.error(`ERROR: ${message}`);
  process.exit(1);
}

function archName(arch) {
  return arch === "x64" ? "x86_64" : arch === "arm64" ? "aarch64" : arch;
}

function platformPrefix() {
  const arch = archName(process.arch);
  if (process.platform === "linux") {
    return `linux-${arch}`;
  }
  if (process.platform === "win32") {
    return `windows-${arch}`;
  }
  if (process.platform === "darwin") {
    return `darwin-${arch}`;
  }
  return fail(`plataforma no soportada: ${process.platform}`);
}

function platformKeyFor(extension, prefix) {
  if (process.platform === "linux" && extension === ".deb") {
    return `${prefix}-deb`;
  }
  if (process.platform === "linux" && extension === ".rpm") {
    return `${prefix}-rpm`;
  }
  if (process.platform === "win32" && extension === ".exe") {
    return prefix;
  }
  if (process.platform === "win32" && extension === ".msi") {
    return `${prefix}-msi`;
  }
  if (process.platform === "darwin" && extension === ".dmg") {
    return prefix;
  }
  return null;
}

const config = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
const version = config.version;
if (!version) {
  fail("src-tauri/tauri.conf.json no declara version");
}

const tag = process.env.RELEASE_TAG ?? `v${version}`;
if (tag.replace(/^v/, "") !== version) {
  fail(`el tag ${tag} no coincide con la versión ${version}`);
}

if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  fail("falta TAURI_SIGNING_PRIVATE_KEY; no se firma sin clave");
}

const repo = process.env.GITHUB_REPOSITORY ?? "wulflo/study-planner";
const outDir = process.env.RELEASE_OUT_DIR ?? "dist-release";
const bundleRoot = path.join("target", "release", "bundle");
const prefix = platformPrefix();
const targets = [
  ["deb", ".deb"],
  ["rpm", ".rpm"],
  ["nsis", ".exe"],
  ["msi", ".msi"],
  ["dmg", ".dmg"],
];

const tauriCli = path.join("node_modules", "@tauri-apps", "cli", "tauri.js");

await mkdir(outDir, { recursive: true });

const checksums = [];
const platforms = {};
let signed = 0;

for (const [dir, extension] of targets) {
  const bundleDir = path.join(bundleRoot, dir);
  let entries;
  try {
    entries = await readdir(bundleDir);
  } catch {
    continue;
  }

  const platformKey = platformKeyFor(extension, prefix);
  if (!platformKey) {
    continue;
  }

  for (const name of entries.filter((entry) => entry.endsWith(extension))) {
    const artifact = path.join(bundleDir, name);
    const sign = spawnSync(
      process.execPath,
      [tauriCli, "signer", "sign", "--app-version", version, artifact],
      { stdio: "inherit", env: process.env },
    );
    if (sign.status !== 0) {
      fail(`la firma de ${artifact} falló`);
    }

    const signaturePath = `${artifact}.sig`;
    const signature = (await readFile(signaturePath, "utf8")).trim();
    if (!signature) {
      fail(`la firma de ${artifact} quedó vacía`);
    }

    const assetName = path.basename(name).replace(/\s+/g, "_");
    const assetPath = path.join(outDir, assetName);
    await copyFile(artifact, assetPath);
    await copyFile(signaturePath, `${assetPath}.sig`);

    const bytes = await readFile(assetPath);
    const sha256 = createHash("sha256").update(bytes).digest("hex");
    const url = `https://github.com/${repo}/releases/download/${tag}/${assetName}`;
    platforms[platformKey] = { url, signature, sha256 };
    checksums.push(`${sha256}  ${assetName}`);
    signed += 1;

    console.log(`Artefacto: ${assetPath}`);
    console.log(`Firma:     ${assetPath}.sig`);
    console.log(`SHA-256:   ${sha256}`);
  }
}

if (signed === 0) {
  fail(
    "no se encontraron artefactos .deb, .rpm, .exe, .msi o .dmg para firmar",
  );
}

const notes =
  process.env.RELEASE_NOTES ??
  `Study Planner ${version}. Actualización firmada para Linux, Windows y macOS.`;
const manifest = {
  version,
  pub_date: new Date().toISOString(),
  notes,
  platforms,
};
const manifestName = `manifest-${process.platform}-${process.arch}.json`;
await writeFile(
  path.join(outDir, manifestName),
  `${JSON.stringify(manifest, null, 2)}\n`,
);

const sumsName = `SHA256SUMS-${process.platform}-${process.arch}.txt`;
await writeFile(path.join(outDir, sumsName), `${checksums.join("\n")}\n`);
console.log(`Sumas: ${path.join(outDir, sumsName)}`);
console.log(`Manifiesto parcial: ${path.join(outDir, manifestName)}`);
