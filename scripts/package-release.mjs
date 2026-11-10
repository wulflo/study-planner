#!/usr/bin/env node
// Prepara el artefacto y el manifiesto de una release de Study Planner.
//
// Requiere el AppImage ya compilado y las variables de firma de Tauri:
//   TAURI_SIGNING_PRIVATE_KEY           contenido de la clave privada
//   TAURI_SIGNING_PRIVATE_KEY_PASSWORD  passphrase
// Opcionales:
//   GITHUB_REPOSITORY   por defecto wulflo/study-planner
//   RELEASE_TAG         por defecto v<versión>
//   RELEASE_OUT_DIR     por defecto dist-release
//   RELEASE_NOTES       notas visibles en la app
//
// No publica nada: genera dist-release/latest.json, el AppImage renombrado y
// su .sig para que el workflow los suba a GitHub.

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

const tag = process.env.RELEASE_TAG ?? `v${version}`;
if (tag.replace(/^v/, "") !== version) {
  fail(`el tag ${tag} no coincide con la versión ${version}`);
}

const repo = process.env.GITHUB_REPOSITORY ?? "wulflo/study-planner";
const outDir = process.env.RELEASE_OUT_DIR ?? "dist-release";
const bundleDir = "target/release/bundle/appimage";

function archName(arch) {
  return arch === "x64" ? "x86_64" : arch === "arm64" ? "aarch64" : arch;
}

const arch = archName(process.arch);
const platform = `linux-${arch}`;

const candidates = (await readdir(bundleDir)).filter((name) =>
  name.endsWith(".AppImage"),
);
if (candidates.length !== 1) {
  fail(`se esperaba un AppImage en ${bundleDir}, hay ${candidates.length}`);
}
const artifact = path.join(bundleDir, candidates[0]);

if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
  fail("falta TAURI_SIGNING_PRIVATE_KEY; no se publica sin firma");
}

// Firma con la CLI de Tauri: embebe `version:<versión>` en el trusted comment.
// Se invoca el JS de la CLI directamente para que funcione igual en Windows.
const tauriCli = path.join("node_modules", "@tauri-apps", "cli", "tauri.js");
const sign = spawnSync(
  process.execPath,
  [tauriCli, "signer", "sign", "--app-version", version, artifact],
  { stdio: "inherit", env: process.env },
);
if (sign.status !== 0) {
  fail("la firma del AppImage falló");
}

const signaturePath = `${artifact}.sig`;
const signature = (await readFile(signaturePath, "utf8")).trim();
if (!signature) {
  fail("la firma quedó vacía");
}

await mkdir(outDir, { recursive: true });
const assetName = `study-planner_${version}_${arch}.AppImage`;
const assetPath = path.join(outDir, assetName);
await copyFile(artifact, assetPath);
await copyFile(signaturePath, `${assetPath}.sig`);

const bytes = await readFile(assetPath);
const sha256 = createHash("sha256").update(bytes).digest("hex");
const url = `https://github.com/${repo}/releases/download/${tag}/${assetName}`;

const notes =
  process.env.RELEASE_NOTES ??
  `Study Planner ${version}. Actualización firmada para Linux, Windows y macOS.`;

const manifest = {
  version,
  pub_date: new Date().toISOString(),
  notes,
  platforms: {
    [platform]: {
      url,
      signature,
      sha256,
    },
  },
};

const manifestJson = `${JSON.stringify(manifest, null, 2)}\n`;
await writeFile(path.join(outDir, "manifest-appimage.json"), manifestJson);
// Se mantiene para uso local; el job de release lo regenera fusionando todas
// las plataformas antes de publicar.
await writeFile(path.join(outDir, "latest.json"), manifestJson);

console.log(`AppImage: ${assetPath}`);
console.log(`Firma:   ${assetPath}.sig`);
console.log(`SHA-256: ${sha256}`);
console.log(`Manifiesto: ${path.join(outDir, "latest.json")}`);
console.log(`URL:     ${url}`);
