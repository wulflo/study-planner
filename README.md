# Omega

Organización académica personal, **estrictamente local**. Horarios, materias, exámenes, notas, tareas, apuntes y repaso espaciado en una sola bóveda SQLite, sin cuentas, sin nube y sin telemetría.

> **Estado:** Fase 0 (base técnica) en cierre. Todavía no hay funcionalidad de dominio; la app muestra el contrato técnico y un autodiagnóstico del esquema.

## Principios

- **Local primero:** los datos viven en `/omega`; las copias en `/loglife`. Ambas rutas son configurables y nunca cambian en silencio.
- **Sin red en la interfaz:** la webview no tiene permisos de archivos, SQL, shell, diálogos ni HTTP. La única excepción futura es un módulo de actualización aislado en Rust, opt-in y con firma obligatoria.
- **Exportación total:** un único `.txt` canónico que permite reconstruir la bóveda. `exportar → importar → exportar` debe dar bytes idénticos en la misma versión.
- **Sin cifrado de bóveda:** decisión explícita del usuario. La confidencialidad depende del disco cifrado del sistema y de la custodia física.
- **Continuidad:** cada fase deja tests, changelog y `docs/CHECKLIST.md` para retomar sin internet.

## Stack

| Capa | Tecnología |
|---|---|
| UI | React 19, TypeScript, Vite 8, Zustand, date-fns, Material Color Utilities |
| Host | Tauri 2.12 (Rust) |
| Persistencia | SQLite con `rusqlite` (bundled), WAL, migraciones versionadas |
| Exportación | Rust (`omega-transfer`), formato canónico propio |
| Updater | Rust aislado (`omega-updater`), pendiente Fase 5 |
| Tests | Vitest + Testing Library; `cargo test` |

## Requisitos

- Linux x86_64 (probado en CachyOS/Arch).
- Node.js 22+ y npm.
- Rust estable 1.90+ con `cargo`.
- WebKitGTK 4.1, GTK 3, libsoup 3 y dependencias de Tauri.
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
./scripts/check.sh     # atajo de check:all + cargo check -p omega
./scripts/audit-network.sh
```

## Documentación

- [`docs/CHECKLIST.md`](docs/CHECKLIST.md) — estado, fases, reglas y próximos pasos.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — capas, seguridad y flujos.
- [`docs/DATA_MODEL.md`](docs/DATA_MODEL.md) — esquema y convenciones.
- [`docs/EXPORT_FORMAT.md`](docs/EXPORT_FORMAT.md) — contrato del `.txt`.
- [`docs/UPDATER.md`](docs/UPDATER.md) — diseño del updater aislado.
- [`docs/KEYS.md`](docs/KEYS.md) — custodia de la clave de firma.
- [`docs/DECISIONS.md`](docs/DECISIONS.md) — decisiones y descartes.
- [`docs/OFFLINE.md`](docs/OFFLINE.md) — trabajo sin internet.
- [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md) — licencias y excepciones.
- [`CHANGELOG.md`](CHANGELOG.md) — cambios por versión.

## Estructura

```text
src/                 UI React
src-tauri/           host Tauri, comandos, CSP y permisos
crates/omega-core/   dominio puro, constantes, IDs
crates/omega-store/  SQLite, migraciones, backups
crates/omega-transfer/ exportación/importación
crates/omega-updater/  red aislada (Fase 5)
docs/                documentación persistente
scripts/             verificación y auditoría
fixtures/export/     fixtures de round-trip
```

## Seguridad

- No hay claves, contraseñas ni bases de datos en el repositorio.
- No se aceptan contribuciones que agreguen red, cuentas, telemetría o permisos amplios al frontend.
- La clave privada del updater y su passphrase se custodian fuera del repo; ver `docs/KEYS.md`.
- Si encontrás un problema de seguridad, no abras un issue público: contactá al mantenedor de forma privada.

## Licencia

MIT OR Apache-2.0. Las dependencias de plataforma (WebKitGTK/GTK) son LGPL y se enlazan dinámicamente; ver `THIRD_PARTY_LICENSES.md`.
