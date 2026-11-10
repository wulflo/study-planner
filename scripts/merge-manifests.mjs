#!/usr/bin/env node
// Fusiona los manifiestos parciales de cada plataforma en latest.json.
//
// Cada job de la matriz de release firma sus artefactos y escribe
// `manifest-<plataforma>-<arquitectura>.json` (o `manifest-appimage.json`).
// Este script corre en el job de publicación, después de descargar todos los
// artefactos, y valida que todos declaren la misma versión.

import { readdir, readFile, unlink, writeFile } from "node:fs/promises";
import path from "node:path";

function fail(message) {
  console.error(`ERROR: ${message}`);
  process.exit(1);
}

const outDir = process.env.RELEASE_OUT_DIR ?? "dist-release";
const entries = await readdir(outDir);
const partialNames = entries
  .filter((name) => name.startsWith("manifest-") && name.endsWith(".json"))
  .sort();

if (partialNames.length === 0) {
  fail(`no hay manifiestos parciales en ${outDir}`);
}

const partials = [];
for (const name of partialNames) {
  const raw = await readFile(path.join(outDir, name), "utf8");
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    fail(`${name} no es JSON válido: ${error.message}`);
  }
  if (!parsed.version || !parsed.platforms) {
    fail(`${name} no declara version o platforms`);
  }
  partials.push({ name, manifest: parsed });
}

const version = partials[0].manifest.version;
for (const { name, manifest } of partials) {
  if (manifest.version !== version) {
    fail(
      `${name} declara ${manifest.version}, pero ${partials[0].name} declara ${version}`,
    );
  }
}

const platforms = {};
for (const { name, manifest } of partials) {
  for (const [key, artifact] of Object.entries(manifest.platforms)) {
    if (platforms[key]) {
      fail(`la plataforma ${key} aparece en más de un manifiesto (${name})`);
    }
    if (!artifact.url || !artifact.signature || !artifact.sha256) {
      fail(`la plataforma ${key} de ${name} está incompleta`);
    }
    platforms[key] = artifact;
  }
}

if (Object.keys(platforms).length === 0) {
  fail("los manifiestos parciales no contienen plataformas");
}

const notes =
  partials.find(({ manifest }) => manifest.notes?.trim())?.manifest.notes ??
  `Study Planner ${version}.`;
const pubDate =
  partials.find(({ manifest }) => manifest.pub_date)?.manifest.pub_date ??
  new Date().toISOString();

const merged = {
  version,
  pub_date: pubDate,
  notes,
  platforms,
};

await writeFile(
  path.join(outDir, "latest.json"),
  `${JSON.stringify(merged, null, 2)}\n`,
);

for (const { name } of partials) {
  await unlink(path.join(outDir, name));
}

console.log(`Manifiesto: ${path.join(outDir, "latest.json")}`);
console.log(`Versión:    ${version}`);
console.log(`Plataformas: ${Object.keys(platforms).join(", ")}`);
