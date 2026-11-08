# Changelog

Todos los cambios relevantes de Study Planner se registran acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado pretende seguir [SemVer](https://semver.org/lang/es/).

## [No publicado]

### Agregado

- Bóveda local elegible desde la app: el selector nativo se abre solo desde Rust, la raíz se guarda en la configuración de la app y se crean `<bóveda>/omega` y `<bóveda>/longlife/<dispositivo>`.
- Validación de la raíz de la bóveda: debe existir, ser una carpeta escribible, no ser la raíz del sistema de archivos y no estar dentro del repositorio.
- Lock de instancia en `<bóveda>/omega/.study-planner.lock`: una segunda instancia no puede abrir la misma bóveda y recibe un error explícito.
- Backups verificados con la API de backup online de SQLite: escritura `.partial`, verificación antes del renombre atómico, SHA-256, `integrity_check` y `foreign_key_check` antes de considerarlos válidos. Un backup inválido nunca reemplaza al anterior.
- Backup principal en `<bóveda>/omega/backup/study-planner.db`, recreado si falta, está corrupto o tiene un esquema distinto.
- Backup previo verificado (`pre-migration-vN-*.db`) antes de migrar y restauración automática de la base si la migración falla.
- Snapshots diarias en `<bóveda>/longlife/<dispositivo>/<AAAA-MM-DD>` con `study-planner.db` y `snapshot.json`; las incompletas o de esquema viejo se descartan y se conservan las últimas 7 por dispositivo.
- Botón «Crear backup ahora» en la vista «Bóveda», con tamaño, integridad, fecha, SHA-256 del backup y cantidad de snapshots visibles sin exponer rutas.
- Exportación canónica v1 en `<bóveda>`: un `.txt` determinista con todas las tablas, settings portables, adjuntos embebidos en base64 y `body-sha256`. `exportar → importar en bóveda vacía → exportar` produce bytes idénticos.
- Importación transaccional solo en bóveda vacía: valida cabecera, checksum, orden canónico, columnas, FK e integridad; si falla, revierte y no deja archivos parciales. El modo reemplazo queda disponible solo en el núcleo.
- Restauración del backup principal desde la UI con confirmación explícita, backup previo `pre-restore-*.db` y reapertura de la bóveda.
- Log local `<bóveda>/omega/study-planner.log` para backup, exportación, importación, restauración y errores, con timestamp UTC y sin rutas absolutas.
- Adjuntos en `<bóveda>/omega/attachments` identificados por SHA-256, exportados embebidos y verificados al importar.
- La vista «Bóveda» muestra si hay datos y habilita exportar, importar (solo vacía) y restaurar.
- Apertura y migración de SQLite en `<bóveda>/omega/study-planner.db`, con esquema, integridad y claves foráneas visibles en la UI sin exponer rutas.
- Tareas persistentes con crear, completar, reabrir y eliminar (borrado lógico), con estados `pending`, `in_progress` y `done`.
- Vista «Hoy» con los pendientes reales de la bóveda y vista «Tareas» con alta rápida y listas de abiertas/hechas, incluida la edición del estado y el borrado.
- Logo propio integrado en la barra de título, el favicon y todos los íconos de compilación generados con `tauri icon`; reemplaza el SVG provisional.
- Vistas provisionales solo para las secciones que siguen pendientes; cada una muestra su fase, alcance y un botón para volver a «Hoy».
- Modal «Checklist persistente» con el resumen del proyecto, accesible desde «Ver checklist».
- Barra de título propia con controles integrados de minimizar, maximizar/restaurar y cerrar, con estado sincronizado al redimensionar la ventana.
- Arrastre de la ventana desde la barra de título y bordes invisibles de redimensionado para la ventana sin decoración nativa.
- Permisos mínimos de ventana en `capabilities/default.json`: `minimize`, `toggle-maximize`, `close`, `start-dragging` y `start-resize-dragging`.

### Cambiado

- La app se renombró de `Omega` a `Study Planner`; los crates pasan a `study-planner-*`, el paquete a `study-planner` y el identificador a `app.studyplanner.desktop`.
- Al elegir la bóveda se crean automáticamente `<bóveda>/omega` y `<bóveda>/longlife/<dispositivo>`; la snapshot del día se crea al abrir la bóveda y se poda a 7 días.
- La sección «Bóveda» dejó de ser un placeholder: muestra el diagnóstico real, los respaldos y permite cambiar de carpeta. La sección «Tareas» pasó a ser funcional.
- `rusqlite` se compila con la feature `backup`; la bóveda y los backups viven exclusivamente en Rust.
- El magic de exportación pasa a `STUDY-PLANNER-EXPORT` / `STUDY-PLANNER-END`.
- La ventana principal usa `decorations: false`; la barra de título de la app reemplaza la decoración del sistema.
- La barra de título reserva una altura fija y agrupa metadatos y controles para mantener estable el layout.

### Conocido

- Dependabot reporta una debilidad media en `glib` 0.18.5 (iteradores de `VariantStrIter`). Llega de forma transitiva por `gtk 0.18` en la pila de Tauri; la corrección requiere `glib 0.20` y una actualización mayor de Tauri/gtk-rs. Study Planner no usa esos iteradores directamente. Ver `docs/CHECKLIST.md`.
- La restauración desde la UI, la exportación/importación canónica y los adjuntos ya están implementados; el modo de reemplazo total desde la UI sigue fuera de alcance.

### Seguridad

- La raíz de la bóveda se valida antes de escribir; se rechazan archivos, rutas inexistentes, la raíz del sistema y cualquier carpeta dentro del repositorio.
- Las carpetas de la bóveda se crean con permisos `0700` y los archivos con `0600` en Unix.
- Un backup no se considera válido solo por existir: se reabre y se verifican integridad, claves foráneas y checksum.
- La decisión MPL-2.0 quedó registrada en ADR-017: excepción acotada a dependencias transitivas de Tauri, sin modificar, conservando avisos.

## [0.1.0] — 2026-11-08 — Fase 0: base técnica

### Agregado

- Scaffold de Tauri 2 + React 19 + TypeScript + Vite 8.
- Workspace Rust con `study-planner-core`, `study-planner-store`, `study-planner-transfer`, `study-planner-updater` y `src-tauri`.
- Esquema SQLite v1 con entidades académicas, horarios, asistencia, exámenes, notas, tareas, temas, repaso, apuntes, adjuntos, notificaciones y auditoría.
- `Store` con WAL, claves foráneas, `synchronous=FULL`, migración con checksum SHA-256, `vault_id`, `integrity_check` y `foreign_key_check`.
- Comandos Tauri `get_app_info` y `schema_self_check` con base en memoria.
- Shell de UI con navegación, estado técnico, marca Study Planner e ícono local.
- CSP de producción y desarrollo, `freezePrototype` y `withGlobalTauri: false`.
- Capacidad mínima `core:default`; eliminación del plugin `opener`.
- Tests de UI con Vitest/Testing Library y tests de Rust para esquema y migraciones.
- Documentación persistente: checklist, arquitectura, modelo de datos, formato de exportación, updater, claves, decisiones, modo offline y licencias.
- Scripts `scripts/check.sh` y `scripts/audit-network.sh`.
- CI sin secretos en `.github/workflows/ci.yml`.
- Repositorio remoto configurado en [`wulflo/study-planner`](https://github.com/wulflo/study-planner); `main` con commit inicial.
- `.gitignore` endurecido para claves, bases, secretos y backups.

### Cambiado

- La app se renombró de `tauri-app` a `Study Planner`; identificador `app.studyplanner.desktop`.
- El gestor de paquetes es npm, con `package-lock.json` versionado.
- La persistencia queda exclusivamente en Rust; el frontend no tiene permisos de `fs`, `http`, `sql`, `shell` ni `opener`.

### Seguridad

- La webview no puede iniciar requests externos.
- El updater está desactivado (`UPDATER_IMPLEMENTED = false`) y no tiene dependencias de red.
- No se generaron claves ni secretos.
- La bóveda no se cifra por decisión explícita; la confidencialidad depende del disco del sistema.

### Conocido / pendiente

- Revisión de la licencia transitiva MPL-2.0 de Tauri (`option-ext` en runtime; `cssparser`, `selectors`, `dtoa-short` solo en build): resuelta como excepción acotada en ADR-017 (ver [No publicado]). Ver `THIRD_PARTY_LICENSES.md`.
- La exportación `.txt` está especificada pero se implementa en Fase 1.
- El updater se implementa en Fase 5.
- El repositorio es local y remoto: [`wulflo/study-planner`](https://github.com/wulflo/study-planner) es público; no hay releases todavía.
- La demo de Fase 0 y la confirmación del usuario quedaron cerradas el 2026-11-08 (ver [No publicado]).

## [0.0.0] — 2026-11-08

- Punto de partida del proyecto. Sin código previo.
