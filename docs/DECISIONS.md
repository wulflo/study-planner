# Decisiones (ADR)

Registro breve de decisiones que afectan la arquitectura. Una decisión nueva se agrega al final; no se reescribe la historia. Si cambia una decisión, se crea otra ADR que la reemplaza.

---

## ADR-001 — Aplicación estrictamente local

**Estado:** aceptada.

**Contexto:** el usuario quiere una agenda académica sin cuentas, sin nube y sin telemetría.

**Decisión:** todos los datos viven en una bóveda SQLite local. No hay servidor, cuentas, sincronización ni analytics. La UI no tiene red.

**Consecuencias:**

- La copia de seguridad y la exportación son responsabilidad de la app.
- No hay colaboración ni acceso multi-dispositivo.
- La única excepción futura es el updater aislado (ADR-007).

---

## ADR-002 — Tauri 2 + React + TypeScript + Vite, con npm

**Estado:** aceptada.

**Contexto:** se necesita una app de escritorio Linux, con UI moderna y binario razonablemente pequeño. `pnpm` no está instalado en el entorno.

**Decisión:** Tauri 2, React 19, TypeScript, Vite 8, Zustand, date-fns, Material Color Utilities e i18next. Gestor: **npm** con `package-lock.json`.

**Consecuencias:**

- Un solo lenguaje de UI y un backend Rust.
- WebKitGTK como dependencia de plataforma (LGPL).
- El lockfile debe commitearse; no se actualiza a ciegas.

---

## ADR-003 — SQLite con `rusqlite` en Rust, sin plugin SQL

**Estado:** aceptada.

**Contexto:** el frontend no debe tener acceso a archivos ni SQL. Se necesita control transaccional, migraciones y backups.

**Decisión:** `rusqlite` con SQLite bundled, WAL, `synchronous=FULL`, claves foráneas activas, migraciones versionadas con checksum. No se usa `tauri-plugin-sql` ni se expone SQL al frontend.

**Consecuencias:**

- Toda la persistencia pasa por comandos Rust.
- El binario incluye SQLite y no depende de la versión del sistema.
- Los repositorios y tests viven en `study-planner-store`.

---

## ADR-004 — Bóveda sin cifrado

**Estado:** aceptada explícitamente por el usuario.

**Contexto:** se evaluó cifrar la bóveda con contraseña, Argon2id y AES-GCM.

**Decisión:** **no** se cifra la bóveda. No hay contraseña maestra, recovery key ni cifrado a nivel de aplicación.

**Consecuencias:**

- La confidencialidad depende del cifrado del disco del sistema y de la custodia física.
- La exportación `.txt` contiene todos los datos sin cifrar; la UI debe advertirlo.
- No reintroducir cifrado de bóveda sin una nueva decisión explícita.

---

## ADR-005 — Exportación `.txt` canónica y total

**Estado:** aceptada; contrato en `docs/EXPORT_FORMAT.md`.

**Contexto:** el usuario quiere poder reconstruir todo sin depender de la app y detectar corrupción.

**Decisión:** un archivo de texto con cabecera, cuerpo JSONL canónico, checksum SHA-256 y terminador. El round-trip `exportar → importar → exportar` debe ser byte a byte idéntico en la misma versión. `snapshot_at` se deriva del máximo `updated_at` para que sea reproducible.

**Consecuencias:**

- El serializador debe ser determinista (claves ordenadas, sin floats, LF, UTF-8).
- Los adjuntos se embeben en v1 para que la exportación sea total.
- Cambios de formato suben `export_format_version`.

---

## ADR-006 — Permisos mínimos y CSP estricta

**Estado:** aceptada.

**Contexto:** la webview es la superficie de ataque más expuesta.

**Decisión:** `capabilities/default.json` tiene `core:default` más los permisos explícitos de ventana para la barra de título propia (`minimize`, `toggle-maximize`, `close`, `start-dragging`, `start-resize-dragging`). Sin `fs`, `http`, `sql`, `shell`, `opener`, `dialog`, `notification` ni `updater`. CSP de producción sin `unsafe-eval`, sin CDNs, `connect-src` solo IPC. `withGlobalTauri: false` y `freezePrototype: true`.

**Consecuencias:**

- Cualquier función nueva que necesite archivos o red se implementa como comando Rust.
- Un permiso nuevo requiere justificación y auditoría.
- Los estilos inline siguen permitidos por React/M3; no habilitan recursos remotos.

---

