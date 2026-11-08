# Formato de exportación `.txt` — v1

Contrato del archivo canónico de Omega. La implementación completa vive en `crates/omega-transfer` y corresponde a la Fase 1; en Fase 0 solo existen las constantes y la validación de magia/versión.

## Objetivo

Un único archivo de texto, legible y determinista, que permita reconstruir **toda** la bóveda:

```text
exportar → importar en bóveda vacía → exportar  ===  bytes idénticos
```

La igualdad byte a byte se garantiza dentro de la misma versión de formato y esquema. Entre versiones se compara el bloque canónico, no el archivo completo.

## Estructura

```text
OMEGA-EXPORT
format-version: 1
schema-version: 1
app-version: 0.1.0
vault-id: 019abcde-1234-7abc-8def-0123456789ab
snapshot-at: 2026-11-08T08:00:00.000Z
attachments: embedded
body-sha256: 9f2c...64hex...
---
{"table":"academic_year","row":{"created_at":"...","deleted_at":null,"end_date":"...","id":"...","name":"...","start_date":"...","updated_at":"..."}}
{"table":"term","row":{...}}
...
{"table":"attachment_blob","row":{"attachment_id":"...","data_base64":"...","sha256":"..."}}
{"table":"setting","row":{"key":"locale","scope":"portable","updated_at":"...","value_json":"\"es\""}}
OMEGA-END
```

### Cabecera

- Codificación UTF-8 **sin BOM**.
- Saltos de línea `LF` (`\n`) únicamente. Se rechazan `CRLF`.
- Primera línea exacta: `OMEGA-EXPORT`.
- Luego, en este orden fijo, una línea `clave: valor` por campo:

| Campo | Regla |
|---|---|
| `format-version` | entero; en v1 es `1` |
| `schema-version` | entero; debe coincidir con el esquema soportado |
| `app-version` | versión que generó el archivo; informativa |
| `vault-id` | UUID de la bóveda; se preserva al importar |
| `snapshot-at` | máximo `updated_at` exportado, o `1970-01-01T00:00:00.000Z` si no hay filas |
| `attachments` | `embedded` en v1 |
| `body-sha256` | SHA-256 en hex minúscula del cuerpo canónico |

- Una línea exacta `---` separa cabecera y cuerpo.
- No se permiten campos desconocidos en v1; un lector v1 rechaza el archivo con un error claro.
- No se permiten comentarios, líneas vacías ni espacios al final.

### Cuerpo

- Una línea por registro, terminada en `\n`, incluida la última.
- Cada línea es un objeto JSON compacto:

```json
{"table":"<tabla>","row":{"<columna>":<valor>,...}}
```

