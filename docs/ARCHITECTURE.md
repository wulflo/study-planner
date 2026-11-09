# Arquitectura

Study Planner es una aplicación de escritorio Tauri 2 con un frontend React y un núcleo Rust que posee la base de datos y el sistema de archivos. La seguridad se apoya en tres ideas: **la webview no toca disco ni red**, **el dominio es puro y testeable** y **la red está aislada en un único crate opcional**.

## Capas

```text
┌─────────────────────────────────────────────────────────┐
│ src/  (React + TypeScript)                              │
│  · vistas, estado de UI, i18n                           │
│  · src/lib/ipc.ts: contrato tipado                      │
└───────────────────────────┬─────────────────────────────┘
                            │ invoke() — comandos propios
┌───────────────────────────▼─────────────────────────────┐
│ src-tauri/  (host Tauri)                                │
│  · comandos, validación de entrada                      │
│  · ventana, CSP, capabilities                           │
│  · composición de servicios y rutas de la bóveda        │
└─────────────┬───────────────────────────┬───────────────┘
              │                           │
┌─────────────▼───────────────┐ ┌─────────▼───────────────┐
│ study-planner-store         │ │ study-planner-transfer  │
│  · SQLite, WAL, migraciones │ │  · export/import .txt   │
│  · backups y snapshots      │ │  · checksum, round-trip │
└─────────────┬───────────────┘ └─────────┬───────────────┘
              │                           │
┌─────────────▼───────────────────────────▼───────────────┐
│ study-planner-core  (dominio puro)                      │
│  · constantes, IDs, reglas de negocio                   │
│  · sin Tauri, sin disco, sin red                        │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ study-planner-updater  (Fase 5, aislado)                │
│  · única dependencia de red del proyecto                │
│  · sin bindings JS, URL fija, firma obligatoria         │
└─────────────────────────────────────────────────────────┘
```

### `study-planner-core`

No depende de Tauri, SQLite, red ni sistema de archivos. Contiene:

- Constantes estables: `APP_NAME`, `SCHEMA_VERSION`, `EXPORT_FORMAT_VERSION`.
- `AppInfo`, generación de UUIDv7.
- En fases siguientes: cálculo de recurrencias, prioridad de estudio, SM-2, agregaciones de notas y validaciones puras.

Regla: si una función se puede testear sin base de datos ni reloj real, va acá.

### `study-planner-store`

Dueño exclusivo de la conexión SQLite. Responsabilidades:

- Abrir la base con `foreign_keys=ON`, `synchronous=FULL`, `busy_timeout` y WAL.
- Aplicar migraciones hacia adelante con checksum SHA-256 y `PRAGMA user_version`.
- Rechazar bases con esquema más nuevo que el soportado.
- Exponer repositorios tipados (Fase 1+) y transacciones.
- Ejecutar backups online y verificarlos.
- Nunca entrega la conexión al frontend.

La conexión se crea en Rust y se guarda en el estado de Tauri (`State<...>`), protegida para acceso serializado o con un pool controlado. El frontend solo invoca comandos de alto nivel.

### `study-planner-transfer`

Codifica y decodifica el `.txt` canónico. No conoce Tauri ni la UI. El contrato está en `docs/EXPORT_FORMAT.md`.

- Exportar: lee una foto consistente de la base, ordena, serializa y calcula checksum.
- Importar: valida magia, versión, checksum, integridad referencial y versión de esquema; escribe en una transacción.
- Los tests de round-trip viven acá y usan fixtures versionados.

### `study-planner-updater`

Crate separado para que la red no se filtre al resto. Contiene el manifiesto, la validación de URL/host, SHA-256, la verificación minisign y el cliente HTTP bloqueante con rustls. Es la única dependencia de red del proyecto. Detecta el runtime (AppImage, `.deb`, `.rpm`, Windows, MSIX, macOS, Snap) y bloquea la instalación en Snap y MSIX, donde la actualización la gestiona la tienda. Diseño y flujo en `docs/UPDATER.md`; publicación en `docs/RELEASE.md`.

### Notificaciones locales

No hay crate ni plugin aparte. El cálculo determinista vive en `study-planner-store` y la entrega por D-Bus (`org.freedesktop.Notifications`) en `src-tauri/src/notifications.rs`. No se concede ningún permiso de notificación a la webview. Detalle en `docs/NOTIFICATIONS.md`.

### `src-tauri`

Host y composición:

- Registra comandos explícitos en `invoke_handler`.
- Valida parámetros y traduce errores de dominio a errores IPC.
- Configura CSP, ventana, bundle y capacidades.
- Usa `tauri-plugin-single-instance` para enfocar la ventana existente cuando se vuelve a abrir la app; el callback llama a `background::show_main_window`. En Unix, `instance` agrega un respaldo con lock de archivo y socket local para los entornos donde el plugin no puede publicar su nombre D-Bus.
- Abre el repositorio con `repository::open_repository`, que lanza `xdg-open`/`open`/`rundll32` sobre una URL fija definida en Rust. La webview nunca recibe una URL configurable.
- No expone plugins de `fs`, `http`, `sql`, `shell`, `opener`, `dialog`, `notification` ni `updater` al frontend.