## ADR-007 — Updater aislado, multiplataforma

**Estado:** aceptada; implementada en Fase 5 y ampliada a `.deb`/`.rpm`/Windows/macOS en 0.1.3.

**Contexto:** se necesita actualizar sin abrir la app a la red y sin confiar en un manifiesto sin firma.

**Decisión:** crate `study-planner-updater` separado, sin bindings JS, URL HTTPS fija en compilación, firma obligatoria, opt-in, sin datos de usuario. El updater detecta AppImage, `.deb`, `.rpm`, Windows (NSIS/MSI) y macOS (`.dmg`); Snap queda gestionado por snapd. Tauri updater no soporta Android/iOS.

**Consecuencias:**

- La clave privada y su passphrase son material crítico (ver `docs/KEYS.md`).
- Si Android fuera objetivo principal, habría que cambiar de stack (Kotlin + Compose + APK firmado).
- En Linux, `.deb`/`.rpm` requieren `pkexec` y un agente polkit; Windows/MSI puede pedir UAC y macOS puede pedir confirmación de Gatekeeper.

---

## ADR-008 — Adjuntos embebidos por defecto

**Estado:** aceptada; v1 siempre embebe.

**Contexto:** una exportación que solo referencia rutas no es portable.

**Decisión:** v1 incluye todos los blobs en `attachment_blob`, con SHA-256 verificado. La exportación es siempre total y portable. Un futuro modo por referencia subirá la versión y dejará de garantizar la reconstrucción.

**Consecuencias:**

- Los archivos exportados pueden ser grandes.
- El importador debe verificar hash y tamaño antes de escribir.
- El almacenamiento físico en la bóveda es por hash (`attachments/<sha256>`).

---

## ADR-009 — Punto fijo entero para notas y pesos

**Estado:** aceptada.

**Contexto:** los flotantes producen diferencias de redondeo y rompen el round-trip.

**Decisión:** notas en centipuntos (`8500` = 85.00), pesos en basis points (`10000` = 100 %). Nada de `REAL` para valores académicos.

**Consecuencias:**

- La UI formatea; la base guarda enteros.
- Los cálculos usan enteros o decimales explícitos, no `f64` para almacenar.
- Los tests comparan valores exactos.

---

## ADR-010 — Borrado lógico

**Estado:** aceptada.

**Contexto:** se necesita deshacer, conservar historial y evitar cascadas destructivas.

**Decisión:** las entidades de dominio usan `deleted_at`; no hay `DELETE` físico. Los índices únicos son parciales (`WHERE deleted_at IS NULL`). Las claves foráneas usan `ON DELETE RESTRICT`.

**Consecuencias:**

- La exportación incluye borrados lógicos.
- Las consultas deben filtrar `deleted_at IS NULL` salvo en vistas de historial.
- El crecimiento de la base es aceptable para uso personal.

---

## ADR-011 — Dependencias permisivas, con excepciones revisadas

**Estado:** aceptada; la revisión pendiente de MPL-2.0 quedó resuelta por ADR-017 (2026-11-08).

**Contexto:** el usuario exige solo licencias permisivas (MIT/Apache/BSD), con excepción de plataforma WebKitGTK/GTK (LGPL).

**Decisión:** se aceptan MIT, Apache-2.0, BSD, ISC, Zlib y Unicode. Cualquier licencia copyleft requiere revisión y decisión registrada. Se detectó MPL-2.0 transitiva en Tauri: `option-ext` (runtime, vía `dirs`) y `cssparser`, `selectors`, `dtoa-short` (solo build, vía `dom_query`/`tauri-utils`). La decisión del usuario quedó registrada en ADR-017.

**Consecuencias:**

- `THIRD_PARTY_LICENSES.md` documenta el inventario y las obligaciones.
- No se copia código GPL/AGPL.
- Si no se acepta MPL, hay que evaluar alternativas o asumir el costo de parchear Tauri.

---

## ADR-012 — Móvil fuera de alcance inicial

**Estado:** aceptada.

**Contexto:** Tauri updater no soporta Android/iOS y el objetivo es una app de escritorio local.

**Decisión:** Linux desktop primero. Android/iOS no se prometen. Si Android se volviera principal, se replantearía el stack (Kotlin + Compose + APK firmado).

**Consecuencias:**

- No se invierte en adaptaciones móviles.
- La UI es responsive, pero no se prueba en móvil.

---

## ADR-013 — Repositorio local, sin remoto por ahora

