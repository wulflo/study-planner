# Empaquetado y releases

> **Estado:** pipeline multiplataforma implementado (2026-11-10). Clave pública de
> producción configurada y secrets `TAURI_SIGNING_PRIVATE_KEY` /
> `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` cargados en GitHub. El updater funciona
> en AppImage, `.deb`, `.rpm`, Windows (NSIS/MSI) y macOS (`.dmg`); Snap queda
> gestionado por snapd y el MSIX oculta el updater porque se actualiza desde la
> Microsoft Store. Falta custodiar la passphrase/copias offline y publicar
> la primera release firmada real. Los instaladores NSIS/MSI y macOS siguen sin
> firma de código del sistema operativo; el MSIX se firma en Microsoft Store.
> Ver límites más abajo.

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
el snap, y de `.github/workflows/store-msix.yml`, que arma el MSIX para
Microsoft Store por separado.

| Plataforma | Runner | Paquetes |
|---|---|---|
| Linux x86_64 | `ubuntu-24.04` | `.AppImage`, `.deb`, `.rpm` |
| Windows x64 | `windows-latest` | `.exe` (NSIS) y `.msi` (el MSIX va aparte) |
| macOS arm64 | `macos-15` | `.dmg` |

Pasos del workflow:

1. Instala Node, Rust, dependencias del sistema (solo Linux) y `npm ci`.
2. Exige que `crates/study-planner-updater/updater.pub` no sea el placeholder.
3. Compila `npm run tauri build -- --bundles <matriz>`:
   - Linux: `appimage,deb,rpm`.
   - Windows: `nsis,msi`.
   - macOS: `dmg`.
4. `node scripts/package-artifacts.mjs` firma `.deb`, `.rpm`, `.exe`, `.msi` y
   `.dmg`, copia cada artefacto con su `.sig`, escribe un `SHA256SUMS` por
   plataforma y un manifiesto parcial `manifest-<plataforma>-<arquitectura>.json`.
5. En Linux, `node scripts/package-release.mjs` firma el AppImage con
   `tauri signer sign --app-version <versión>`, lo copia como
   `study-planner_<versión>_x86_64.AppImage` y escribe
   `dist-release/manifest-appimage.json`.
6. En Linux, verifica la firma del AppImage contra la clave pública embebida con
   `cargo run -p study-planner-updater --example verify_release`.
7. El job `release` descarga los artefactos de las tres plataformas, comprueba
   que `PRIVACY.md` exista, ejecuta `node scripts/merge-manifests.mjs` para
   fusionar los manifiestos parciales en `latest.json` y crea o actualiza la
   release de GitHub, marcada como *latest*.

La firma de Tauri es una firma minisign de integridad. **No** es Authenticode en
Windows ni notarización de Apple en macOS. El MSIX no se firma localmente:
Microsoft Store aplica su propia firma al publicar.

### Límites por plataforma

- **Linux:** el updater reemplaza el AppImage de forma atómica. Los paquetes
  `.deb` y `.rpm` se instalan con `pkexec` + `dpkg`/`dnf`/`zypper`/`rpm` después
  de verificar la firma. Las notificaciones D-Bus funcionan.
- **Windows:** el updater ejecuta el instalador NSIS (`/S`) o MSI
  (`msiexec /qn`), espera a que la app se cierre y deja que el instalador
  actualice. La detección prefiere MSI si la app corre desde `%ProgramFiles%` y
  NSIS en el resto de los casos. En el MSIX el updater queda oculto: la app corre
  desde `WindowsApps` y la actualización la gestiona Microsoft Store.
- **MSIX:** el paquete se sube sin firmar al Partner Center; Microsoft Store
  aplica su identidad y firma al publicar, así que no hace falta ningún
  certificado local. `MSIX_PUBLISHER` e `MSIX_IDENTITY_NAME` deben coincidir con
  los valores de la app reservada. El runtime detectado es `msix` y no admite
  actualizaciones automáticas.
- **macOS:** el updater monta el `.dmg`, reemplaza el bundle `.app` y lo reabre.
  Gatekeeper puede pedir confirmación porque el `.dmg` no está notarizado.
- **Snap:** snapd gestiona las actualizaciones desde la Snap Store; el updater
  automático queda oculto para no escribir fuera del confinamiento.
- La firma de Tauri es de integridad. Los instaladores NSIS/MSI y macOS siguen
  sin Authenticode ni notarización de Apple; el MSIX se firma en la Store. Ver
  más abajo.