## Estructura de la bóveda

El usuario elige **una sola carpeta raíz** (la bóveda) en el primer arranque. La app crea y mantiene el resto automáticamente; ninguna ruta cambia en silencio.

```text
<bóveda>/
├── omega/                              datos vivos y backup principal
│   ├── study-planner.db                base viva (WAL)
│   ├── study-planner.db-wal            journal de WAL
│   ├── study-planner.db-shm            memoria compartida de WAL
│   ├── .study-planner.lock             lock de instancia (se libera al cerrar)
│   ├── attachments/                    adjuntos, nombrados por hash
│   └── backup/
│       ├── study-planner.db            backup principal verificado
│       └── pre-migration-vN-*.db       backup previo a una migración
└── longlife/                           snapshots de resguardo
    └── <dispositivo>/
        ├── 2026-11-08/
        │   ├── study-planner.db        copia íntegra del día
        │   ├── attachments/            adjuntos referenciados
        │   └── snapshot.json           metadatos y checksums
        └── 2026-11-07/
```

Reglas:

- `omega/` es la **única fuente de verdad** para la base viva, su backup principal, los adjuntos y el log local.
- `longlife/<dispositivo>/<AAAA-MM-DD>` guarda **snapshots diarias**; se conservan las últimas 7 por dispositivo.
- `<dispositivo>` es el hostname local sanitizado (alfanumérico, `-` y `_`, hasta 64 caracteres; `equipo` si queda vacío). Identifica la carpeta de snapshots de esta máquina.
- Los nombres van en minúsculas (`omega`, `longlife`) para que una bóveda sincronizada entre sistemas que distinguen mayúsculas no se duplique.
- En Unix las carpetas se crean con permisos `0o700` y los archivos con `0o600`; si el sistema de archivos no soporta POSIX (exFAT/NTFS), se registra el aviso y se continúa.
- La raíz de la bóveda es una setting **de máquina**: no se exporta y nunca se escribe dentro del repositorio.
- Un snapshot solo se considera completo cuando existe `snapshot.json` con los checksums; si falta, se descarta.
- La restauración nunca sobrescribe en silencio: verifica el backup, crea `backup/pre-restore-*.db` y pide confirmación en la UI. La importación es transaccional y solo se acepta en bóveda vacía.
- `<omega>/attachments/` guarda los adjuntos por SHA-256; se exportan embebidos y se verifican al importar.
- `<omega>/study-planner.log` registra eventos de backup, exportación, importación y restauración con timestamp UTC, sin rutas absolutas.

## Flujo de arranque (implementado en Fase 1)

```text
inicio
  → leer la raíz de la bóveda desde vault.json (config de la app)
  → validar la ruta (existe, carpeta, escribible, no raíz, fuera del repo)
  → crear <bóveda>/omega, <bóveda>/omega/backup, <bóveda>/omega/attachments y <bóveda>/longlife/<dispositivo>
  → adquirir <omega>/.study-planner.lock (falla si hay otra instancia)
  → abrir <omega>/study-planner.db
  → leer schema_version
  → si hay migración pendiente:
        backup verificado pre-migration-vN-*.db
        migrar en transacción
        si falla: restaurar el backup y abortar
  → verificar o crear el backup principal
  → crear la snapshot diaria si falta, está incompleta o tiene otro esquema
  → podar snapshots a las últimas 7
  → exponer VaultStatus a la UI
  → mostrar app
```

Un fallo al cargar la bóveda no bloquea la app: la UI muestra el error y permite elegir otra carpeta. Ningún paso cambia rutas ni sobrescribe datos sin confirmación explícita.

La restauración manual está implementada: verifica el backup principal, crea `backup/pre-restore-*.db` con el estado actual, reemplaza la base viva, reabre SQLite y conserva el lock. La restauración automática sigue ocurriendo solo si falla una migración.

## Contrato IPC

Reglas:

- Nombres en `snake_case` en Rust, `camelCase` en TypeScript (serde `rename_all`).
- Entradas y salidas serializables, sin handles, rutas ni conexiones.
- Errores como `Result<T, String>` en Fase 0; en Fase 1 se evalúa un enum de error estable.
- El frontend no puede pedir «abrí este archivo» ni «ejecutá este SQL».
- Toda operación destructiva recibe confirmación desde la UI y vuelve a validar en Rust.

Comandos actuales:

