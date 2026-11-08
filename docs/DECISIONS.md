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
- Los repositorios y tests viven en `omega-store`.

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

## ADR-007 — Updater aislado, Linux AppImage

**Estado:** aceptada; implementación en Fase 5.

**Contexto:** se necesita actualizar sin abrir la app a la red y sin confiar en un manifiesto sin firma.

**Decisión:** crate `omega-updater` separado, sin bindings JS, URL HTTPS fija en compilación, firma obligatoria, opt-in, sin datos de usuario, solo Linux AppImage. Tauri updater no soporta Android/iOS.

**Consecuencias:**

- La clave privada y su passphrase son material crítico (ver `docs/KEYS.md`).
- Si Android fuera objetivo principal, habría que cambiar de stack (Kotlin + Compose + APK firmado).
- Windows/macOS no se actualizan en la primera versión.

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

**Estado:** aceptada con revisión pendiente.

**Contexto:** el usuario exige solo licencias permisivas (MIT/Apache/BSD), con excepción de plataforma WebKitGTK/GTK (LGPL).

**Decisión:** se aceptan MIT, Apache-2.0, BSD, ISC, Zlib y Unicode. Cualquier licencia copyleft requiere revisión y decisión registrada. Se detectó MPL-2.0 transitiva en Tauri: `option-ext` (runtime, vía `dirs`) y `cssparser`, `selectors`, `dtoa-short` (solo build, vía `dom_query`/`tauri-utils`). Está pendiente la decisión del usuario.

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
