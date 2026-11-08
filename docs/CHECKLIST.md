# Study Planner — Checklist persistente

> **Última actualización:** 2026-11-08
> **Fase actual:** 4 — Estudio, repaso espaciado y estadísticas (en curso)
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
| Fase | 4 — Estudio, repaso espaciado y estadísticas (en curso) |
| Versión de app | 0.1.0 |
| Esquema SQLite | v1 |
| Formato de exportación | v1 implementado y verificado con round-trip byte a byte |
| Frontend | React 19 + TypeScript + Vite 8 |
| Backend | Rust + Tauri 2.12 + `rusqlite` 0.41 (bundled, backup) |
| Red en la webview | Prohibida por CSP y permisos |
| Updater | Desactivado (`UPDATER_IMPLEMENTED = false`) |
| Bóveda real | Selección local, lock, backups verificados, snapshots, exportación/importación canónica, restauración con backup previo y log local |
| Secretos | Ninguno generado, ninguno en el repo |
| Git | Repositorio local + remoto GitHub [`wulflo/study-planner`](https://github.com/wulflo/study-planner) (público); `main` con commit inicial `87f2be8` |
| CI | GitHub Actions verde en `main`; check requerido «Tests y verificación» |

### Verificación vigente

```text
npm run check       ✅  25 tests de UI
cargo test -p study-planner-core -p study-planner-store -p study-planner-transfer   ✅  78 tests (31 core, 32 store, 15 transfer)
cargo test -p study-planner                                 ✅  11 tests de bóveda
cargo clippy (core/store/transfer) -- -D warnings           ✅
cargo clippy -p study-planner --all-targets -- -D warnings   ✅
cargo check -p study-planner                                 ✅
./scripts/audit-network.sh                                   ✅
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

- [ ] Temas jerárquicos y autoevaluación (1–5).
- [ ] Clasificación fortaleza/neutral/debilidad con override manual.
- [ ] Repaso espaciado SM-2 con versión de algoritmo.
- [ ] Sesiones de estudio y Pomodoro.
- [ ] Prioridad de estudio determinista y explicable.
- [ ] Estadísticas: tiempo, constancia, materias débiles, próximos repasos.
- [ ] Tests de SM-2, prioridad y agregaciones.
- [ ] Exportación/importación de las nuevas entidades sin romper v1.

**Aceptación:** generar una cola de repaso reproducible y explicar por qué un tema aparece primero.

---

## Fase 5 — Updater, notificaciones y empaquetado

- [ ] Generar clave minisign y passphrase localmente (protocolo en `docs/KEYS.md`).
- [ ] Custodia en VeraCrypt + copia offline + Drive cifrado; passphrase en Bitwarden + papel.
- [ ] Configurar secrets de GitHub con `gh secret set` desde archivos, nunca en YAML.
- [ ] Implementar `study-planner-updater` con URL HTTPS fija en compilación.
- [ ] Verificación de firma obligatoria; rechazar manifiestos sin firma válida.
- [ ] Descarga, verificación, instalación y rollback en Linux AppImage.
- [ ] Sin bindings JS; sin datos de usuario en la petición.
- [ ] Notificaciones locales (clases, tareas, exámenes, repasos) con permisos solo en Rust.
- [ ] Empaquetado AppImage; `.deb` opcional.
- [ ] Auditoría final de red, licencias, CSP, permisos y backups.
- [ ] Documentar recuperación de clave y simulacro de rotación.

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
- [ ] Evaluar la alerta de Dependabot de `glib` 0.18.5 (unsoundness en iteradores de `VariantStrIter`, severidad media): viene de `gtk 0.18` en la pila de Tauri y la corrección exige `glib 0.20`, es decir, una actualización mayor de Tauri/gtk-rs. No se usa directamente en el código de Study Planner.

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
- [ ] **Paso 7 — Transferencia y cierre:** fixture de Fase 4 con semántica y round-trip, documentación, changelog y verificación final.

---

## Próximos 5 pasos

1. Abrir Fase 4: temas jerárquicos, autoevaluación y repaso espaciado.
2. Implementar SM-2 y la cola de prioridad explicable.
3. Estadísticas de estudio, constancia y materias débiles.
4. Mantener la exportación/importación de cada entidad nueva sin romper `export_format_version = 1`.
5. Cerrar la Fase 4 con demo, tests verdes, changelog y confirmación del usuario.
