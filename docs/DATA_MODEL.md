# Modelo de datos

Esquema v1 en `crates/study-planner-store/migrations/0001_initial.sql`. La migración es inmutable: cualquier cambio futuro va en `0002_*.sql` y sube `SCHEMA_VERSION`.

## Convenciones

| Concepto | Representación |
|---|---|
| Identificador | UUIDv7 en `TEXT`, ordenable por tiempo |
| Instante | UTC RFC3339 con milisegundos (`2026-11-08T12:34:56.789Z`) |
| Fecha civil | `YYYY-MM-DD` |
| Hora local | `HH:MM` |
| Peso | basis points enteros (`10000` = 100 %) |
| Nota | centipuntos enteros (`8500` = 85.00) |
| Booleano | `INTEGER` 0/1 con `CHECK` |
| Borrado | `deleted_at TEXT` NULL = activo |
| Texto | UTF-8; sin normalización implícita en la base |
| JSON | `TEXT` con JSON válido cuando el campo lo indica |

Reglas:

- Todas las tablas son `STRICT`.
- `created_at` y `updated_at` tienen default UTC; `updated_at` lo actualiza la capa de repositorios.
- Las claves foráneas usan `ON DELETE RESTRICT`: no se borra físicamente nada del dominio.
- Los índices únicos son parciales para permitir recrear una entidad borrada.
- Los agregados derivados (promedios, porcentajes) se calculan en Rust, no se almacenan salvo caché explícita.

## Grupos de tablas

### Metadatos y migraciones

| Tabla | Propósito |
|---|---|
| `app_meta` | `vault_id`, `schema_version`, `created_at` y metadatos estables |
| `schema_migration` | versión, nombre, checksum SHA-256 y fecha de aplicación |
| `setting` | preferencias `portable` (se exportan) y `machine` (no se exportan) |

`vault_id` se genera en la primera migración y nunca cambia. Identifica la bóveda en backups y exportaciones.

### Calendario académico

| Tabla | Propósito |
|---|---|
| `academic_year` | año académico con fecha de inicio y fin |
| `term` | semestre, trimestre, cuatrimestre, bimestre, anual u otro |
| `holiday` | feriados globales o por término |
| `rotation_config` | rotación semanal A/B, paridad o ciclo de días |

`term` y `academic_year` validan `start_date <= end_date`.

### Materias y horario

| Tabla | Propósito |
|---|---|
| `subject` | materia, docente, aula, créditos, contacto, meta de nota, asistencia mínima |
| `class_slot` | bloque semanal: día 1–7, hora inicio/fin, recurrencia, rotación, vigencia |
| `class_exception` | cancelación, mudanza o clase extra en una fecha |
| `attendance_record` | presente, ausente, tarde o justificado por bloque y fecha |

`class_slot` exige `start_time < end_time`. Una excepción única por `(class_slot_id, date, kind)` mientras no esté borrada. La asistencia es única por `(class_slot_id, date)`.

#### Semántica de rotaciones, excepciones y asistencia

- `weekly_ab` avanza por semana ISO desde `anchor_date`: el índice es la cantidad de semanas desde el lunes del ancla, módulo `cycle_length`.
- `parity` usa la misma aritmética de semanas ISO, pero con etiquetas `Par`/`Impar` por defecto (`cycle_length = 2`).
- `day_cycle` avanza un paso por día hábil (lunes a viernes) desde `anchor_date`; los fines de semana no consumen paso.
- Una franja `biweekly` se dicta solo en semanas pares contadas desde el ancla de la rotación o, si no hay rotación, desde `valid_from`.
- `valid_from` y `valid_to` acotan la vigencia de la franja; fuera de ese rango no genera ocurrencias.
- Un feriado cancela las ocurrencias de esa fecha. Una excepción `cancelled` elimina la clase, `moved` reemplaza hora y/o aula, y `extra` agrega una ocurrencia aunque no haya franja base.
- `attendance_record` es único por franja y fecha. `present`, `late` y `justified` cuentan como asistencia; `absent` no. El porcentaje se calcula en puntos básicos `(attended * 10000) / counted` y `justified` no penaliza.
- `subject.attendance_min_bp = NULL` desactiva el umbral; si está definido, la UI marca la materia cuando el porcentaje cae por debajo.
- La próxima clase se calcula desde fecha y hora locales, aplicando rotaciones, feriados y excepciones.

### Exámenes y temas

| Tabla | Propósito |
|---|---|
| `exam` | examen con fecha, horario, aula, peso y resultado |
| `topic` | tema jerárquico por materia (`parent_topic_id`) |
| `exam_topic` | relación examen–tema, única por par activo |

