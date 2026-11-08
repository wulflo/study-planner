# Changelog

Todos los cambios relevantes de Omega se registran acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado pretende seguir [SemVer](https://semver.org/lang/es/).

## [No publicado]

### Agregado

- Nada todavía.

## [0.1.0] — 2026-11-08 — Fase 0: base técnica

### Agregado

- Scaffold de Tauri 2 + React 19 + TypeScript + Vite 8.
- Workspace Rust con `omega-core`, `omega-store`, `omega-transfer`, `omega-updater` y `src-tauri`.
- Esquema SQLite v1 con entidades académicas, horarios, asistencia, exámenes, notas, tareas, temas, repaso, apuntes, adjuntos, notificaciones y auditoría.
- `Store` con WAL, claves foráneas, `synchronous=FULL`, migración con checksum SHA-256, `vault_id`, `integrity_check` y `foreign_key_check`.
- Comandos Tauri `get_app_info` y `schema_self_check` con base en memoria.
- Shell de UI con navegación, estado técnico, marca Omega e ícono SVG local.
- CSP de producción y desarrollo, `freezePrototype` y `withGlobalTauri: false`.
- Capacidad mínima `core:default`; eliminación del plugin `opener`.
- Tests de UI con Vitest/Testing Library y tests de Rust para esquema y migraciones.
- Documentación persistente: checklist, arquitectura, modelo de datos, formato de exportación, updater, claves, decisiones, modo offline y licencias.
- Scripts `scripts/check.sh` y `scripts/audit-network.sh`.
- CI sin secretos en `.github/workflows/ci.yml`.
- Repositorio remoto configurado en [`wulflo/study-planner`](https://github.com/wulflo/study-planner); `main` con commit inicial.
- `.gitignore` endurecido para claves, bases, secretos y backups.

### Cambiado

- La app se renombró de `tauri-app` a `Omega`; identificador `app.omega.studyplan`.
- El gestor de paquetes es npm, con `package-lock.json` versionado.
- La persistencia queda exclusivamente en Rust; el frontend no tiene permisos de `fs`, `http`, `sql`, `shell` ni `opener`.

### Seguridad

- La webview no puede iniciar requests externos.
- El updater está desactivado (`UPDATER_IMPLEMENTED = false`) y no tiene dependencias de red.
- No se generaron claves ni secretos.
- La bóveda no se cifra por decisión explícita; la confidencialidad depende del disco del sistema.

### Conocido / pendiente

- Revisión de la licencia transitiva MPL-2.0 de Tauri (`option-ext` en runtime; `cssparser`, `selectors`, `dtoa-short` solo en build). Ver `THIRD_PARTY_LICENSES.md`.
- La exportación `.txt` está especificada pero se implementa en Fase 1.
- El updater se implementa en Fase 5.
- El repositorio es local y remoto: [`wulflo/study-planner`](https://github.com/wulflo/study-planner) es público; no hay releases todavía.
- La demo de Fase 0 y la confirmación del usuario están pendientes.

## [0.0.0] — 2026-11-08

- Punto de partida del proyecto. Sin código previo.
