# Study Planner — Checklist persistente

> **Última actualización:** 2026-11-08
> **Fase actual:** 5 — Updater, notificaciones y empaquetado (en curso)
> **Versión:** 0.1.0 · `schema_version = 1` · `export_format_version = 1`

Este archivo es la fuente de verdad para retomar el proyecto después de días o semanas sin internet. Si algo no coincide con el código, gana el código y hay que corregir este documento en el mismo commit.

---

## Cómo usar este documento

1. Leer **Estado actual**, **Reglas para no romper** y la fase abierta.
2. Ejecutar `./scripts/check.sh` antes de cambiar nada. Si falla, primero reparar la base.
3. Trabajar solo en los ítems de la fase actual. No adelantar fases.
4. Marcar `[x]` únicamente cuando el ítem tenga test o verificación reproducible.
5. Cerrar cada fase con: demo, tests verdes, `CHANGELOG.md` actualizado y confirmación del usuario.
6. Si se tomó una decisión nueva, registrarla en `docs/DECISIONS.md` y reflejarla acá.
7. No borrar ítems completados: sirven de historial.

---

## Estado actual (snapshot)

| Campo | Valor |
|---|---|
| Fase | 5 — Updater, notificaciones y empaquetado (en curso) |
| Versión de app | 0.1.0 |
| Esquema SQLite | v1 |
| Formato de exportación | v1 implementado y verificado con round-trip byte a byte |
| Frontend | React 19 + TypeScript + Vite 8 |
| Backend | Rust + Tauri 2.12 + `rusqlite` 0.41 (bundled, backup) |
| Red en la webview | Prohibida por CSP y permisos |
| Updater | Clave pública de producción configurada en `study-planner-updater`; falta publicar una release firmada |
| Notificaciones | Locales, calculadas en el store y entregadas por D-Bus desde Rust; sin plugin ni permisos de webview |
| Bóveda real | Selección local, lock, backups verificados, snapshots, exportación/importación canónica, restauración con backup previo y log local |
| Secretos | Ninguno generado, ninguno en el repo |
| Git | Repositorio local + remoto GitHub [`wulflo/study-planner`](https://github.com/wulflo/study-planner) (público); `main` con commit inicial `87f2be8` |
| CI | GitHub Actions con los crates actuales (core, store, transfer, updater y host), auditoría de red y `release.yml` para tags `v*` |
| Release | AppImage empaquetado y verificado localmente; publicación firmada pendiente de la clave de producción |

### Verificación vigente

```text
npm run check       ✅  34 tests de UI
cargo test -p study-planner-core -p study-planner-store -p study-planner-transfer -p study-planner-updater
                    ✅  117 tests (42 core, 46 store, 16 transfer, 13 updater)
cargo test -p study-planner                                 ✅  14 tests de bóveda y host
cargo clippy (core/store/transfer/updater) -- -D warnings    ✅
cargo clippy -p study-planner --all-targets -- -D warnings   ✅
cargo check -p study-planner                                 ✅
./scripts/audit-network.sh                                   ✅
./scripts/audit-licenses.sh                                  ✅
```

> Los conteos pueden subir; lo importante es que no haya fallos.

---

## Entorno verificado

| Herramienta | Versión / estado |
|---|---|
| SO | CachyOS (Arch), Linux x86_64 |
| Node.js | 26.13.0 |
| npm | 12.2.2 |
| Rust | 1.99.1 |
| Cargo | 1.99.1 |
| WebKitGTK | 2.54.2 (`webkit2gtk-4.1`) |
| libsoup | 3.8.1 |
| gh | 2.103.3, autenticado como `wulflo` |
| VeraCrypt | montado en `/run/media/veracrypt1` (exFAT, 20 GB); **no escribir hasta Fase 5** |

`pnpm` no está instalado. El proyecto usa **npm** y `package-lock.json`.

---

## Reglas para no romper el proyecto (no negociables)

1. **Sin red en la app.** La webview no usa `http`, `fetch`, CDNs, fuentes remotas ni íconos remotos. La única excepción futura es el crate `study-planner-updater`, aislado y sin bindings JS.
2. **Sin permisos nuevos sin justificación.** `src-tauri/capabilities/default.json` contiene `core:default` más los permisos explícitos de ventana necesarios para la barra de título propia (`minimize`, `toggle-maximize`, `close`, `start-dragging`, `start-resize-dragging`). Prohibido agregar `fs`, `http`, `sql`, `shell`, `opener`, `dialog`, `notification` o `updater` al frontend.
3. **Los archivos y la base los escribe Rust.** El frontend nunca recibe rutas ni handles de archivos.
4. **Nada de `flutter_secure_storage`, cuentas, telemetría ni nube.** Study Planner es estrictamente local.
5. **La bóveda no se cifra.** Decisión explícita del usuario. No reintroducir contraseña de bóveda, Argon2id, AES-GCM ni recovery key.
6. **No cambiar el esquema sin migración nueva.** `0001_initial.sql` es inmutable una vez publicado. Todo cambio va en `0002_*.sql` con checksum.
7. **No cambiar el formato de exportación sin subir `export_format_version`.** El round-trip `exportar → importar → exportar` debe ser byte a byte idéntico dentro de la misma versión.
8. **Nunca tocar `<bóveda>/omega` ni `<bóveda>/longlife` sin backup verificado y confirmación.** La raíz de la bóveda se elige una vez; no se cambia en silencio.
9. **Nada de `sudo` para escribir la bóveda.** Si una ruta requiere permisos de root, es una ruta inválida.
10. **Sin secretos en el repo.** Claves, passphrases, `.env`, bases y backups están en `.gitignore`. La passphrase de firma jamás pasa por el chat ni por un archivo junto a la clave.
11. **Enteros de punto fijo.** Notas y pesos se guardan como enteros (centipuntos / basis points). No `REAL` para calificaciones.
12. **Borrado lógico.** Las entidades de dominio usan `deleted_at`; no hay `DELETE` físico.
13. **Dependencias permisivas.** MIT/Apache/BSD/ISC/Unicode. Cualquier licencia nueva (MPL, LGPL, GPL) requiere revisión y decisión registrada. Ver `THIRD_PARTY_LICENSES.md`.
14. **Cada cambio ejecuta `./scripts/check.sh`.** No se commitea con tests rojos.
15. **No actualizar dependencias a ciegas.** `npm update` y `cargo update` solo con internet, con tests posteriores y con el lockfile commiteado.
16. **Commits pequeños y descriptivos.** Un commit por cambio coherente; nunca mezclar refactor masivo con features.

---

## Fase 0 — Base técnica

### Hecho

- [x] Scaffold Tauri 2 + React 19 + TypeScript + Vite 8.
- [x] Renombrar app a `Study Planner`, identificador `app.studyplanner.desktop`; crates `study-planner-*`.
- [x] Workspace Rust: `study-planner-core`, `study-planner-store`, `study-planner-transfer`, `study-planner-updater`, `src-tauri`.
- [x] Esquema SQLite v1 completo (`crates/study-planner-store/migrations/0001_initial.sql`).
- [x] `Store` con WAL, claves foráneas, `synchronous=FULL`, timeout, migración con checksum SHA-256.
- [x] `vault_id`, `schema_version`, `integrity_check`, `foreign_key_check`.
- [x] Comandos Tauri `get_app_info` y `schema_self_check` (base en memoria).
- [x] CSP de producción y desarrollo; `freezePrototype`; `withGlobalTauri: false`.
- [x] Capacidad mínima: `core:default` + permisos de ventana para la barra de título; sin plugin `opener`.
- [x] UI shell con navegación, estado técnico, marca e ícono local.
- [x] Vistas provisionales para las secciones pendientes y modal de checklist del proyecto.
- [x] Bóveda local elegible con selector nativo desde Rust; configuración en la app y diagnóstico sin exponer la ruta.
- [x] Tareas persistentes con crear, completar, reabrir y eliminar mediante borrado lógico.
- [x] Vista «Hoy» con pendientes reales y vista «Tareas» funcional.
- [x] Ventana sin decoración nativa con barra de título propia: minimizar, maximizar/restaurar, cerrar, arrastre y bordes de redimensionado.
- [x] Tests de UI y de Rust; `typecheck`, `lint`, `test`, `build` verdes.
- [x] `docs/CHECKLIST.md` (este archivo).
- [x] Documentación base: `README.md`, arquitectura, modelo de datos, exportación, updater, claves, decisiones y modo offline.
- [x] `CHANGELOG.md` y `THIRD_PARTY_LICENSES.md`.
- [x] Scripts `scripts/check.sh` y `scripts/audit-network.sh`.
- [x] `.gitignore` endurecido (claves, bases, secretos, backups).
- [x] CI sin secretos en `.github/workflows/ci.yml`.
- [x] Repositorio git inicializado, commit inicial local y remoto GitHub configurado (`main`).
- [x] CI verde en GitHub y rama `main` protegida con el check «Tests y verificación».

### Cierre de Fase 0

- [x] Revisar con el usuario la licencia **MPL-2.0** transitiva de Tauri: aceptada como excepción acotada en **ADR-017**; no se modificaron los paquetes y se conservan avisos. Ver `THIRD_PARTY_LICENSES.md`.
- [x] Demo de Fase 0: abrir `npm run tauri dev`, verificar autodiagnóstico y navegación.
- [x] Confirmación explícita del usuario para pasar a Fase 1: «cierra la F0 y continua a F1».

**Estado:** Fase 0 **cerrada** el 2026-11-08. La app abre, muestra esquema v1 e integridad `ok`, los tests están verdes y no hay secretos versionados.

### Criterio de aceptación de Fase 0

- La app abre en Tauri y muestra esquema v1, integridad `ok`, 0 violaciones de claves foráneas.
- `./scripts/check.sh` termina en verde.
- `./scripts/audit-network.sh` no encuentra red en la webview.
- No existe ninguna clave, base ni secreto versionado.
- El usuario confirma el cierre.

---

## Fase 1 — Bóveda, rutas, backups y transferencia

Objetivo: abrir/crear una bóveda real, respaldarla y mover todos los datos con un `.txt` canónico.

- [x] Selección de la raíz de la bóveda en el primer arranque; la app crea `<bóveda>/omega` y `<bóveda>/longlife/<dispositivo>`.
- [x] Snapshots diarios en `<bóveda>/longlife/<dispositivo>/<AAAA-MM-DD>`, retención de 7 por dispositivo.
- [x] Validación de rutas: existen, son escribibles, no requieren `sudo`, no están dentro del repo.
- [x] Asistente de primer arranque; nunca cambiar rutas en silencio. La UI obliga a elegir carpeta y «Cambiar carpeta» siempre pide confirmación explícita.
- [x] Apertura de bóveda con lock file y detección de otra instancia.
- [x] Migración con backup previo obligatorio y restauración automática si falla.
- [x] Backups con API de backup online de SQLite, checksum y verificación por `integrity_check`.
- [x] Restauración desde backup con confirmación y backup del estado actual (`pre-restore-*.db`).
- [x] Implementar exportador canónico v1 y importador.
- [x] Adjuntos embebidos por defecto, con SHA-256 verificado.
- [x] Importación en bóveda vacía; el modo reemplazo existe en el núcleo pero la UI no lo habilita.
- [x] Fixtures de exportación y test de round-trip byte a byte.
- [x] Tests de checksum corrupto, versión futura, adjunto faltante, columna inválida, FK inválida y rollback.
- [x] Pantalla «Bóveda» con estado, diagnóstico y cambio de carpeta.
- [x] Backup manual desde la pantalla «Bóveda» (botón «Crear backup ahora»).
- [x] Exportar, importar y restaurar desde la pantalla «Bóveda».
- [x] Registrar eventos de backup/export/import/restore en `<omega>/study-planner.log` (sin rutas absolutas ni datos sensibles).

**Aceptación:** crear bóveda, escribir un registro de prueba, exportar, importar en bóveda vacía, exportar de nuevo y obtener bytes idénticos; restaurar un backup verificado.

---

## Fase 2 — Año académico, materias, horario y asistencia

- [x] CRUD de año académico, términos, materias y horarios.
- [x] Rotaciones semanales A/B, paridad y ciclos de días.
- [x] Feriados y excepciones de clase (cancelada, movida, extra).
- [x] Vista semanal y vista «Hoy».
- [x] Registro de asistencia (presente, ausente, tarde, justificado).
- [x] Porcentaje de asistencia y umbral mínimo por materia.
- [x] Cálculo de próxima clase y clases solapadas.
- [x] Tests de recurrencia, feriados, excepciones y cambios de horario.
- [x] Exportación/importación de las nuevas entidades sin romper v1.

**Aceptación:** cargar un semestre completo, ver la semana correcta, registrar asistencia y detectar solapamientos.

---

## Fase 3 — Exámenes, notas, tareas y búsqueda

- [x] CRUD de exámenes, temas y pesos.
- [x] Escalas de nota, categorías y entradas; promedio ponderado y descarte del menor.
- [x] CRUD básico de tareas con estados `pending` / `in_progress` / `done` y borrado lógico.
- [x] Subtareas, etiquetas, prioridad, recurrencia, vencimientos y filtros de tareas.
- [x] Filtros guardados y búsqueda FTS local.
- [x] Journal de undo con expiración.
- [x] Vistas de calendario, exámenes, notas y tareas.
- [x] Tests de cálculo de notas, pesos, recurrencia y búsqueda.
- [x] Exportación/importación de las nuevas entidades sin romper v1.

**Aceptación:** calcular la nota final de una materia con categorías y ponderaciones, y encontrar tareas por texto y etiqueta.

---

## Fase 4 — Estudio, repaso espaciado y estadísticas

- [x] Temas jerárquicos y autoevaluación (1–5).
- [x] Clasificación fortaleza/neutral/debilidad con override manual.
- [x] Repaso espaciado SM-2 con versión de algoritmo.
- [x] Sesiones de estudio y Pomodoro.
- [x] Prioridad de estudio determinista y explicable.
- [x] Estadísticas: tiempo, constancia, materias débiles, próximos repasos.
- [x] Tests de SM-2, prioridad y agregaciones.
- [x] Exportación/importación de las nuevas entidades sin romper v1.

**Aceptación:** generar una cola de repaso reproducible y explicar por qué un tema aparece primero.

---

## Fase 5 — Updater, notificaciones y empaquetado

- [x] Generar clave minisign y passphrase localmente (protocolo en `docs/KEYS.md`).
- [x] Backup cifrado del par en Drive: `study-planner-minisign-2026-11-09.tar.gz.gpg` en `gdrive:study-planner-backups/`, cifrado simétrico AES-256 con la misma passphrase, descifrado verificado por `cmp` y copia remota verificada con `rclone check` (0 diferencias).
- [ ] Custodia completa: copia offline y confirmación de la passphrase en Bitwarden + papel.
- [ ] Configurar secrets de GitHub con `gh secret set` desde archivos, nunca en YAML.
- [x] Implementar `study-planner-updater` con URL HTTPS fija en compilación.
- [x] Verificación de firma obligatoria; rechazar manifiestos sin firma válida.
- [x] Descarga, verificación, instalación y rollback en Linux AppImage.
- [x] Sin bindings JS; sin datos de usuario en la petición.
- [x] Notificaciones locales (clases, tareas, exámenes, repasos) con permisos solo en Rust.
- [x] Empaquetado AppImage; `.deb` opcional.
- [x] Auditoría final de red, licencias, CSP, permisos y backups.
- [x] Documentar recuperación de clave y simulacro de rotación.

**Aceptación:** una compilación publicada se actualiza solo con firma válida; una firma inválida se rechaza y queda registrada.

---

## Decisiones cerradas

| Tema | Decisión |
|---|---|
| Plataforma | Desktop Linux primero; Tauri 2. Windows/macOS no prioritarios |
| Móvil | Fuera de alcance inicial; Tauri updater no soporta Android/iOS |
| Estructura de bóveda | `<raíz>/omega` (datos vivos + backup principal) y `<raíz>/longlife/<dispositivo>/<AAAA-MM-DD>` (snapshots, 7 por dispositivo) |
| Frontend | React + TypeScript + Vite + Zustand + date-fns + Material Color Utilities |
| Persistencia | SQLite vía `rusqlite` en Rust; sin `tauri-plugin-sql` |
| Cifrado de bóveda | **No** (decisión explícita del usuario) |
| Exportación | `.txt` canónico, total, determinista, con checksum |
| Adjuntos | Embebidos por defecto; modo referencia no portable (futuro) |
| Red | Solo updater aislado, opt-in, HTTPS fijo, firma obligatoria |
| Updater Linux | AppImage únicamente |
| Licencia del proyecto | MIT OR Apache-2.0 |
| Gestor de paquetes | npm (pnpm no disponible) |
| Idioma de UI | Español inicial; i18next preparado |

---

## Pendientes de decisión del usuario

- [x] ¿Se acepta la dependencia transitiva **MPL-2.0** de Tauri o se busca una alternativa? Aceptada en **ADR-017** (2026-11-08), sin modificar los paquetes.
- [ ] Confirmar URL HTTPS fija del updater. Tentativa: `https://github.com/wulflo/study-planner/releases/latest/download/latest.json`.
- [ ] ¿Firma de releases en CI o firma offline? Recomendado: offline.
- [ ] Custodia final de la clave y su passphrase (VeraCrypt, copia offline, Drive cifrado).
- [x] Repositorio publicado en GitHub: [`wulflo/study-planner`](https://github.com/wulflo/study-planner), público, sin datos ni secretos.
- [x] Estructura de bóveda `<raíz>/omega` + `<raíz>/longlife` confirmada en **ADR-016**.
- [ ] Evaluar la alerta de Dependabot de `glib` 0.18.5 (unsoundness en iteradores de `VariantStrIter`, severidad media): viene de `gtk 0.18` en la pila de Tauri y la corrección exige `glib 0.20`, es decir, una actualización mayor de Tauri/gtk-rs. No se usa directamente en el código de Study Planner. Las actualizaciones automáticas de `glib < 0.20.0` se ignoran en `.github/dependabot.yml` para que el workflow de Dependabot no falle; la alerta sigue abierta para reevaluar al actualizar Tauri/gtk-rs.

---

## Comandos de referencia

```bash
# Desarrollo
npm install                 # solo con internet
npm run tauri dev           # app completa

# Verificación rápida
npm run typecheck
npm run lint
npm test
npm run build
npm run check               # typecheck + lint + test + build

# Verificación completa
npm run check:all
./scripts/check.sh
./scripts/audit-network.sh
./scripts/audit-licenses.sh

# Rust
cargo fmt --all -- --check
cargo clippy -p study-planner-core -p study-planner-store -p study-planner-transfer --all-targets -- -D warnings
cargo clippy -p study-planner --all-targets -- -D warnings
cargo test -p study-planner-core -p study-planner-store -p study-planner-transfer
cargo test -p study-planner
cargo check -p study-planner

# Empaquetado (Fase 5)
npm run tauri build
```

---

## Modo offline

Resumen; detalle en `docs/OFFLINE.md`.

- El repo debe compilar sin internet si `node_modules`, `~/.npm`, `~/.cargo` y `target/` ya existen.
- `npm ci --offline` y `cargo build --offline` son los comandos de emergencia.
- No ejecutar `npm update`, `cargo update`, `npm install` ni `cargo add` sin red.
- Las dependencias del sistema (WebKitGTK, GTK, libsoup) no se pueden instalar offline.
- Al volver internet: sincronizar, correr `./scripts/check.sh`, actualizar este checklist y commitear.

---

## Mapa de archivos

```text
.
├── src/                         UI React
│   ├── App.tsx                  shell y autodiagnóstico
│   ├── components/              barra de título y bordes de ventana
│   ├── lib/ipc.ts               contrato IPC tipado
│   └── styles.css               tokens Material 3
├── src-tauri/                   host Tauri
│   ├── src/lib.rs               comandos y builder
│   ├── src/updater.rs           comandos del updater aislado
│   ├── src/notifications.rs     planificador y D-Bus
│   ├── tauri.conf.json          CSP, ventana, bundle
│   └── capabilities/default.json permisos mínimos
├── crates/
│   ├── study-planner-core/              dominio puro, constantes, IDs
│   ├── study-planner-store/             SQLite, migraciones, backups
│   ├── study-planner-transfer/          exportación/importación canónica
│   └── study-planner-updater/           red aislada (Fase 5)
├── docs/                        documentación persistente
├── scripts/                     check y auditoría de red
├── fixtures/export/             fixtures de round-trip
└── .github/workflows/ci.yml     CI sin secretos
```

---

## Registro de sesiones

### 2026-11-08 — Fase 0

- Se creó el scaffold, el workspace Rust y el esquema v1.
- Se implementaron `Store`, migraciones con checksum y comandos de autodiagnóstico.
- Se configuraron CSP, permisos mínimos, UI shell y tests.
- Se documentó la arquitectura, el modelo de datos, el formato de exportación, el updater, las claves y el modo offline.
- Se endureció `.gitignore`, se agregaron scripts de verificación/auditoría y CI sin secretos.
- Se publicó el repositorio público [`wulflo/study-planner`](https://github.com/wulflo/study-planner) y se subió `main`.
- Se configuraron descripción, temas, issues, auto-merge, borrado de rama al mergear, alertas de vulnerabilidades y protección de `main` con CI requerido.
- Se integró una barra de título propia con minimizar, maximizar/restaurar y cerrar, más arrastre y bordes de redimensionado.
- Se renombró el proyecto de `Omega` a `Study Planner`: crates `study-planner-*`, identifier `app.studyplanner.desktop`, magic `STUDY-PLANNER-EXPORT`.
- Se fijó la estructura de bóveda `<raíz>/omega` + `<raíz>/longlife/<dispositivo>/<AAAA-MM-DD>` siguiendo el patrón de Better Tracker.
- Pendiente: revisión MPL y confirmación para Fase 1.

### 2026-11-08 — Revisión en vivo y logo

- Se verificó la app en Tauri: esquema v1, integridad `ok`, 0 violaciones de claves foráneas y formato de exportación v1.
- Se detectó que la navegación solo cambiaba la etiqueta del hero; se agregaron vistas provisionales por sección con su fase y alcance.
- «Ver checklist» abre un modal con el resumen de Fase 0; «Abrir bóveda» queda deshabilitado y señalizado como Fase 1 (luego reemplazado por el corte funcional).
- Se integró el logo propio en la barra de título, el favicon y los íconos de compilación generados con `tauri icon`.
- Demo de Fase 0 verificada; queda la confirmación del usuario y la revisión MPL.

### 2026-11-08 — Corte funcional: bóveda y tareas

- Se agregó la selección de bóveda con selector nativo desde Rust (`DialogExt`); la webview no recibe la ruta ni tiene permiso `dialog`.
- La raíz se guarda en `vault.json` dentro de la configuración de la app; la base se abre en `<bóveda>/omega/study-planner.db` con migración e informe de integridad.
- Se agregaron `Task`, `TaskStatus` y validación de títulos en `study-planner-core`, más el repositorio con borrado lógico en `study-planner-store`.
- Se agregaron los comandos `get_vault_status`, `choose_vault`, `list_tasks`, `create_task`, `set_task_done` y `delete_task`.
- «Bóveda» y «Tareas» dejaron de ser placeholders; «Hoy» muestra los pendientes reales.
- Verificación en vivo: bóveda abierta con esquema v1, integridad `ok` y 0 violaciones de claves foráneas; tareas visibles después de reiniciar; completar, reabrir y eliminar actualizaron la base. La creación quedó cubierta por tests de UI y del store.
- `npm run check` con 12 tests de UI; 15 tests de Rust; clippy de `src-tauri`; `.deb` y AppImage regenerados con el corte funcional.
- Se limpiaron los datos y la configuración de la bóveda usados en la verificación manual: el próximo arranque vuelve a pedir una carpeta.

### 2026-11-08 — Cierre F0 y corte F1: backups verificados y snapshots

- Se revisó y aceptó la **MPL-2.0** transitiva de Tauri como excepción acotada (**ADR-017**); no se modificaron los paquetes y se documentaron las obligaciones de avisos.
- **Fase 0 cerrada** con confirmación explícita del usuario. Queda registrado que la exportación `.txt` y el updater siguen en fases posteriores.
- Se habilitó `rusqlite` con la feature `backup` y se agregó `Store::backup_to` / `Store::verify_backup`: backup online, escritura `.partial`, verificación previa al renombre atómico, SHA-256, `integrity_check` y `foreign_key_check`.
- Se agregó validación de la raíz: existe, es carpeta, es escribible, no es la raíz del sistema y no está dentro del repositorio.
- Se agregó un lock de archivo en `<omega>/.study-planner.lock`; una segunda instancia recibe un error explícito y no abre la bóveda.
- La apertura migra con backup previo verificado (`pre-migration-vN-*.db`) y restaura automáticamente si la migración falla.
- Se mantiene un backup principal verificado en `<omega>/backup/study-planner.db`.
- Se crean snapshots diarias en `<longlife>/<dispositivo>/<AAAA-MM-DD>` con `snapshot.json`; las incompletas o de esquema viejo se descartan y se conservan las últimas 7.
- La vista «Bóveda» muestra tamaño, integridad, fecha, SHA-256 del backup y snapshots, con el botón «Crear backup ahora».
- Verificación en vivo con una bóveda temporal fuera del repo: estructura correcta, permisos `0700`/`0600`, backup y snapshot con el **mismo SHA-256**, integridad `ok`, 0 violaciones de FK; el botón actualizó el backup y mostró el aviso de éxito.
- Tests: 13 de UI, 6 de core, 11 de store y 8 de bóveda (26 de Rust en total). `npm run check`, `cargo fmt`, clippy y tests verdes.
- Se actualizaron `scripts/check.sh` y `package.json` para incluir clippy y tests del host Tauri, y `scripts/audit-network.sh` para aceptar `tauri-plugin-dialog` solo desde Rust (sin permiso `dialog:` ni import del frontend). La auditoría pasa completa.
- Se regeneraron los artefactos con `npm run tauri build`: `.deb` (3.16 MiB), `.rpm` (3.17 MiB) y AppImage (109.05 MiB).
- Se limpió la bóveda temporal y `vault.json`; el próximo arranque vuelve a pedir carpeta.

### 2026-11-08 — F1 cerrada: transferencia canónica, restauración y log local

- Se completó `study-planner-transfer`: exportación canónica de todas las tablas, adjuntos embebidos en base64 con SHA-256, parseo estricto de cabecera/cuerpo/orden, importación transaccional y reconstrucción de FTS.
- La importación normal es `EmptyOnly`: se rechaza una bóveda con datos de dominio. El modo `Replace` existe en el núcleo, pero la UI no lo habilita hasta definir backup y confirmación específicos.
- Se agregó `hasData` a `VaultStatus` para que la UI bloquee la importación cuando ya hay datos.
- Comandos nuevos: `export_vault`, `import_vault` y `restore_main_backup`. Los selectores de archivo se abren solo desde Rust; la webview nunca recibe rutas.
- La restauración verifica el backup principal, crea un backup `pre-restore-*.db`, reemplaza la base, reabre la bóveda y conserva el lock.
- Cada backup, exportación, importación, error y restauración se registra en `<omega>/study-planner.log` con timestamp UTC, sin rutas absolutas.
- La vista «Bóveda» incorpora exportación, importación solo en bóveda vacía y restauración con confirmación explícita.
- Tests: 17 de UI, 6 de core, 11 de store, 13 de transfer y 11 de bóveda (41 de Rust). `./scripts/check.sh` y `./scripts/audit-network.sh` pasan completos.
- Aceptación verificada: exportar → importar en bóveda vacía → exportar produce bytes idénticos; también con todas las tablas, borrados lógicos y un adjunto. La restauración de un backup verificado está cubierta por test y por el comando de UI.
- Artefactos de F1 regenerados con `npm run tauri build`: `.deb` (3.25 MiB), `.rpm` (3.25 MiB) y AppImage (109.12 MiB).

### 2026-11-08 — Fase 2 en curso (bitácora por pasos)

> Esta bitácora se actualiza **después de cada paso**, antes de seguir, para poder
> retomar el trabajo si se corta la conexión o la sesión. Cada paso termina con
> `./scripts/check.sh` en verde y un commit local pequeño.

- [x] **Paso 0 — Línea base:** `./scripts/check.sh` verde (17 tests de UI y 41 de Rust) antes de tocar nada.
- [x] **Paso 1 — Modelos puros:** año académico, término, materia, franja horaria, feriado, excepción, asistencia y rotación con validaciones y matemática de fechas propia (sin dependencias nuevas). 15 tests de `study-planner-core`.
- [x] **Paso 2 — Persistencia académica:** repositorios CRUD con borrado lógico para año, término y materia; tests de validación y FK. 14 tests de `study-planner-store`.
- [x] **Paso 3 — Persistencia de horario:** franjas, rotaciones, feriados, excepciones y asistencia; vista semanal compuesta y próxima clase. 20 tests de `study-planner-store`.
- [x] **Paso 4 — Cálculo semanal:** ocurrencias efectivas, rotaciones (A/B, paridad, ciclo de días, bisemanal), feriados, excepciones, próxima clase, solapamientos y porcentaje de asistencia; tests puros y de store.
- [x] **Paso 5 — Comandos e IPC:** comandos Tauri y contrato tipado en `src/lib/ipc.ts` para toda la estructura académica y el horario. Checkpoint de la Fase 1 pendiente de commit incluido; `./scripts/check.sh` verde.
- [x] **Paso 6 — UI académica:** sección «Materias» con año, término y materias, edición y borrado lógico con mensajes de conflicto. 18 tests de UI.
- [x] **Paso 7 — UI de horario:** sección «Horario» con grilla semanal navegable, alta/edición/baja lógica de franjas, configuración de rotación, feriados, excepciones y registro de asistencia por clase, más solapamientos y umbral por materia. 20 tests de UI; `./scripts/check.sh` verde.
- [x] **Paso 8 — Vista Hoy:** próxima clase y clases del día con datos reales, registro de asistencia y aviso de umbral. 21 tests de UI; `./scripts/check.sh` verde.
- [x] **Paso 9 — Transferencia:** fixture con las ocho tablas de Fase 2 y test que verifica exportación, importación, semántica (franja, excepción, asistencia, rotación) y round-trip byte a byte; se corrigió la unicidad de directorios temporales en tests concurrentes. 61 tests de Rust (16 core, 20 store, 14 transfer, 11 host); `./scripts/check.sh` verde.
- [x] **Paso 10 — Cierre:** documentación de semántica de rotaciones/asistencia en `docs/DATA_MODEL.md`, changelog actualizado, resumen del proyecto en la UI y verificación final con `./scripts/check.sh` y build de producción. Fase 2 cerrada.

**Estado Fase 2:** 21 tests de UI y 61 tests de Rust en verde. Sin dependencias nuevas, sin permisos nuevos, sin migración (el esquema v1 ya tenía las tablas) y `export_format_version = 1` intacto.

### 2026-11-08 — Fase 3 en curso (bitácora por pasos)

> Misma regla que la Fase 2: cada paso termina con `./scripts/check.sh` en verde
> y un commit local pequeño.

- [x] **Paso 0 — Línea base:** `./scripts/check.sh` verde (21 tests de UI y 61 de Rust) antes de tocar nada.
- [x] **Paso 1 — Modelos puros:** exámenes, temas, escalas, categorías, entradas y cálculo de promedio ponderado con descarte del menor; extensión de tareas con subtareas, etiquetas, prioridad, recurrencia y filtros. 31 tests de `study-planner-core`.
- [x] **Paso 2 — Persistencia de exámenes y temas:** CRUD con borrado lógico, jerarquía sin ciclos y vínculo examen–tema.
- [x] **Paso 3 — Persistencia de notas:** escalas, categorías y entradas; promedio por materia con enteros de punto fijo.
- [x] **Paso 4 — Tareas completas:** subtareas, etiquetas, prioridad, vencimientos, recurrencia, filtros guardados, búsqueda FTS y journal de undo con expiración. 32 tests de `study-planner-store`.
- [x] **Paso 5 — Comandos e IPC:** comandos Tauri y contrato tipado para todas las entidades nuevas.
- [x] **Paso 6 — UI de exámenes y temas:** sección «Exámenes» funcional con pesos, resultados y vínculo a temas.
- [x] **Paso 7 — UI de notas:** sección «Notas» con escalas, categorías, entradas y promedio final.
- [x] **Paso 8 — UI de tareas y búsqueda:** subtareas, etiquetas, prioridad, filtros guardados y deshacer.
- [x] **Paso 9 — Calendario:** vista mensual con exámenes y vencimientos de tareas.
- [x] **Paso 10 — Transferencia y cierre:** fixture de Fase 3 con semántica y round-trip, documentación, changelog y verificación final.

**Estado Fase 3:** 25 tests de UI y 89 tests de Rust en verde (31 core, 32 store, 15 transfer, 11 host). Sin dependencias nuevas, sin permisos nuevos, sin migración (el esquema v1 ya tenía las tablas) y `export_format_version = 1` intacto. La aceptación se verifica con el promedio ponderado con descarte del menor en «Notas» y la búsqueda de tareas por texto y etiqueta.

### 2026-11-08 — Fase 4 en curso (bitácora por pasos)

> Misma regla que las fases anteriores: cada paso termina con `./scripts/check.sh` en
> verde y un commit local pequeño. La bitácora se actualiza **antes** de seguir.

- [x] **Paso 0 — Línea base:** `./scripts/check.sh` verde (25 tests de UI y 89 de Rust) antes de tocar nada.
- [x] **Paso 1 — Modelos puros:** autoevaluación 1–5, clasificación con override, SM-2 versionado, prioridad explicable, sesiones y estadísticas. 42 tests de `study-planner-core` (11 nuevos).
- [x] **Paso 2 — Persistencia de estudio:** autoevaluaciones, overrides, ítems de repaso, sesiones y consultas agregadas con tests. 39 tests de `study-planner-store` (7 nuevos).
- [x] **Paso 3 — Comandos e IPC:** comandos Tauri y contrato tipado para todas las entidades de estudio.
- [x] **Paso 4 — UI de fortalezas:** temas, autoevaluación, clasificación, override y repaso desde la sección «Fortalezas».
- [x] **Paso 5 — UI de estudio:** cola de prioridad explicable, Pomodoro y registro de sesiones desde la sección «Estudio».
- [x] **Paso 6 — UI de estadísticas:** tiempo, constancia, materias débiles y próximos repasos. 29 tests de UI.
- [x] **Paso 7 — Transferencia y cierre:** fixture de Fase 4 con semántica y round-trip. `self_assessment`, `strength_override`, `review_item` y `study_session` viajan en la exportación canónica v1; el test reimporta, verifica SM-2, override y minutos reales, y compara bytes. 16 tests de transfer.

**Estado Fase 4:** 29 tests de UI y 108 tests de Rust en verde (42 core, 39 store, 16 transfer, 11 host). Sin dependencias nuevas, sin permisos nuevos, sin migración (el esquema v1 ya tenía las tablas) y `export_format_version = 1` intacto. La aceptación se verifica con la cola de prioridad explicable en «Estudio»: cada tema muestra sus motivos puntuados y el orden es determinista. **Fase cerrada** al continuar con la Fase 5 a pedido del usuario.

### 2026-11-08 — Fase 5 en curso (bitácora por pasos)

> Misma regla que las fases anteriores: cada paso termina con `./scripts/check.sh` en
> verde y un commit local pequeño. La bitácora se actualiza **antes** de seguir.

- [x] **Paso 0 — Línea base y corrección visual:** `./scripts/check.sh` verde antes de tocar nada. Se normalizaron los controles en toda la interfaz: selects con padding derecho y chevron propio, botones con menos padding y radio de 12 px, y separación mínima entre botones contiguos.
- [x] **Paso 1 — Núcleo del updater:** manifiesto, validación de URL/host, comparación de versión, firma minisign y SHA-256 con tests. 9 tests de `study-planner-updater`; la firma debe declarar la misma versión que el manifiesto.
- [x] **Paso 2 — Descarga e instalación:** cliente HTTP aislado con timeout y hosts permitidos, descarga temporal, verificación SHA-256 + firma + versión, reemplazo atómico, backup y rollback. 13 tests del updater (4 nuevos; sin red real en tests).
- [x] **Paso 3 — Integración Tauri y UI:** comandos Rust `get_updater_status`, `set_updater_auto_check`, `check_updates`, `install_update`, `rollback_update` y `list_network_events`; ajuste opt-in persistido en `setting`; registro en `network_event`; vista «Ajustes» con estado, búsqueda, instalación, rollback y auditoría. La webview nunca recibe URL, firma ni ruta configurable. 32 tests de UI y 13 del host.
- [x] **Paso 4 — Notificaciones locales:** cálculo determinista de recordatorios de clases, tareas, exámenes y repasos en `study-planner-store`; reclamo con `dedupe_key` único, marcado `fired`, liberación si falla la entrega y listado. Planificador cada 60 s y entrega por D-Bus (`org.freedesktop.Notifications`) desde Rust, sin plugin ni permisos de webview. Vista «Ajustes» con activación, tiempos de anticipación visibles, revisión manual y notificación de prueba. 4 tests nuevos del store y 1 del host.
- [x] **Paso 5 — Empaquetado, CI y cierre documental:** AppImage compilado y verificado localmente (`Study Planner_0.1.0_amd64.AppImage`, 110,79 MiB); workflow de release con build en GitHub Actions, firma con `--app-version`, generación de `latest.json` y verificación contra la clave embebida antes de publicar. CI actualizado a los crates reales, con updater, host y auditoría. Inventario de licencias actualizado (499 paquetes Cargo). Simulacro de firma/rotación con clave de test fuera del repo. Guías en `docs/RELEASE.md`, `docs/NOTIFICATIONS.md` y `docs/KEYS.md`.
- [x] **Clave de producción generada:** par minisign creado dentro de `/run/media/veracrypt1/study-planner-keys`, passphrase aleatoria en un archivo temporal con permisos `600`, firma de prueba verificada contra la clave pública y clave pública copiada a `crates/study-planner-updater/updater.pub`. La passphrase y la clave privada nunca se mostraron en el chat ni se versionaron.
- [x] **Backup cifrado en Drive:** `/run/media/veracrypt1/Repo - Keys/study-planner-keys/study-planner-minisign-2026-11-09.tar.gz.gpg` (AES-256, S2K SHA-512, 65011712 iteraciones) subido a `gdrive:study-planner-backups/`. Se descifró en un directorio temporal y los tres archivos coincidieron con `cmp`; la copia remota se verificó con `rclone check` (0 diferencias, SHA-256 `1b836fa2…ffa6`). El `.gpg` se abre con la passphrase de minisign y no la contiene.
- [ ] **Cierre real de Fase 5:** depende de custodiar la passphrase (Bitwarden + papel), copiar la clave a la copia offline, configurar los secrets y publicar una release firmada.

> **Pendiente del usuario:** confirmar que la passphrase quedó guardada en Bitwarden y papel para borrar `/tmp/opencode/study-planner-keygen/passphrase.txt`. Después hay que configurar los secrets del release workflow y publicar la primera release firmada.

### 2026-11-09 — Fase 6 en curso (bitácora por pasos)

> Misma regla que las fases anteriores: cada paso termina con `./scripts/check.sh` en
> verde y un commit local pequeño.

- [x] **Paso 0 — Línea base:** `./scripts/check.sh` verde (29 tests de UI y 108 de Rust) antes de tocar nada.
- [x] **Paso 1 — Detección de Snap en el updater:** variante `UpdateError::ManagedBySnap`, `Runtime` (`AppImage`/`Snap`/`Development`) con precedencia de `SNAP` sobre `APPIMAGE`, guardas en estado, auto-check, búsqueda e instalación. Test `snap_takes_precedence_and_disables_appimage_updates`. 15 tests del host.
- [x] **Paso 2 — UI de Snap:** `managedBySnap` en el contrato IPC, aviso en «Ajustes» y controles de updater deshabilitados. Test de UI para el modo snap. 35 tests de UI.
- [x] **Paso 3 — Empaquetado:** `snap/snapcraft.yaml` con `core24`, `strict`, extensión `gnome`, plugs `home` y `removable-media`, sin plugs de red; `.snapcraftignore`, ícono de tienda y `layout` de WebKitGTK 4.1.
- [x] **Paso 4 — CI de build:** `.github/workflows/snap.yml` compila y sube el `.snap` como artefacto; no publica. `scripts/check-snap-version.sh` sincroniza la versión con la app y `scripts/check.sh` / `scripts/audit-network.sh` validan el confinamiento y la ausencia de red.
- [x] **Paso 5 — Documentación:** `docs/SNAP.md`, sección snap en `docs/UPDATER.md`, README, changelog y licencias.
- [x] **Registro y credenciales:** nombre `study-planner` registrado a la cuenta `wulflo` y `SNAPCRAFT_STORE_CREDENTIALS` cargado en GitHub con un token limitado a `study-planner`/`edge`, ACLs de paquete y expiración 2027-11-08.
- [ ] **Publicación:** agregar `action-publish` al workflow y probar el snap en Ubuntu 24.04 limpio. No se hace hasta que el usuario lo pida.

**Estado Fase 6:** 35 tests de UI, 15 tests del host y el resto de la suite en verde. Snap estricto sin red, updater deshabilitado bajo snap, workflow de solo build, nombre registrado y credenciales cargadas. La publicación queda pendiente de la prueba en limpio y de una decisión explícita del usuario.

### 2026-11-10 — Paquetes de escritorio (extensión de release)

- [x] **Matriz de release:** `release.yml` compila Linux (`AppImage`, `.deb`, `.rpm`), Windows (`.exe` NSIS, `.msi`) y macOS (`.dmg`), firma cada artefacto con `TAURI_SIGNING_PRIVATE_KEY` y publica una release *latest*.
- [x] **Port mínimo multiplataforma:** `zbus` quedó como dependencia solo de Linux; las notificaciones devuelven un error explícito fuera de Linux y el planificador no arranca. El updater sigue siendo solo AppImage/Snap.
- [x] **Firma y sumas:** `scripts/package-artifacts.mjs` firma `.deb`, `.rpm`, `.exe`, `.msi` y `.dmg`, copia cada `.sig` y genera `SHA256SUMS` por plataforma.
- [x] **Secrets:** `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` cargados en GitHub desde los archivos de la bóveda, sin imprimirlos.
- [ ] **Validar en CI:** ejecutar el workflow en las tres plataformas y corregir lo que falle.
- [ ] **Firma de código opcional:** Authenticode en Windows y Developer ID + notarización en macOS. La firma Tauri actual es solo de integridad.

**Estado:** Linux es la plataforma completa; Windows/macOS son builds experimentales sin notificaciones locales ni updater. La firma Tauri no reemplaza la firma de código del sistema operativo.

---

## Próximos 5 pasos

1. Guardar la passphrase en Bitwarden y papel, y confirmar para borrar el archivo temporal.
2. Copiar `study-planner.key` y `.pub` a la copia offline (el backup `.gpg` de Drive ya está subido).
3. Ejecutar el workflow de release en Linux, Windows y macOS; corregir lo que falle.
4. Publicar una release firmada (`v0.2.0` o superior) y probar el updater real.
5. Probar el `.snap` en Ubuntu 24.04 limpio y, si se pide, publicar en `edge`.