`weight_bp` va de 0 a 10000. `result_status` es `pending`, `passed`, `failed` o `absent`.

### Notas

| Tabla | Propósito |
|---|---|
| `grade_scale` | escala de nota (mínimo, máximo, aprobación, precisión) |
| `grade_category` | categoría ponderada por materia, con descarte del menor |
| `grade_entry` | nota individual, opcionalmente ligada a un examen |

`grade_entry` exige `max_cp > 0` y `0 <= score_cp <= max_cp`. `grade_category.weight_bp` va de 0 a 10000.

### Tareas y etiquetas

| Tabla | Propósito |
|---|---|
| `task` | tarea, práctica, proyecto o personal; jerarquía, fechas, prioridad, estado, recurrencia |
| `tag` | etiqueta con nombre único activo |
| `task_tag` | relación tarea–etiqueta única por par activo |
| `saved_filter` | filtro guardado por pantalla |
| `undo_journal` | journal de deshacer con expiración |

`task` valida `start_date <= due_date`, padre distinto de sí misma y estados `pending`, `in_progress`, `done`.

### Estudio y repaso

| Tabla | Propósito |
|---|---|
| `self_assessment` | autoevaluación 1–5 por tema, con fuente |
| `strength_override` | clasificación manual fortaleza/neutral/debilidad |
| `review_item` | estado SM-2 por tema: ease, intervalo, repeticiones, lapsos, vencimiento |
| `study_session` | sesión Pomodoro/libre/repaso con tiempos y pausas |

`review_item` usa `ease_factor_cp >= 130` y algoritmo `sm2` versionado para permitir migraciones futuras.

### Apuntes y adjuntos

| Tabla | Propósito |
|---|---|
| `note` | nota con título, cuerpo, fijado y vínculo a materia/tema |
| `attachment` | metadatos de adjunto: dueño, nombre, MIME, tamaño, SHA-256, almacenamiento |

`attachment.storage` es `vault` (archivo en la bóveda) o `embedded` (dentro de la exportación). El nombre es único por dueño mientras esté activo.

### Notificaciones y eventos técnicos

| Tabla | Propósito |
|---|---|
| `notification` | notificación programada, disparada, pospuesta o descartada, con `dedupe_key` único |
| `network_event` | auditoría de intentos de red del updater |

`network_event` nunca se exporta. `notification` tampoco en v1: se regenera a partir del calendario y las tareas.

### Búsqueda

| Tabla | Propósito |
|---|---|
| `task_fts` | FTS5 de tareas (título, descripción, etiquetas) |
| `note_fts` | FTS5 de apuntes (título, cuerpo) |

Son tablas derivadas: no se exportan y se reconstruyen al importar. `task_fts.task_id` y `note_fts.note_id` son `UNINDEXED` para recuperar la entidad.

## Settings iniciales

Portables (se exportan):

- `theme.mode`, `theme.accent`, `theme.variant`, `theme.contrast`
- `locale`
- `backup.retention.snapshots` (7 por dispositivo por defecto), `backup.interval_hours`
- `notifications.enabled` y anticipaciones
- `study.weak_threshold`, `study.strong_threshold`, `study.priority_decay_days`
- `updater.auto_check`, `updater.strict_offline`

De máquina (no se exportan):

- `paths.vault_root`, `paths.device_id`
- `backup.last_snapshot_at`
- `locale.timezone`
- `window.last_route`

`paths.vault_root` es la carpeta raíz elegida por el usuario. Las subcarpetas `omega/` y `longlife/` se derivan de ella; no se guardan como rutas independientes.

Los valores se guardan como JSON (`value_json`), no como texto libre.

## Invariantes que la UI debe respetar

1. No se puede borrar un año, término o materia con dependencias activas.
2. Una nota no puede superar su máximo ni ser negativa.
3. Un peso no puede salir de 0–10000.
4. Un horario no puede terminar antes de empezar.
5. No puede haber dos asistencias activas para el mismo bloque y fecha.
6. No puede haber dos excepciones activas del mismo tipo para el mismo bloque y fecha.
7. Un tema no puede ser padre de sí mismo (la jerarquía se valida en Rust además del `CHECK`).
8. Los adjuntos deben tener hash y tamaño verificados antes de registrarse.

## Migraciones

1. Crear `000N_descripcion.sql`.
2. Agregar la migración a la lista ordenada del `Store`.
3. Subir `SCHEMA_VERSION` en `study-planner-core`.
4. Backup obligatorio antes de aplicar en una bóveda real.
5. Test: base vacía → migrar; base vN → migrar; base vN+1 → rechazar; idempotencia.

Nunca editar una migración ya publicada. El checksum registrado detecta cambios accidentales.