**Estado:** reemplazada por ADR-014 (2026-11-08).

**Contexto:** el usuario todavía no creó el repositorio en GitHub.

**Decisión:** el proyecto se inicializa como repositorio git local. No se crea remoto, no se configura `origin` y no se hace push. El CI queda versionado pero no se ejecuta hasta que exista un remoto.

**Consecuencias:**

- Los commits son locales.
- La URL del updater y los secrets de GitHub quedan pendientes.
- Cuando el usuario decida publicar, se revisa `.gitignore`, se audita el historial y recién entonces se agrega el remoto.

---

## ADR-014 — Remoto público en GitHub

**Estado:** aceptada (2026-11-08). Reemplaza ADR-013.

**Contexto:** el usuario creó el repositorio público `https://github.com/wulflo/study-planner` y pidió configurarlo.

**Decisión:** `main` se publica en GitHub como repositorio público. El código y la documentación son públicos; la bóveda, los backups, las claves y los secretos **nunca** se publican. El CI se ejecuta en GitHub Actions sin secretos. La URL del updater queda tentativamente en ese repositorio, pero se fija recién en Fase 5.

**Consecuencias:**

- Cualquiera puede leer el código y reportar problemas.
- El historial debe auditarse antes de cada push para no filtrar datos.
- Los secretos de firma se configuran en Fase 5, nunca en el YAML.
- Si el repositorio cambia de nombre o de visibilidad, hay que actualizar la URL fija del updater y este documento.

---

## ADR-015 — Ventana sin decoración nativa con controles propios

**Estado:** aceptada (2026-11-08).

**Contexto:** el usuario pidió que minimizar, maximizar y cerrar estén integrados en la interfaz, en lugar de depender de la decoración del sistema.

**Decisión:** la ventana principal usa `decorations: false` y la app dibuja su propia barra de título con controles de minimizar, maximizar/restaurar y cerrar. La barra es una región de arrastre de Tauri; los controles y los elementos interactivos quedan excluidos. Como Linux sin decoración pierde el borde nativo de redimensionado, se agregan ocho bordes invisibles que delegan en `startResizeDragging`. Los permisos se limitan a las cinco acciones de ventana necesarias.

**Consecuencias:**

- La UI debe reservar una altura fija para la barra de título y respetar el área de arrastre.
- El doble clic en la barra maximiza/restaura mediante Tauri; no se agrega un manejador propio para evitar un doble toggle.
- Cualquier elemento interactivo nuevo en la barra debe ser un control real (botón, enlace) para no romper el arrastre.
- El redimensionado depende de los bordes propios; si se elimina un borde, esa dirección deja de funcionar.
- En Linux, GTK puede emitir el aviso `gtk_window_set_titlebar() called on a realized window` al iniciar; es inocuo y no afecta el render.
- No se agregan permisos de archivos, SQL, shell ni red: solo acciones de ventana.

---

## ADR-016 — Nombre Study Planner y estructura de bóveda `omega/` + `longlife/`

**Estado:** aceptada (2026-11-09).

**Contexto:** el proyecto se llamaba internamente `Omega`, pero ese nombre corresponde a una subcarpeta de datos, no a la aplicación. El usuario confirmó que la app es **Study Planner** y que la bóveda sigue el patrón ya usado en Better Tracker: `<bóveda>/omega` para los datos vivos y el backup principal, y `<bóveda>/longlife` para las snapshots de resguardo.

**Decisión:**

- El nombre visible y de producto es **Study Planner**. Los crates pasan a `study-planner-core`, `study-planner-store`, `study-planner-transfer` y `study-planner-updater`; el paquete npm y el binario son `study-planner`; el identifier es `app.studyplanner.desktop`.
- El magic de exportación pasa a `STUDY-PLANNER-EXPORT` / `STUDY-PLANNER-END`.
- El usuario elige **una raíz de bóveda** en el primer arranque. La app crea automáticamente:
  - `<bóveda>/omega/` → base viva, adjuntos y backup principal.
  - `<bóveda>/longlife/<dispositivo>/<AAAA-MM-DD>/` → snapshots diarias.
- La retención inicial es de 7 snapshots por dispositivo.
- Los nombres van en minúsculas para evitar duplicados en bóvedas sincronizadas entre sistemas que distinguen mayúsculas.
- La raíz de la bóveda es una setting de máquina: no se exporta y nunca vive dentro del repositorio.

**Consecuencias:**

