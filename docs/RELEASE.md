# Empaquetado y releases

> **Estado:** pipeline multiplataforma implementado (2026-11-10). Clave pública de
> producción configurada y secrets `TAURI_SIGNING_PRIVATE_KEY` /
> `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` cargados en GitHub. Falta custodiar la
> passphrase/copias offline y publicar la primera release firmada real.
> Windows/macOS son builds experimentales; ver límites más abajo.

## Decisión

Los paquetes de release se compilan **en GitHub Actions**, no en el equipo local.
La compilación local sirve solo para validar el empaquetado antes de un cambio
grande; los artefactos que se publican salen de
`.github/workflows/release.yml` (escritorio) y `.github/workflows/snap.yml` (snap).

## Workflow de release

Se dispara con un tag `v*` o manualmente indicando un tag existente:

```bash
git tag -a v0.2.0 -m "Study Planner 0.2.0"
git push origin v0.2.0
```

El workflow `.github/workflows/release.yml` compila en una matriz de tres
plataformas. Es independiente de `.github/workflows/snap.yml`, que solo construye
el snap.

| Plataforma | Runner | Paquetes |
|---|---|---|
| Linux x86_64 | `ubuntu-24.04` | `.AppImage`, `.deb`, `.rpm` |
| Windows x64 | `windows-latest` | `.exe` (NSIS) y `.msi` |
| macOS arm64 | `macos-14` | `.dmg` |

Pasos del workflow:

1. Instala Node, Rust, dependencias del sistema (solo Linux) y `npm ci`.
2. Exige que `crates/study-planner-updater/updater.pub` no sea el placeholder.
3. Compila `npm run tauri build -- --bundles <matriz>`:
   - Linux: `appimage,deb,rpm`.
   - Windows: `nsis,msi`.
   - macOS: `dmg`.
4. En Linux, `node scripts/package-release.mjs` firma el AppImage con
   `tauri signer sign --app-version <versión>`, lo copia como
   `study-planner_<versión>_x86_64.AppImage` y genera
   `dist-release/latest.json`.
5. `node scripts/package-artifacts.mjs` firma `.deb`, `.rpm`, `.exe`, `.msi` y
   `.dmg`, copia cada artefacto con su `.sig` y escribe un `SHA256SUMS` por
   plataforma.
6. En Linux, verifica la firma del AppImage contra la clave pública embebida con
   `cargo run -p study-planner-updater --example verify_release`.
7. El job `release` descarga los artefactos de las tres plataformas y crea o
   actualiza la release de GitHub, marcada como *latest*.

La firma de Tauri es una firma minisign de integridad. **No** es Authenticode en
Windows ni notarización de Apple en macOS: esos pasos, si se agregan, usan
certificados distintos.

### Límites por plataforma

- **Linux:** es la plataforma completa. El updater de AppImage y las
  notificaciones D-Bus funcionan.
- **Windows/macOS:** builds experimentales. La app compila sin `zbus`, pero las
  notificaciones locales devuelven un error claro y el updater queda inactivo
  (no hay AppImage). No hay firma de código ni notarización.
- La actualización automática solo existe en Linux/AppImage y Snap. Los paquetes
  `.deb`/`.rpm` se actualizan con el gestor del sistema; `.exe`/`.msi` y `.dmg`
  con el instalador correspondiente.

### Secrets requeridos

| Secret | Contenido |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Contenido completo de la clave privada minisign (no una ruta) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase de la clave |

