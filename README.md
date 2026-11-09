# Study Planner

Organización académica personal, **estrictamente local**. Horarios, materias, exámenes, notas, tareas, apuntes y repaso espaciado en una sola bóveda SQLite, sin cuentas, sin nube y sin telemetría.

**Repositorio:** [github.com/wulflo/study-planner](https://github.com/wulflo/study-planner) (público, solo código; nunca datos de la bóveda).

> **Estado:** Fases 0 a 5 cerradas y publicadas: `v0.1.5` con AppImage, `.deb`, `.rpm`, Windows (NSIS/MSI/MSIX) y macOS, updater firmado multiplataforma, notificaciones locales y empaquetado MSIX para Microsoft Store. Snap `study-planner` 0.1.0 en `stable` (revisión 1); la próxima revisión usa 0.1.5. La app incluye temas claro/oscuro/sistema, colores de acento, modo segundo plano con bandeja y recordatorios con sonido opcional. La ficha pública del snap puede tardar mientras Canonical revisa el primer envío.

## Principios

- **Local primero:** elegís una carpeta raíz (la bóveda) y la app crea `<bóveda>/omega` para los datos vivos y el backup principal verificado, y `<bóveda>/longlife` para las snapshots diarias. La ruta se valida y nunca cambia en silencio.
- **Continuidad verificada:** cada backup se comprueba con SHA-256, `integrity_check` y `foreign_key_check`; se conserva el backup previo a cada migración, las últimas 7 snapshots por dispositivo y un backup `pre-restore-*.db` antes de cada restauración.
- **Sin red en la interfaz:** la webview no tiene permisos de archivos, SQL, shell, diálogos ni HTTP. La única excepción es el updater aislado en Rust, opt-in, con URL fija y firma obligatoria.
- **Exportación total:** un único `.txt` canónico que permite reconstruir la bóveda. `exportar → importar → exportar` da bytes idénticos en la misma versión; la importación solo se acepta en una bóveda vacía.
- **Sin cifrado de bóveda:** decisión explícita del usuario. La confidencialidad depende del disco cifrado del sistema y de la custodia física.
- **Continuidad:** cada fase deja tests, changelog y `docs/CHECKLIST.md` para retomar sin internet. La MPL-2.0 transitiva de Tauri está aceptada como excepción acotada (ADR-017); ver `THIRD_PARTY_LICENSES.md`.

## Stack

| Capa | Tecnología |
|---|---|
| UI | React 19, TypeScript, Vite 8, Zustand, date-fns, Material Color Utilities |
| Host | Tauri 2.12 (Rust) |
| Persistencia | SQLite con `rusqlite` (bundled), WAL, migraciones versionadas |
| Exportación | Rust (`study-planner-transfer`), formato canónico propio |
| Updater | Rust aislado (`study-planner-updater`), opt-in y con firma minisign |
| Empaquetado | AppImage, deb, rpm, exe/msi/msix, dmg y snap `strict` |
| Tests | Vitest + Testing Library; `cargo test` |

## Requisitos

- Linux x86_64 (probado en CachyOS/Arch). Windows x64 y macOS arm64 se compilan y actualizan, pero no tienen notificaciones locales ni firma de código del sistema.
- Node.js 22+ y npm.
- Rust estable 1.90+ con `cargo`.
- WebKitGTK 4.1, GTK 3, libsoup 3 y dependencias de Tauri.
- En Linux, `libayatana-appindicator3` para el icono de bandeja del modo segundo plano. Si falta, la app sigue funcionando pero cerrar la ventana termina el proceso.
- `rg` (ripgrep) para `scripts/audit-network.sh`; si falta, el script usa `grep`.

## Arranque

```bash
npm install
npm run tauri dev
```

Solo frontend (sin IPC de Tauri):

```bash
npm run dev
```

## Verificación

```bash
npm run check          # typecheck + lint + tests + build
npm run check:all      # lo anterior + fmt + clippy + tests Rust
./scripts/check.sh     # atajo de check:all + cargo check -p study-planner
./scripts/audit-network.sh
```

## Documentación

- [`docs/CHECKLIST.md`](docs/CHECKLIST.md) — estado, fases, reglas y próximos pasos.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — capas, seguridad y flujos.
- [`docs/DATA_MODEL.md`](docs/DATA_MODEL.md) — esquema y convenciones.
- [`docs/EXPORT_FORMAT.md`](docs/EXPORT_FORMAT.md) — contrato del `.txt`.
- [`docs/UPDATER.md`](docs/UPDATER.md) — diseño del updater aislado.
- [`docs/NOTIFICATIONS.md`](docs/NOTIFICATIONS.md) — recordatorios locales, sonido y segundo plano.
- [`docs/RELEASE.md`](docs/RELEASE.md) — workflow multiplataforma, firma y secrets.
- [`docs/SNAP.md`](docs/SNAP.md) — snap estricto, build y publicación.
- [`docs/KEYS.md`](docs/KEYS.md) — custodia de la clave de firma.
- [`PRIVACY.md`](PRIVACY.md) — política de privacidad para Microsoft Store.
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — decisiones y descartes.
- [`docs/OFFLINE.md`](docs/OFFLINE.md) — trabajo sin internet.
- [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md) — licencias y excepciones.
- [`CHANGELOG.md`](CHANGELOG.md) — cambios por versión.

## Estructura

```text
src/                 UI React
src-tauri/           host Tauri, comandos, CSP y permisos
crates/study-planner-core/    dominio puro, constantes, IDs
crates/study-planner-store/   SQLite, migraciones, backups
crates/study-planner-transfer/ exportación/importación
crates/study-planner-updater/  red aislada (Fase 5)
docs/                documentación persistente
scripts/             verificación y auditoría
fixtures/export/     fixtures de round-trip
```

La bóveda vive fuera del repositorio:

```text
<bóveda>/omega/                          datos vivos, backup principal, adjuntos y log local
<bóveda>/longlife/<dispositivo>/<fecha>/ snapshots diarias (7 por dispositivo)
```

## Seguridad

- No hay claves, contraseñas ni bases de datos en el repositorio.
- El snap usa confinamiento `strict`, solo `home`/`removable-media` y ningún plug de red; ver `docs/SNAP.md`.
- No se aceptan contribuciones que agreguen red, cuentas, telemetría o permisos amplios al frontend.
- La clave privada del updater y su passphrase se custodian fuera del repo; ver `docs/KEYS.md`.
- Si encontrás un problema de seguridad, usá el [reporte privado de GitHub](https://github.com/wulflo/study-planner/security/advisories/new). No abras un issue público. Ver [`SECURITY.md`](SECURITY.md).

## Licencia

MIT OR Apache-2.0. Las dependencias de plataforma (WebKitGTK/GTK) son LGPL y se enlazan dinámicamente; ver `THIRD_PARTY_LICENSES.md`.