| Comando | Entrada | Salida |
|---|---|---|
| `get_app_info` | — | `AppInfo` |
| `schema_self_check` | — | esquema, integridad, FK, `vault_id` de una base en memoria |
| `get_vault_status` | — | estado de la bóveda (sin rutas), incluido el backup y las snapshots |
| `choose_vault` | — | abre el selector nativo; `VaultStatus` o `null` si se cancela |
| `create_backup` | — | crea y verifica el backup principal; `VaultStatus` actualizado |
| `export_vault` | — | abre el selector nativo de guardado; `ExportInfo` o `null` si se cancela |
| `import_vault` | — | abre el selector nativo; importa solo en bóveda vacía; `ImportOutcome` o `null` |
| `restore_main_backup` | — | crea backup previo, restaura el principal verificado y reabre la bóveda |
| `list_tasks` | — | `Task[]` de la bóveda abierta |
| `create_task` | `title` | `Task` pendiente |
| `set_task_done` | `id`, `done` | `Task` actualizada |
| `delete_task` | `id` | borrado lógico (`deleted_at`) |

`src/lib/ipc.ts` es la única puerta del frontend hacia Rust. `tauri-plugin-dialog` se usa solo desde Rust (`DialogExt`): la webview no recibe la ruta elegida ni tiene permiso `dialog`.

## Seguridad

### CSP

Producción:

```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data: blob:; font-src 'self';
connect-src ipc: http://ipc.localhost; media-src 'none'; object-src 'none';
frame-src 'none'; frame-ancestors 'none'; worker-src 'self' blob:;
base-uri 'none'; form-action 'none'
```

- Sin `unsafe-eval`, sin CDNs, sin fuentes ni imágenes remotas.
- `style-src 'unsafe-inline'` es necesario por los estilos dinámicos de React/M3; no permite cargar CSS remoto.
- `connect-src` solo admite el IPC de Tauri.
- En desarrollo se agrega `unsafe-eval` y el WebSocket de Vite; nunca en producción.

### Capacidades

`src-tauri/capabilities/default.json` contiene `core:default` más los permisos explícitos de ventana que necesita la barra de título propia: `allow-minimize`, `allow-toggle-maximize`, `allow-close`, `allow-start-dragging` y `allow-start-resize-dragging`. No hay permisos de archivos, SQL, shell ni red. Cualquier permiso nuevo debe justificarse, documentarse y auditarse con `scripts/audit-network.sh`.

### Webview

- `withGlobalTauri: false`: no se expone `window.__TAURI__` global.
- `freezePrototype: true`: mitiga prototipos contaminados.
- Sin `dangerousDisableAssetCspModification`, sin `allowlist` amplia, sin `devtools` en release.

### Archivos

- Solo Rust abre, lee, escribe y borra.
- Las rutas se guardan como settings `machine`, no se exportan.
- Los backups se escriben como `.partial`, se verifican con `integrity_check`, `foreign_key_check` y SHA-256, y solo entonces reemplazan al backup anterior mediante renombre atómico.
- Los adjuntos se validan por hash y tamaño antes de servirlos a la UI.

### Red

- La webview no puede iniciar requests.
- El updater es la única excepción, en un crate separado, sin JS, con URL fija compilada y firma obligatoria.
- Cada intento de red del updater se registra en `network_event`.

## Datos y consistencia

- SQLite con WAL para lecturas concurrentes y escrituras serializadas.
- `synchronous=FULL` para durabilidad; el costo es aceptable para uso personal.
- Migraciones dentro de una transacción; backup previo en disco.
- Borrado lógico con `deleted_at`; los índices únicos son parciales (`WHERE deleted_at IS NULL`).
- Fechas civiles (`YYYY-MM-DD`) y horas locales (`HH:MM`) se guardan como texto; los instantes van en UTC RFC3339 con milisegundos.
- Dinero/notas/pesos: enteros de punto fijo. Nada de `REAL` para valores académicos.
- FTS5 es derivado: se reconstruye al importar y no se exporta.

## Build y distribución

- `npm run build` compila el frontend a `dist/`.
- `npm run tauri build` produce el bundle; en Linux el objetivo del updater es AppImage.
- El binario incluye SQLite (feature `bundled`), así que no depende de la versión del sistema.
- WebKitGTK/GTK se enlazan dinámicamente y son la excepción LGPL de plataforma.

## No objetivos

- Cifrado de bóveda.
- Cuentas, sincronización, nube, telemetría.
- Servidor local o API remota.
- Android/iOS en la primera versión.
- Plugins de Tauri que amplíen permisos del frontend.

## Riesgos conocidos

| Riesgo | Mitigación |
|---|---|
| Pérdida de datos por migración | Backup verificado + transacción + restauración automática |
| Corrupción silenciosa | `integrity_check`, checksums, backups rotativos |
| Fuga por red | CSP, permisos mínimos, updater aislado, auditoría estática |
| Dependencia no permisiva | Revisión de licencias; MPL-2.0 transitiva aceptada como excepción acotada en ADR-017 |
| Bóveda abierta en dos instancias | Lock de archivo en `omega/.study-planner.lock`; la segunda instancia recibe un error explícito |
| Ruta de bóveda peligrosa | Validación de existencia, escritura, raíz del sistema y ubicación dentro del repositorio |
| Clave de firma comprometida | Custodia offline, passphrase separada, rotación documentada |
| Cambio accidental de rutas | Settings `machine`, confirmación explícita, nunca silencioso |