Se cargan desde archivos, nunca se pegan en YAML, issues ni chat:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < /ruta/segura/study-planner.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < /ruta/segura/passphrase.txt
```

## URL fija y manifiesto

La app consulta siempre:

```text
https://github.com/wulflo/study-planner/releases/latest/download/latest.json
```

Antes de etiquetar una release, `./scripts/check.sh` debe estar verde: incluye
tests, clippy, auditoría de red/permisos/secretos y auditoría de licencias.

El manifiesto generado tiene esta forma (los hosts deben pertenecer a la lista
permitida del updater):

```json
{
  "version": "0.2.0",
  "pub_date": "2026-11-09T20:00:00.000Z",
  "notes": "Study Planner 0.2.0.",
  "platforms": {
    "linux-x86_64": {
      "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/study-planner_0.2.0_x86_64.AppImage",
      "signature": "<base64 de la firma minisign>",
      "sha256": "<64 hex>"
    }
  }
}
```

La release debe ser la **latest** (no borrador ni pre-release) para que
`releases/latest/download/latest.json` resuelva al manifiesto correcto. El
workflow usa `gh release create --latest`.

## Verificación manual de un artefacto

Sin red, con la clave pública embebida:

```bash
cargo run -q -p study-planner-updater --example verify_release -- \
  study-planner_0.2.0_x86_64.AppImage \
  study-planner_0.2.0_x86_64.AppImage.sig \
  0.2.0
```

Si la firma declara otra versión, el comando falla con
`la firma declara la versión X, pero el manifiesto anuncia Y`.

## Release offline (alternativa)

Si no se quiere usar la clave en CI:

1. Compilar el AppImage en una máquina con la clave privada.
2. Firmar con `--app-version` y generar el manifiesto con el mismo formato.
3. Subir manualmente `latest.json`, el AppImage y su `.sig` a una release
   marcada como latest.
4. Verificar con `verify_release` antes de publicar.

La firma offline es la opción recomendada mientras la clave no tenga un
procedimiento de custodia en CI.

## Paquetes `.deb` y `.rpm`

El workflow de Linux compila `--bundles appimage,deb,rpm`. Los tres se firman
con la clave de Tauri y se adjuntan a la release.

- `.deb`: se instala con `apt`/`dpkg`; no lo reemplaza el updater de la app.
- `.rpm`: se instala con `dnf`/`rpm`; tampoco lo reemplaza el updater.
- El AppImage sigue siendo el único artefacto Linux que el updater puede
  reemplazar de forma atómica.

Para generarlos localmente:

```bash
npm run tauri build -- --bundles deb,rpm
```

`rpm` debe estar instalado en el sistema (`sudo apt install rpm` en Ubuntu).

## Windows y macOS

Los jobs de Windows y macOS compilan los mismos crates, pero con dos límites
explícitos:

- **Notificaciones:** `zbus` y D-Bus son solo Linux. En otras plataformas la
  entrega devuelve `Las notificaciones locales solo están disponibles en Linux.`
- **Updater:** el instalador de actualizaciones reemplaza un AppImage. En
  Windows/macOS queda inactivo.

La firma que produce el workflow es minisign (`TAURI_SIGNING_PRIVATE_KEY`), útil
para verificar integridad. No reemplaza la firma de código del sistema operativo:

- Windows: Authenticode requiere un certificado de code signing.
- macOS: Gatekeeper requiere Developer ID + notarización de Apple.

Hasta no agregar esas firmas, Windows/macOS son builds experimentales y pueden
mostrar advertencias al instalar.

## Rotación y recuperación

El protocolo completo está en `docs/KEYS.md`. Antes de la primera release real
hay que ejecutar el simulacro de recuperación y rotación; el resultado se anota
en `docs/CHECKLIST.md` sin incluir claves ni passphrases.

Simulación ya verificada (2026-11-09) con una clave de test fuera del repo:

- Se generó un par de prueba y se firmó un AppImage real con `version:0.1.0`.
- `verify_release` aceptó la firma y el SHA-256.
- Con una versión distinta, la verificación rechazó la firma.
- La clave de prueba quedó en `/tmp/opencode` y no se versionó.

La rotación real sigue este orden: clave nueva + release firmada con ambas,
transición, retiro de la vieja y actualización de `updater.pub`, `docs/KEYS.md`
y este documento.
