# Changelog

Todos los cambios relevantes de Study Planner se registran acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado pretende seguir [SemVer](https://semver.org/lang/es/).

## [No publicado]

### Agregado

- Updater aislado en `study-planner-updater`: manifiesto JSON, versión semver, SHA-256, firma minisign obligatoria con `version:<versión>` en el trusted comment, hosts HTTPS permitidos y redirects limitados.
- Descarga con `reqwest` bloqueante y rustls solo dentro del crate del updater; verificación antes de instalar, reemplazo atómico del AppImage, backup `.old` y rollback con el binario fallido en `.failed`.
- Comandos Tauri del updater sin parámetros de red: estado, ajuste opt-in, comprobación, instalación, rollback y listado de eventos. La webview nunca recibe URL, firma ni ruta configurable.
- Vista «Ajustes» con estado del updater, clave pública, activación opt-in, búsqueda e instalación de actualizaciones, rollback y auditoría de eventos de red.
- Registro `network_event` con tipo, host, bytes, resultado y detalle técnico acotado; sin datos de la bóveda ni identificadores de usuario.
- Notificaciones locales sin plugin ni permisos de webview: cálculo determinista de recordatorios de clases, tareas, exámenes y repasos en el store, deduplicación con `dedupe_key` única, marcado `fired` y liberación para reintento.
- Planificador de recordatorios cada 60 s y entrega por D-Bus (`org.freedesktop.Notifications`) desde Rust; vista «Ajustes» con activación, anticipaciones visibles, revisión manual, notificación de prueba y últimos avisos entregados.
- Ajustes portables `get_setting` / `set_setting` en el store y persistencia de `updater.auto_check`.
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
- Estructura académica local: años, términos (semestre, trimestre, cuatrimestre, bimestre, anual u otro) y materias con color, docente, aula, créditos y umbral de asistencia, con edición y borrado lógico.
- Horario semanal con franjas por materia, día y hora, recurrencia semanal o bisemanal, vigencia y clave de rotación.
- Rotaciones configurables: semanal A/B, paridad de semanas ISO y ciclo de días hábiles, con ancla y etiquetas.
- Feriados por término y excepciones por clase: cancelada, movida (nueva hora y/o aula) o extra.
- Registro de asistencia por clase y fecha (presente, tarde, justificado, ausente), porcentaje en puntos básicos y umbral mínimo por materia; justificar no penaliza.
- Vista «Horario» con grilla semanal navegable, solapamientos detectados y alta/edición de franjas, rotaciones, feriados y excepciones.
- Vista «Hoy» con la próxima clase y las clases del día, registro directo de asistencia y aviso cuando una materia cae bajo el umbral.
- Los años, términos, materias, franjas, rotaciones, feriados, excepciones y asistencias ya viajan en la exportación canónica v1; un test verifica su semántica y el round-trip byte a byte.
- Exámenes y temas jerárquicos con fecha, horario, aula, peso, resultado y vínculo examen–tema; la sección «Exámenes» permite crear, editar, eliminar y asociar temas.
- Escalas de nota, categorías y entradas con centipuntos y basis points; promedio ponderado determinista, descarte del menor y mapeo a la escala, visibles en la sección «Notas».
- Tareas avanzadas con subtareas, etiquetas, prioridad, importancia, estimación, vencimientos, recurrencia diaria/semanal/mensual y estados `pending` / `in_progress` / `done`.
- Búsqueda FTS local por texto y etiquetas, filtros por estado, prioridad, etiqueta y rango de fechas, y filtros guardados por pantalla.
- Journal de undo para eliminación de tareas con expiración de 5 minutos y restauración desde la UI.
- Vista «Calendario» mensual con exámenes y vencimientos de tareas, navegación por mes y vuelta a «Hoy».
- Las entidades de Fase 3 viajan en la exportación canónica v1; un fixture verifica tablas, valores, cálculo de promedio importado y round-trip byte a byte.
- Autoevaluación de temas de 1 a 5, clasificación automática fortaleza/neutral/debilidad por umbrales configurables y override manual con motivo.
- Repaso espaciado SM-2 versionado: factor de facilidad en centésimas con mínimo 1.30, intervalos de 1, 6 y `round(intervalo × EF / 100)` días, reinicio y conteo de lapsos cuando la calidad es menor que 3.
- Cola de prioridad determinista y explicable: puntaje por clasificación, repaso vencido o próximo y examen próximo, con motivos visibles y desempate por tema.
- Sesiones de estudio Pomodoro, libres y de repaso con minutos planificados, minutos reales e interrupciones; la sesión puede asociarse a materia y tema.
- Vista «Estudio» con Pomodoro, cola de prioridad explicable e historial de sesiones; vista «Fortalezas» con autoevaluación, override y programación del repaso; vista «Estadísticas» con tiempo total, rachas, minutos por materia, materias débiles y próximos repasos.
- Las entidades de Fase 4 (`self_assessment`, `strength_override`, `review_item`, `study_session`) viajan en la exportación canónica v1; un fixture verifica su semántica y el round-trip byte a byte.
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
- Las secciones «Materias», «Horario», «Calendario», «Tareas», «Exámenes», «Notas», «Estudio», «Fortalezas», «Estadísticas», «Ajustes» y «Hoy» usan datos reales de la bóveda; ya no queda ningún placeholder de navegación.
- La vista «Hoy» ganó el bloque de horario y asistencia por encima de los pendientes, sin agregar dependencias nuevas ni permisos de red.
- El esquema v1 ya contenía las tablas de las fases 2, 3 y 4; no hubo migración nueva y `export_format_version` sigue en 1.
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