- Claves de primer nivel exactamente `table` y `row`, en ese orden.
- Claves de `row` ordenadas lexicográficamente por byte (UTF-8), sin excepción.
- Sin espacios, tabulaciones ni saltos dentro del JSON.
- `null` explícito; nunca se omite una columna.
- Strings JSON estándar: se escapan `"`, `\` y caracteres de control; el resto se emite UTF-8 sin escapar.
- Solo enteros y strings; sin números flotantes, sin exponentes, sin `true`/`false` (los booleanos de SQLite son `0`/`1`).
- Se exportan **todas** las columnas de la tabla, incluidas `created_at`, `updated_at` y `deleted_at`.
- Las filas borradas lógicamente también se exportan.

### Terminador

- Última línea exacta: `OMEGA-END` seguida de `\n`.
- El cuerpo canónico son los bytes desde el primer `{` hasta el `\n` anterior a `OMEGA-END`, inclusive ese `\n`.
- `body-sha256` se calcula sobre esos bytes exactos, no sobre el archivo completo.

## Orden de tablas

Orden fijo, pensado para respetar dependencias:

```text
academic_year
term
holiday
rotation_config
subject
class_slot
class_exception
attendance_record
exam
topic
exam_topic
grade_scale
grade_category
grade_entry
task
tag
task_tag
saved_filter
self_assessment
strength_override
review_item
study_session
note
attachment
attachment_blob
setting
```

- Dentro de cada tabla, las filas se ordenan por `id` ascendente (comparación de strings).
- `setting` se ordena por `key`.
- `attachment_blob` se ordena por `attachment_id`.
- `topic` y `task` pueden tener referencias a un padre que aparece después; el importador usa `PRAGMA defer_foreign_keys = ON` y valida al hacer commit.

## Tablas excluidas

| Tabla | Motivo |
|---|---|
| `schema_migration` | metadato técnico; se regenera |
| `app_meta` | `vault_id`, `schema_version` y `created_at` van en la cabecera |
| `undo_journal` | estado efímero de la sesión |
| `notification` | se regenera desde calendario/tareas |
| `network_event` | auditoría local del updater |
| `task_fts`, `note_fts` | derivadas; se reconstruyen al importar |

Los settings `machine` (`paths.*`, `locale.timezone`, `window.last_route`) no se exportan. Los `portable` sí.

## Adjuntos

- v1 exporta **todos** los blobs, sin importar el valor de `attachment.storage`.
- Cada fila de `attachment_blob` contiene `attachment_id`, `sha256` y `data_base64`.
- `data_base64` es base64 estándar con padding, sin saltos de línea.
- El importador verifica que el SHA-256 del blob decodificado coincida con `sha256` antes de escribirlo.
- Los blobs se guardan en el almacén por hash de la bóveda (`attachments/<sha256>`), con permisos restrictivos.
- `attachment.storage` es un valor lógico que se preserva tal cual para no romper el round-trip. En v1 la exportación siempre es portable; un futuro modo `referenced` subirá `format-version` y dejará de garantizar la reconstrucción total.

## Importación

Pasos obligatorios, en este orden:

1. Leer bytes completos; rechazar BOM, `CRLF`, caracteres inválidos o tamaño absurdo.
2. Validar primera línea, cabecera, orden de campos y `---`.
3. Rechazar `format-version` desconocida o mayor que la soportada.
4. Exigir `schema-version` igual a `SCHEMA_VERSION`; si es menor, rechazar en v1 (la migración de archivos antiguos será una fase posterior).
5. Recalcular `body-sha256` y comparar en tiempo constante.
6. Parsear cada línea; validar tabla, columnas exactas, tipos, nulos, enums, longitudes y checks de dominio.
7. Verificar que el orden de tablas y de filas sea el canónico.
8. Importar solo en una bóveda **vacía**, salvo modo «reemplazar» explícito con backup verificado y confirmación.
9. Abrir transacción, `PRAGMA defer_foreign_keys = ON`, insertar en orden.
10. Fijar `app_meta.vault_id` al `vault-id` de la cabecera.
11. Validar `foreign_key_check` y `integrity_check` antes del commit.
12. Reconstruir `task_fts` y `note_fts`.
13. Si algo falla: rollback, no dejar archivos parciales y reportar la línea exacta del error.

La importación nunca ejecuta SQL del archivo: solo inserta valores validados en tablas conocidas.

## Exportación

1. Abrir una transacción de lectura para obtener una foto consistente.
2. Leer todas las tablas en el orden fijo.
3. Calcular `snapshot-at` como máximo `updated_at` de las filas exportadas.
4. Serializar con el escritor canónico (claves ordenadas, sin espacios, enteros).
5. Escribir a un archivo temporal `.partial` en el mismo directorio, con permisos `600`.
6. Calcular checksum, escribir cabecera + cuerpo + terminador.
7. `fsync`, cerrar y renombrar atómicamente al nombre final.
8. Si falla, borrar el `.partial`.

El archivo contiene **todos** los datos sin cifrar. La UI debe advertirlo antes de exportar.

## Ejemplo mínimo

```text
OMEGA-EXPORT
format-version: 1
schema-version: 1
app-version: 0.1.0
vault-id: 019abcde-1234-7abc-8def-0123456789ab
snapshot-at: 2026-11-08T08:00:00.000Z
attachments: embedded
body-sha256: 0000000000000000000000000000000000000000000000000000000000000000
---
{"table":"academic_year","row":{"created_at":"2026-11-08T07:00:00.000Z","deleted_at":null,"end_date":"2026-12-20","id":"019abcde-0000-7000-8000-000000000001","name":"2026","start_date":"2026-03-01","updated_at":"2026-11-08T08:00:00.000Z"}}
OMEGA-END
```

El checksum del ejemplo es ficticio; un archivo real debe tener el SHA-256 correcto.

## Compatibilidad y versionado

- `format-version` es independiente de `schema-version`.
- Cambios en columnas, tablas exportadas, orden, serialización o checksum suben `format-version`.
- Un lector rechaza versiones mayores; un lector nuevo puede aceptar versiones antiguas solo con una ruta de migración explícita y testeada.
- El test de oro es el round-trip byte a byte con fixtures versionados en `fixtures/export/`.

## Tests obligatorios (Fase 1)

- [ ] Exportar fixture vacío e importarlo: bytes idénticos.
- [ ] Exportar fixture con todas las tablas, adjuntos y borrados lógicos: bytes idénticos.
- [ ] Checksum alterado: rechazo sin escribir.
- [ ] `format-version` futura: rechazo claro.
- [ ] `schema-version` futura: rechazo claro.
- [ ] Columna faltante, columna extra o tipo incorrecto: rechazo con línea.
- [ ] FK inválida: rollback y bóveda intacta.
- [ ] Adjunto con hash incorrecto: rollback.
- [ ] Importación interrumpida: no queda bóveda a medio escribir.
- [ ] Exportación interrumpida: no queda archivo final corrupto.