### Secrets de firma del updater

| Secret | Contenido |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Contenido completo de la clave privada minisign (no una ruta) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase de la clave |

Se cargan desde archivos, nunca se pegan en YAML, issues ni chat:

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < /ruta/segura/study-planner.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < /ruta/segura/passphrase.txt
```

### Secrets y variables del MSIX

Valores reservados en el Partner Center para Study Planner (no son secretos:
aparecen en el manifiesto del paquete y en la ficha pública):

| Campo del Partner Center | Valor | Variable |
|---|---|---|
| `Package/Identity/Name` | `wulfo.StudyPlannerMax` | `MSIX_IDENTITY_NAME` |
| `Package/Identity/Publisher` | `CN=7F51EF66-8407-4798-A8F4-3C4A407DA359` | `MSIX_PUBLISHER` |
| `Package/Properties/PublisherDisplayName` | `wulfo` | `MSIX_PUBLISHER_DISPLAY` |
| `Package/Properties/DisplayName` | `Study Planner Max` | `MSIX_DISPLAY_NAME` |
| Package Family Name (PFN) | `wulfo.StudyPlannerMax_m5406cz1kxxs8` | derivada; no se configura |
| Store ID | `9N1B9V99VGHR` | solo para la ficha de la Store |

`scripts/prepare-store-msix.mjs` y el workflow `store-msix.yml` usan estos
valores por defecto, así que no hace falta configurar nada para que el `.msix`
coincida con la app reservada. Las variables solo se necesitan para
sobrescribirlos (otra app u otra cuenta) y, si se definen, deben coincidir
exactamente con el Partner Center: la Store rechaza el paquete cuando
`Identity Name`, `Publisher` o `PublisherDisplayName` no coinciden.
`MSIX_DISPLAY_NAME` no lo valida la Store, pero conviene que sea el mismo nombre
reservado para que el mosaico instalado y la ficha coincidan. No se usa ningún
certificado ni contraseña de firma.

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
    },
    "linux-x86_64-deb": {
      "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/Study_Planner_0.2.0_amd64.deb",
      "signature": "<base64>",
      "sha256": "<64 hex>"
    },
    "linux-x86_64-rpm": {
      "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/Study_Planner-0.2.0-1.x86_64.rpm",
      "signature": "<base64>",
      "sha256": "<64 hex>"
    },
    "windows-x86_64": {
      "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/Study_Planner_0.2.0_x64-setup.exe",
      "signature": "<base64>",
      "sha256": "<64 hex>"
    },
    "windows-x86_64-msi": {
      "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/Study_Planner_0.2.0_x64_en-US.msi",
      "signature": "<base64>",
      "sha256": "<64 hex>"
    },
    "darwin-aarch64": {
      "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/Study_Planner_0.2.0_aarch64.dmg",
      "signature": "<base64>",
      "sha256": "<64 hex>"
    }
  }
}
```

Las claves de plataforma que usa el updater son:

| Entorno | Clave |
|---|---|
| AppImage | `linux-<arch>` |
| `.deb` | `linux-<arch>-deb` |
| `.rpm` | `linux-<arch>-rpm` |
| Windows NSIS | `windows-<arch>` |
| Windows MSI | `windows-<arch>-msi` |
| macOS | `darwin-<arch>` |

`<arch>` es `x86_64` o `aarch64`. El MSIX (`msix`) no tiene clave de plataforma:
su actualización la gestiona la Store, así que el updater no consulta el
manifiesto desde esa instalación.

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

- `.deb`: el updater verifica la firma, escribe el paquete en un directorio
  temporal privado y ejecuta `pkexec dpkg -i <paquete>`.
- `.rpm`: igual, pero usa `pkexec dnf install -y --nogpgcheck <paquete>`,
  `pkexec zypper --non-interactive install --allow-unsigned-rpm <paquete>` o
  `pkexec rpm -U --replacepkgs <paquete>` según el gestor disponible.
- La app sigue corriendo la versión anterior hasta que se reinicia.
- El AppImage se reemplaza de forma atómica y conserva el binario anterior como
  `.old` para el rollback.

Para generarlos localmente:

```bash
npm run tauri build -- --bundles deb,rpm
```

`rpm` debe estar instalado en el sistema (`sudo apt install rpm` en Ubuntu).

## Windows y macOS

