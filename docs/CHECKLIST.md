# Omega — Checklist persistente

> **Última actualización:** 2026-11-08
> **Fase actual:** 0 — Base técnica (en cierre)
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
| Fase | 0 — Base técnica |
| Versión de app | 0.1.0 |
| Esquema SQLite | v1 |
| Formato de exportación | v1 (contrato, implementación en Fase 1) |
| Frontend | React 19 + TypeScript + Vite 8 |
| Backend | Rust + Tauri 2.12 + `rusqlite` 0.41 (bundled) |
| Red en la webview | Prohibida por CSP y permisos |
| Updater | Desactivado (`UPDATER_IMPLEMENTED = false`) |
| Bóveda real | Aún no creada; `/omega` y `/loglife` no fueron tocados |
| Secretos | Ninguno generado, ninguno en el repo |
| Git | Repositorio local + remoto GitHub [`wulflo/study-planner`](https://github.com/wulflo/study-planner) (público); `main` con commit inicial `87f2be8` |
| CI | GitHub Actions verde en `main`; check requerido «Tests y verificación» |

### Verificación vigente

```text
npm run typecheck   ✅
npm run lint        ✅
npm test            ✅  2 tests
npm run build       ✅
cargo test -p omega-core -p omega-store -p omega-transfer   ✅  8 tests
cargo clippy (core/store/transfer) -- -D warnings           ✅
cargo check -p omega                                        ✅
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

1. **Sin red en la app.** La webview no usa `http`, `fetch`, CDNs, fuentes remotas ni íconos remotos. La única excepción futura es el crate `omega-updater`, aislado y sin bindings JS.
2. **Sin permisos nuevos sin justificación.** `src-tauri/capabilities/default.json` solo contiene `core:default`. Prohibido agregar `fs`, `http`, `sql`, `shell`, `opener`, `dialog`, `notification` o `updater` al frontend.
3. **Los archivos y la base los escribe Rust.** El frontend nunca recibe rutas ni handles de archivos.
4. **Nada de `flutter_secure_storage`, cuentas, telemetría ni nube.** Omega es estrictamente local.
5. **La bóveda no se cifra.** Decisión explícita del usuario. No reintroducir contraseña de bóveda, Argon2id, AES-GCM ni recovery key.
6. **No cambiar el esquema sin migración nueva.** `0001_initial.sql` es inmutable una vez publicado. Todo cambio va en `0002_*.sql` con checksum.
7. **No cambiar el formato de exportación sin subir `export_format_version`.** El round-trip `exportar → importar → exportar` debe ser byte a byte idéntico dentro de la misma versión.
8. **Nunca tocar `/omega` ni `/loglife` sin backup verificado y confirmación.** Las rutas se configuran una vez; no se cambian en silencio.
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
- [x] Renombrar app a `Omega`, identificador `app.omega.studyplan`.
- [x] Workspace Rust: `omega-core`, `omega-store`, `omega-transfer`, `omega-updater`, `src-tauri`.
- [x] Esquema SQLite v1 completo (`crates/omega-store/migrations/0001_initial.sql`).
- [x] `Store` con WAL, claves foráneas, `synchronous=FULL`, timeout, migración con checksum SHA-256.
- [x] `vault_id`, `schema_version`, `integrity_check`, `foreign_key_check`.
- [x] Comandos Tauri `get_app_info` y `schema_self_check` (base en memoria).
- [x] CSP de producción y desarrollo; `freezePrototype`; `withGlobalTauri: false`.
- [x] Capacidad mínima `core:default`; sin plugin `opener`.
- [x] UI shell con navegación, estado técnico, marca e ícono SVG local.
- [x] Tests de UI y de Rust; `typecheck`, `lint`, `test`, `build` verdes.
- [x] `docs/CHECKLIST.md` (este archivo).
- [x] Documentación base: `README.md`, arquitectura, modelo de datos, exportación, updater, claves, decisiones y modo offline.
- [x] `CHANGELOG.md` y `THIRD_PARTY_LICENSES.md`.
- [x] Scripts `scripts/check.sh` y `scripts/audit-network.sh`.
- [x] `.gitignore` endurecido (claves, bases, secretos, backups).
- [x] CI sin secretos en `.github/workflows/ci.yml`.
- [x] Repositorio git inicializado, commit inicial local y remoto GitHub configurado (`main`).
- [x] CI verde en GitHub y rama `main` protegida con el check «Tests y verificación».

### Pendiente para cerrar Fase 0

- [ ] Revisar con el usuario la licencia **MPL-2.0** transitiva de Tauri (`option-ext` en runtime; `cssparser`, `selectors`, `dtoa-short` solo en build). Ver `THIRD_PARTY_LICENSES.md`.
- [ ] Demo de Fase 0: abrir `npm run tauri dev`, verificar autodiagnóstico y navegación.
- [ ] Confirmación explícita del usuario para pasar a Fase 1.

### Criterio de aceptación de Fase 0

- La app abre en Tauri y muestra esquema v1, integridad `ok`, 0 violaciones de claves foráneas.
- `./scripts/check.sh` termina en verde.
- `./scripts/audit-network.sh` no encuentra red en la webview.
- No existe ninguna clave, base ni secreto versionado.
- El usuario confirma el cierre.

---

## Fase 1 — Bóveda, rutas, backups y transferencia

Objetivo: abrir/crear una bóveda real, respaldarla y mover todos los datos con un `.txt` canónico.

- [ ] Módulo de rutas: valores por defecto `/omega` (bóveda) y `/loglife` (backups), configurables en primer arranque.
- [ ] Validación de rutas: existen, son escribibles, no requieren `sudo`, no están dentro del repo.
- [ ] Asistente de primer arranque; nunca cambiar rutas en silencio.
- [ ] Apertura de bóveda con lock file y detección de otra instancia.
- [ ] Migración con backup previo obligatorio y restauración automática si falla.
- [ ] Backups con API de backup online de SQLite, checksum y verificación por `integrity_check`.
- [ ] Retención diaria/semanal/mensual según settings.
- [ ] Restauración desde backup con confirmación y backup del estado actual.
- [ ] Implementar exportador canónico v1 y importador.
- [ ] Adjuntos embebidos por defecto, con SHA-256 verificado.
- [ ] Importación en bóveda vacía; reemplazo total solo con backup + confirmación.
- [ ] Fixtures de exportación y test de round-trip byte a byte.
- [ ] Tests de checksum corrupto, versión futura, adjunto faltante y backup interrumpido.
- [ ] Pantalla «Bóveda»: estado, rutas, backup manual, exportar, importar, restaurar.
- [ ] Registrar eventos de backup/export/import en el log local (sin datos sensibles innecesarios).

**Aceptación:** crear bóveda, escribir un registro de prueba, exportar, importar en bóveda vacía, exportar de nuevo y obtener bytes idénticos; restaurar un backup verificado.

---

## Fase 2 — Año académico, materias, horario y asistencia

- [ ] CRUD de año académico, términos, materias y horarios.
- [ ] Rotaciones semanales A/B, paridad y ciclos de días.
- [ ] Feriados y excepciones de clase (cancelada, movida, extra).
- [ ] Vista semanal y vista «Hoy».
- [ ] Registro de asistencia (presente, ausente, tarde, justificado).
- [ ] Porcentaje de asistencia y umbral mínimo por materia.
- [ ] Cálculo de próxima clase y clases solapadas.
- [ ] Tests de recurrencia, feriados, excepciones y cambios de horario.
- [ ] Exportación/importación de las nuevas entidades sin romper v1.

**Aceptación:** cargar un semestre completo, ver la semana correcta, registrar asistencia y detectar solapamientos.

---

## Fase 3 — Exámenes, notas, tareas y búsqueda

- [ ] CRUD de exámenes, temas y pesos.
- [ ] Escalas de nota, categorías y entradas; promedio ponderado y descarte del menor.
- [ ] CRUD de tareas, subtareas, etiquetas, prioridad, recurrencia y estados.
- [ ] Filtros guardados y búsqueda FTS local.
- [ ] Journal de undo con expiración.
- [ ] Vistas de calendario, exámenes, notas y tareas.
- [ ] Tests de cálculo de notas, pesos, recurrencia y búsqueda.
- [ ] Exportación/importación de las nuevas entidades sin romper v1.

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
- [ ] Implementar `omega-updater` con URL HTTPS fija en compilación.
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

- [ ] ¿Se acepta la dependencia transitiva **MPL-2.0** de Tauri o se busca una alternativa? (ver licencias).
- [ ] Confirmar URL HTTPS fija del updater. Tentativa: `https://github.com/wulflo/study-planner/releases/latest/download/latest.json`.
- [ ] ¿Firma de releases en CI o firma offline? Recomendado: offline.
- [ ] Custodia final de la clave y su passphrase (VeraCrypt, copia offline, Drive cifrado).
- [x] Repositorio publicado en GitHub: [`wulflo/study-planner`](https://github.com/wulflo/study-planner), público, sin datos ni secretos.
- [ ] Confirmar rutas definitivas `/omega` y `/loglife` o elegir otras.

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
cargo clippy -p omega-core -p omega-store -p omega-transfer --all-targets -- -D warnings
cargo test -p omega-core -p omega-store -p omega-transfer
cargo check -p omega

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
│   ├── lib/ipc.ts               contrato IPC tipado
│   └── styles.css               tokens Material 3
├── src-tauri/                   host Tauri
│   ├── src/lib.rs               comandos y builder
│   ├── tauri.conf.json          CSP, ventana, bundle
│   └── capabilities/default.json permisos mínimos
├── crates/
│   ├── omega-core/              dominio puro, constantes, IDs
│   ├── omega-store/             SQLite, migraciones, backups
│   ├── omega-transfer/          exportación/importación canónica
│   └── omega-updater/           red aislada (Fase 5)
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
- Pendiente: revisión MPL, demo y confirmación para Fase 1.

---

## Próximos 5 pasos

1. Verificar el CI en GitHub (`main`) y la configuración del repositorio.
2. Hacer la demo de Fase 0 con `npm run tauri dev`.
3. Resolver la decisión sobre MPL-2.0 y las rutas `/omega` y `/loglife`.
4. Confirmar el cierre de Fase 0.
5. Iniciar Fase 1 por el módulo de rutas y el backup verificado, antes de la UI.
