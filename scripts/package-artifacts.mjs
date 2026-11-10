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
//   RELEASE_OUT_DIR   por defecto dist-release
//
// No publica nada: deja los artefactos, sus .sig y un SHA256SUMS por plataforma.

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

const config = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
const version = config.version;
if (!version) {
  fail("src-tauri/tauri.conf.json no declara version");
}

if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  fail("falta TAURI_SIGNING_PRIVATE_KEY; no se firma sin clave");
}

const outDir = process.env.RELEASE_OUT_DIR ?? "dist-release";
const bundleRoot = path.join("target", "release", "bundle");
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
let signed = 0;

for (const [dir, extension] of targets) {
  const bundleDir = path.join(bundleRoot, dir);
  let entries;
  try {
    entries = await readdir(bundleDir);
  } catch {
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

const sumsName = `SHA256SUMS-${process.platform}-${process.arch}.txt`;
await writeFile(path.join(outDir, sumsName), `${checksums.join("\n")}\n`);
console.log(`Sumas: ${path.join(outDir, sumsName)}`);
