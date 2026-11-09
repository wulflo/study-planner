# Empaquetado y releases

> **Estado:** pipeline implementado y clave pública de producción configurada
> (2026-11-09). Falta custodiar la passphrase/copias y configurar los secrets para
> publicar la primera release firmada.

## Decisión

El AppImage de release se compila **en GitHub Actions**, no en el equipo local.
La compilación local sirve solo para validar el empaquetado antes de un cambio
grande; el artefacto que se publica sale del workflow `.github/workflows/release.yml`.

## Workflow de release

Se dispara con un tag `v*` o manualmente indicando un tag existente:

```bash
git tag -a v0.2.0 -m "Study Planner 0.2.0"
git push origin v0.2.0
```

Pasos del workflow:

1. Instala Node, Rust, dependencias del sistema y `npm ci`.
2. Exige que `crates/study-planner-updater/updater.pub` no sea el placeholder.
   Sin clave pública real no se publica una release que el updater no podrá usar.
3. Compila `npm run tauri build -- --bundles appimage`.
4. Ejecuta `node scripts/package-release.mjs`, que:
   - firma el AppImage con `tauri signer sign --app-version <versión>`
     (el trusted comment incluye `version:<versión>`);
   - copia el artefacto como `study-planner_<versión>_x86_64.AppImage`;
   - calcula SHA-256;
   - genera `dist-release/latest.json` con la URL HTTPS del asset.
5. Verifica la firma contra la clave pública embebida con
   `cargo run -p study-planner-updater --example verify_release`.
6. Crea o actualiza la release de GitHub con el AppImage, su `.sig` y
   `latest.json`, y la marca como *latest*.

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

## Paquete `.deb`

El objetivo inicial es AppImage. El workflow pide solo `--bundles appimage` para
no publicar paquetes que el updater no puede reemplazar. Un `.deb` opcional se
puede generar con:

```bash
npm run tauri build -- --bundles deb
```

No se publica como parte del updater.

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