- Los documentos, comandos y pruebas deben hablar de `<bóveda>/omega` y `<bóveda>/longlife`, no de rutas absolutas `/omega` o `/loglife`.
- `omega` sigue apareciendo en el código y en la UI solo como nombre de la subcarpeta de datos.
- Cambiar la estructura o la retención requiere una nueva decisión y una migración de la bóveda, no un cambio silencioso.
- El identificador `app.studyplanner.desktop` queda fijo; cambiarlo después obliga a migrar datos de `app_data_dir`.

---

## ADR-017 — MPL-2.0 transitiva aceptada como excepción revisada

**Estado:** aceptada (2026-11-08). Resuelve la revisión pendiente de ADR-011.

**Contexto:** Tauri incorpora paquetes MPL-2.0 de forma transitiva: `option-ext` en runtime (a través de `dirs` → `tauri`/`wry`) y `cssparser`, `selectors` y `dtoa-short` solo en build (a través de `dom_query`/`tauri-utils`). El usuario ya revisó la situación y pidió continuar sin bloquear el proyecto por esa dependencia. MPL-2.0 es copyleft a nivel de archivo y es compatible con distribuir un programa mayor bajo MIT/Apache-2.0.

**Decisión:**

- Se acepta MPL-2.0 **solo** para los paquetes transitivos listados, sin modificarlos.
- Se conservan sus avisos de copyright y licencia en `THIRD_PARTY_LICENSES.md` y en la distribución.
- Si en el futuro se parchea un archivo MPL, ese archivo se publica bajo MPL-2.0 y la decisión se registra.
- Cualquier nueva dependencia MPL, GPL o AGPL sigue requiriendo revisión explícita; esta ADR no es una aceptación general de copyleft.
- No se agrega una dependencia MPL directa ni se cambia el stack para evitar la transitiva de Tauri.

**Consecuencias:**

- No hay que parchear Tauri ni asumir el costo de mantenimiento de un fork.
- El inventario de licencias debe regenerarse antes de una release y confirmar que los paquetes MPL siguen sin modificaciones.
- La licencia del proyecto sigue siendo `MIT OR Apache-2.0`.
- `THIRD_PARTY_LICENSES.md` deja de tener la decisión como pendiente y pasa a documentar la obligación de conservar avisos.

---

## ADR-018 — Importación solo en bóveda vacía y restauración con backup previo

**Estado:** aceptada (2026-11-08).

**Contexto:** la Fase 1 necesita recuperar una bóveda completa desde un `.txt` canónico y restaurar el backup principal sin riesgo de mezclar datos ni perder el estado actual. El núcleo de transferencia ya soporta un modo `Replace`, pero habilitarlo desde la UI exige definir confirmaciones, backups y una política de adjuntos.

**Decisión:**

- La importación normal desde la UI usa `ImportMode::EmptyOnly`: se rechaza cualquier bóveda con datos de dominio y se informa que hay que elegir una carpeta vacía.
- El modo `Replace` queda implementado solo en `study-planner-transfer` como base para una fase futura; la UI no lo expone.
- La restauración del backup principal exige confirmación explícita en la pantalla «Bóveda». Antes de reemplazar la base viva se crea un backup verificado `pre-restore-<timestamp>.db`.
- La restauración verifica el backup principal (esquema, `integrity_check`, `foreign_key_check`) antes de tocar la base; si no pasa, no se restaura.
- El lock de la bóveda se conserva durante la restauración y la conexión SQLite se reabre al terminar.
- Cada operación y cada error se registra en `<omega>/study-planner.log` con timestamp UTC y sin rutas absolutas.

**Consecuencias:**

- No se puede importar sobre una bóveda con datos desde la UI; para hacerlo hay que abrir una carpeta nueva.
- La restauración siempre deja dos backups: el principal restaurado y el `pre-restore-*` del estado anterior.
- Habilitar el reemplazo total requiere una decisión nueva con confirmación de doble paso y pruebas de adjuntos huérfanos.
- El log local crece sin rotación automática en Fase 1; la rotación se evaluará junto con el diagnóstico de Fase 5.

---

## ADR-019 — Notificaciones locales sin plugin

**Estado:** aceptada; implementada en Fase 5.

**Contexto:** hacen falta recordatorios de clases, tareas, exámenes y repasos sin ampliar la superficie de la webview ni agregar red. El plugin oficial de notificaciones exigiría permisos nuevos y una dependencia más de Tauri.

**Decisión:**