Los jobs de Windows y macOS compilan los mismos crates. La app funciona sin
`zbus`; las notificaciones locales devuelven `Las notificaciones locales solo
están disponibles en Linux.`

- **Windows:** el updater ejecuta el instalador NSIS con `/S` o MSI con
  `msiexec /i <archivo> /qn /norestart`. Después cierra la app para que el
  instalador pueda reemplazar los archivos.
- **macOS:** el updater monta el `.dmg`, copia el `.app` nuevo, reemplaza el
  bundle y lo reabre con `open`.

La firma que produce el workflow es minisign (`TAURI_SIGNING_PRIVATE_KEY`), útil
para verificar integridad. No reemplaza la firma de código del sistema operativo:

- Windows: Authenticode para NSIS/MSI requiere un certificado de code signing.
  El MSIX no lo necesita porque lo firma Microsoft Store.
- macOS: Gatekeeper requiere Developer ID + notarización de Apple.

Hasta no agregar esas firmas, Windows/macOS pueden mostrar advertencias al
instalar o al abrir la app.

## MSIX y Microsoft Store

Tauri 2.12 solo genera NSIS y MSI en Windows, así que el MSIX se arma en un
workflow independiente, `.github/workflows/store-msix.yml`, con el mismo patrón
que better-tracker y SinNotas:

1. Se dispara a mano con `gh workflow run store-msix.yml` o desde la pestaña
   *Actions*. No publica en GitHub Releases ni toca los artefactos del workflow
   de release.
2. Compila solo el ejecutable con `npm run tauri build -- --no-bundle` y
   `scripts/prepare-store-msix.mjs` copia `target/release/study-planner.exe` y
   los íconos de tienda desde `src-tauri/icons` (`StoreLogo.png`,
   `Square44x44Logo.png`, `Square150x150Logo.png` y `Square310x310Logo.png`) a
   `target/msix/stage`.
3. Genera `AppxManifest.xml` desde
   `packaging/windows-store/AppxManifest.template.xml` con `Identity Name`,
   `Publisher`, versión MSIX de cuatro componentes, `runFullTrust` y
   `Windows.FullTrustApplication`. El manifiesto no declara `uap:DefaultTile`:
   MakeAppx rechaza `Square310x310Logo` sin `Wide310x150Logo`.
4. Empaqueta con `MakeAppx.exe pack /d target/msix/stage /p <salida> /o`. No
   firma nada y deja `study-planner_<versión>_x64.msix` como artefacto del
   workflow (`store-msix`).
5. Falla si no existe `PRIVACY.md`: Microsoft Store exige una política de
   privacidad pública, enlazada desde la ficha del producto. La URL pública es
   `https://github.com/wulflo/study-planner/blob/main/PRIVACY.md`.

El manifiesto usa por defecto la identidad reservada en el Partner Center:
`Identity Name` `wulfo.StudyPlannerMax`, `Publisher`
`CN=7F51EF66-8407-4798-A8F4-3C4A407DA359`, `PublisherDisplayName` `wulfo` y
`DisplayName` `Study Planner Max`. Los tres primeros deben coincidir
exactamente con el Partner Center; si no, la Store rechaza la carga. La PFN
(`wulfo.StudyPlannerMax_m5406cz1kxxs8`) se deriva de `Identity Name` y
`Publisher`, así que si alguno cambia, la PFN también. Se pueden sobrescribir con
`MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER`, `MSIX_PUBLISHER_DISPLAY` y
`MSIX_DISPLAY_NAME`. La app detecta que corre desde `WindowsApps` y oculta todo
el panel de actualizaciones en «Ajustes» con el aviso «Esta instalación se
actualiza desde Microsoft Store».

### Publicación en la Store

El workflow no sube nada al Partner Center. El `.msix` queda como artefacto
`store-msix` del run; se descarga y se sube manualmente como paquete de la
aplicación en el Partner Center. No hace falta firmarlo antes: Microsoft Store
lo vuelve a firmar con la identidad de la cuenta al certificar la publicación.
El mismo paquete no se puede instalar por sideloading sin un certificado de
confianza, pero ese no es el flujo previsto.

Verificación local (requiere Windows SDK):

```powershell
npm run tauri build -- --no-bundle
node scripts/prepare-store-msix.mjs
makeappx pack /d target\msix\stage /p target\msix\out\study-planner_0.1.3_x64.msix /o
```

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