- El cálculo de recordatorios vive en `study-planner-store` (`due_reminders`), es determinista para una hora local y usa los ajustes portables del esquema v1.
- La deduplicación usa la tabla `notification` con `dedupe_key` única: se reclama, se envía y se marca `fired`; si la entrega falla, se libera el reclamo.
- La entrega se hace por D-Bus (`org.freedesktop.Notifications`) desde el host Rust con `zbus`, sin plugin ni permiso `notification:*`.
- Los ajustes de anticipación se siembran en la migración inicial; no hay migración nueva.
- La UI solo invoca comandos sin argumentos de red y muestra los últimos avisos entregados.

**Consecuencias:**

- Dependencias nuevas: `zbus` (D-Bus) y `chrono` (fechas locales), sin capacidades de red.
- En escritorios sin bus de sesión la entrega falla de forma controlada y se reintenta.
- Los avisos no se entregan con la app cerrada; no hay servicio del sistema en esta fase.
- La tabla conserva los avisos entregados sin purga automática por ahora.

## ADR-020 — MSIX para Microsoft Store sin target nativo

**Estado:** aceptada; implementada.

**Contexto:** Microsoft Store distribuye apps de escritorio como MSIX. Tauri 2.12 solo genera NSIS y MSI en Windows; no existe un bundle MSIX nativo.

**Decisión:**

- El `.msix` se arma en un workflow independiente, `.github/workflows/store-msix.yml`, con `scripts/prepare-store-msix.mjs` y `MakeAppx.exe` del Windows SDK en `windows-latest`. El workflow de release no lo toca.
- El manifiesto sale de `packaging/windows-store/AppxManifest.template.xml`, usa el identificador de la app, `runFullTrust`, `Windows.FullTrustApplication` y los íconos de tienda de `src-tauri/icons`. No declara `uap:DefaultTile`, porque MakeAppx exige `Wide310x150Logo` cuando aparece `Square310x310Logo`.
- La firma Authenticode no se hace localmente: el `.msix` se sube sin firmar al Partner Center y Microsoft Store aplica la identidad y firma de la cuenta al publicar. No se usan certificados ni secretos de firma.
- `PRIVACY.md` es obligatoria y el workflow `store-msix.yml` falla si no existe.
- La app detecta `WindowsApps` en su ruta, reporta `runtime: "msix"` y oculta el updater: la Store gestiona las actualizaciones.

**Consecuencias:**

- El MSIX no participa del manifiesto `latest.json` ni del updater propio, ni de la release de GitHub: queda como artefacto `store-msix` del workflow manual.
- La publicación en la Store es manual: se descarga el `.msix` del run y se sube al Partner Center.
- El paquete no se puede instalar por sideloading sin un certificado de confianza; ese no es el flujo soportado.
- El workflow de la Store depende del Windows SDK preinstalado en el runner.

## ADR-021 — Instancia única con `tauri-plugin-single-instance`

**Estado:** aceptada; implementada.

**Contexto:** abrir la app desde el menú con una ventana ya oculta en la bandeja iniciaba un segundo proceso en lugar de mostrar el existente. El lock de la bóveda evitaba compartir la base, pero no resolvía la experiencia de escritorio.

**Decisión:**

- Se registra `tauri-plugin-single-instance` como primer plugin; el callback llama a `background::show_main_window` (mostrar, desminimizar y enfocar).
- En Linux el plugin usa un nombre de sesión D-Bus; en Windows y macOS usa los mecanismos nativos.
- En AppImage, `.deb`, `.rpm`, Windows y macOS funciona sin permisos adicionales.
- Bajo snap `strict` el nombre D-Bus requiere un slot privilegiado y revisión manual de la Store; no se declara y el plugin degrada a «arrancar normalmente». Para ese caso, `src-tauri/src/instance.rs` mantiene un respaldo en Unix: `flock` sobre `instance.lock` en el directorio de datos y un socket local `instance.sock`. La segunda instancia se cierra y le pide a la primera que muestre su ventana.
- El icono de bandeja se crea una sola vez (`TRAY_ID`): si ya existe, `setup_tray` no agrega otro.

**Consecuencias:**

- Una segunda instancia no llega a ejecutar `setup` ni bloquea la bóveda.
- En el snap, la segunda apertura ya no crea otra ventana ni otro icono; enfocar la instancia viva depende del socket local, no de un slot `dbus`.
- En Windows la detección queda a cargo del mutex con nombre del plugin, que no requiere configuración adicional.

