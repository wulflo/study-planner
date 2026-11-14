# Changelog

Todos los cambios relevantes de Study Planner se registran acá. El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado pretende seguir [SemVer](https://semver.org/lang/es/).

## [0.1.5] — 2026-11-09 — Identidad de Microsoft Store

### Corregido

- El manifiesto MSIX ahora usa la identidad real de Partner Center: `Identity Name` `wulflo.StudyPlannerMax` y `PublisherDisplayName` `wulflo`. Antes generaba `wulfo` y la Store rechazaba el paquete.
- El workflow `store-msix` verifica los cuatro campos de identidad y el `AppxManifest.xml` generado antes de empaquetar.

### Cambiado

- Workflows endurecidos: checkout sin credenciales persistentes, `upload-artifact` v7, `download-artifact` v8 y acciones de Snap fijadas a `v1.3.0`/`v1.2.0`.

## [0.1.4] — 2026-11-09 — MSIX, instancia única y limpieza de interfaz

### Agregado

- Empaquetado MSIX para Windows y Microsoft Store en un workflow independiente (`.github/workflows/store-msix.yml`), con el mismo patrón que better-tracker y SinNotas: `scripts/prepare-store-msix.mjs` prepara el payload y el manifiesto desde `packaging/windows-store/AppxManifest.template.xml`, y `MakeAppx.exe` deja el `.msix` sin firmar como artefacto, listo para subir al Partner Center. El workflow de release ya no genera MSIX. El manifiesto usa la identidad reservada `wulflo.StudyPlannerMax` (`Publisher` `CN=7F51EF66-8407-4798-A8F4-3C4A407DA359`, `PublisherDisplayName` `wulflo`) y el nombre visible `Study Planner Max`. Microsoft Store aplica su identidad y firma al publicar.
- Detecta la instalación MSIX por la ruta `WindowsApps`, expone el runtime `msix` y oculta el panel del updater con el aviso de que se actualiza desde Microsoft Store.
- `PRIVACY.md` con la política de privacidad requerida por Microsoft Store; el workflow de la Store falla si falta.
- Botón «Visitar repositorio» en «Ajustes». Abre la URL fija del proyecto con el lanzador del sistema desde Rust (`xdg-open`, `open` o `rundll32`), sin plugins ni permisos nuevos en la webview.
- Instancia única: si la app ya está corriendo en segundo plano, abrirla de nuevo enfoca la ventana existente en lugar de iniciar otro proceso.

### Corregido

- Instancia única reforzada: además del nombre D-Bus del plugin, un lock de archivo y un socket local en el directorio de datos evitan que una segunda apertura cree otra ventana y otro icono en la bandeja cuando el plugin no puede publicar su nombre (snap `strict`, contenedores o sesiones sin bus). Si el aviso no llega, la segunda instancia se cierra igual.
- El icono de bandeja es idempotente: si ya existe, no se agrega un segundo icono.
- «Buscar actualizaciones» e «Instalar actualización» ya no bloquean la interfaz: la comprobación, descarga e instalación corren fuera del hilo principal.

### Eliminado

- Paneles de autodiagnóstico, checklist de proyecto, «Ver checklist» y «Siguiente» de la interfaz de desarrollo, junto con la tarjeta de auditoría de eventos de red en «Ajustes».

## [0.1.3] — 2026-11-09 — Updater multiplataforma y detección de runtime

### Corregido

- El estado del updater ahora informa correctamente cuando la app corre desde un AppImage. El backend serializaba `runningAsAppimage` (con «i» minúscula) mientras la interfaz leía `runningAsAppImage`, así que el botón de instalar quedaba deshabilitado y la vista mostraba «runtime desarrollo».
- La detección del AppImage usa `APPIMAGE`, `ARGV0` y el ejecutable actual, y Snap tiene prioridad para no intentar reemplazar archivos gestionados por snapd.

### Agregado

- Campo `runtime` en el estado del updater: `appimage`, `snap`, `deb`, `rpm`, `windows`, `macos` o `development`.
- Detección de paquetes Linux mediante `dpkg-query -S` y `rpm -qf` sobre el ejecutable actual.
- Actualización de `.deb` con `pkexec dpkg -i` y de `.rpm` con `pkexec dnf/zypper/rpm`, siempre después de verificar la firma minisign.
- Actualización de Windows ejecutando el instalador NSIS con `/S` o MSI con `msiexec /qn`, y cierre de la app para liberar los archivos.
- Actualización de macOS montando el `.dmg`, reemplazando el bundle `.app` y reabriéndolo.
- Manifiesto multi-plataforma: `linux-<arch>`, `linux-<arch>-deb`, `linux-<arch>-rpm`, `windows-<arch>`, `windows-<arch>-msi` y `darwin-<arch>`.
- `scripts/merge-manifests.mjs` fusiona los manifiestos parciales de cada job de release en `latest.json`.
- La vista «Ajustes» muestra el runtime detectado y explica si se pedirá contraseña de administrador o si la app se reiniciará.

## [0.1.2] — 2026-11-08 — Corrección de la versión en la barra superior

### Corregido

- La barra superior ya no muestra `v0.1.0` mientras carga la información de la app. Ahora deja el espacio vacío hasta recibir la versión real, así una instalación actualizada no parece seguir en una versión anterior.

## [0.1.1] — 2026-11-08 — Temas, segundo plano y updater

### Agregado

- Apariencia configurable en «Ajustes»: modo claro, oscuro o sistema y ocho colores de acento (violeta, azul, verde azulado, verde, ámbar, naranja, rosa y rojo) que tiñen fondos, superficies y controles. La preferencia se guarda en `localStorage`, así que funciona antes de configurar la bóveda.
- Modo segundo plano con icono de bandeja: mostrar, revisar recordatorios y salir. Cerrar la ventana la oculta en lugar de terminar la app cuando la bandeja está disponible.
- Sonido opcional en los recordatorios mediante el hint `sound-name` de `org.freedesktop.Notifications`; se puede activar o desactivar desde «Ajustes» y se aplica también a la notificación de prueba.
- Categoría `Education` en el bundle de Linux para que el menú de aplicaciones clasifique Study Planner correctamente.
- Updater aislado en `study-planner-updater`: manifiesto JSON, versión semver, SHA-256, firma minisign obligatoria con `version:<versión>` en el trusted comment, hosts HTTPS permitidos y redirects limitados.
- Descarga con `reqwest` bloqueante y rustls solo dentro del crate del updater; verificación antes de instalar, reemplazo atómico del AppImage, backup `.old` y rollback con el binario fallido en `.failed`.
- Comandos Tauri del updater sin parámetros de red: estado, ajuste opt-in, comprobación, instalación, rollback y listado de eventos. La webview nunca recibe URL, firma ni ruta configurable.
- Vista «Ajustes» con estado del updater, clave pública, activación opt-in, búsqueda e instalación de actualizaciones, rollback y auditoría de eventos de red.
- Registro `network_event` con tipo, host, bytes, resultado y detalle técnico acotado; sin datos de la bóveda ni identificadores de usuario.
- Notificaciones locales sin plugin ni permisos de webview: cálculo determinista de recordatorios de clases, tareas, exámenes y repasos en el store, deduplicación con `dedupe_key` única, marcado `fired` y liberación para reintento.
- Planificador de recordatorios cada 60 s y entrega por D-Bus (`org.freedesktop.Notifications`) desde Rust; vista «Ajustes» con activación, anticipaciones visibles, revisión manual, notificación de prueba y últimos avisos entregados.
- Ajustes portables `get_setting` / `set_setting` en el store y persistencia de `updater.auto_check`.
- Empaquetado AppImage verificado localmente y pipeline de release en GitHub Actions: build, firma con `--app-version`, generación de `latest.json`, verificación contra la clave pública embebida y publicación de la release latest.
- Release multiplataforma en `.github/workflows/release.yml`: matriz Linux (`AppImage`, `.deb`, `.rpm`), Windows (`.exe` NSIS, `.msi`) y macOS (`.dmg`), con firma minisign de cada artefacto y `SHA256SUMS` por plataforma.
- `scripts/package-artifacts.mjs` para firmar y preparar `.deb`, `.rpm`, `.exe`, `.msi` y `.dmg`; `scripts/package-release.mjs` sigue generando el AppImage y `latest.json`.
- `zbus` pasó a dependencia solo de Linux; las notificaciones D-Bus se mantienen en Linux y devuelven un error claro en otras plataformas. Windows/macOS son builds experimentales, sin notificaciones ni updater.
- Herramienta `verify_release` para validar artefacto, firma y versión sin red.
- CI actualizado a los crates reales (`study-planner-*`), con updater, host y auditoría de red.
- Clave pública minisign de producción configurada en `crates/study-planner-updater/updater.pub`; la clave privada y la passphrase se custodian fuera del repositorio.
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
- Empaquetado Snap `study-planner` sobre `core24` con confinamiento `strict`, extensión `gnome` y plugs `home`/`removable-media`; sin plugs de red.
- Workflow `.github/workflows/snap.yml` que compila el `.snap` en GitHub Actions, lo sube como artefacto y, al lanzarlo a mano, lo publica en el canal elegido (`stable` por defecto).
- Workflow `.github/workflows/snap-promote.yml` para promover una revisión ya subida a `stable`, `beta` o `edge` sin recompilar.
- Snap `study-planner` 0.1.0 (revisión 1) liberado en `stable`; ficha pública pendiente de la revisión inicial de Canonical.
- `scripts/check-snap-version.sh` para exigir que la versión del snap coincida con la de la app; `scripts/check.sh` y `scripts/audit-network.sh` validan confinamiento, plugs y ausencia de red.
- Guía `docs/SNAP.md` con build, credenciales restringidas a `stable`, `beta` y `edge`, instalación local, conexión de `removable-media` y pasos de publicación.

### Cambiado

- Bajo snap (`SNAP` definido), el updater de AppImage se deshabilita: estado `managedBySnap`, guardas en los comandos, sin auto-check y controles deshabilitados en «Ajustes». Las actualizaciones las gestiona snapd.
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

### Corregido

- El updater ya no depende de que haya una bóveda abierta: el estado y la comprobación funcionan sin ella y el registro de red es best-effort. La detección de AppImage usa `APPIMAGE`, `ARGV0` y el ejecutable actual, por lo que el botón de instalar deja de aparecer bloqueado en un AppImage publicado.
- La ventana maximizada ya no deja padding lateral de más: la barra lateral queda pegada al borde y el contenido principal se centra con un ancho máximo propio.
- En ventanas angostas la barra lateral colapsa a una franja de íconos (y a una barra horizontal en anchos muy reducidos), con etiquetas accesibles y tooltips.
- El redimensionado de la ventana sin decoración dejó de acumular artefactos grises: se quitaron los `backdrop-filter`, los fondos pasaron a ser sólidos y los bordes de redimensionado se ocultan al maximizar.
- CI: `cargo fetch --locked` antes de las auditorías para que `cargo metadata --offline` tenga también las dependencias de otros targets (fallaba con `hermit-abi`).
- Release: el job de macOS usa `macos-15`; `macos-14` ya no está disponible en GitHub Actions.
- Snap: el toolchain de Rust se instala con `rustup-init` oficial porque el snap `rustup/latest` (core26) choca con la libc del SDK gnome-46 (core24), y el `.deb` se toma del `target/` del workspace (no de `src-tauri/target/`).
- Dependabot: se ignoran las actualizaciones de `glib < 0.20.0`, imposibles de resolver mientras Tauri use gtk 0.18, para que el workflow no falle.

### Conocido

- Dependabot reporta una debilidad media en `glib` 0.18.5 (iteradores de `VariantStrIter`). Llega de forma transitiva por `gtk 0.18` en la pila de Tauri; la corrección requiere `glib 0.20` y una actualización mayor de Tauri/gtk-rs. Study Planner no usa esos iteradores directamente. Ver `docs/CHECKLIST.md`.
- La restauración desde la UI, la exportación/importación canónica y los adjuntos ya están implementados; el modo de reemplazo total desde la UI sigue fuera de alcance.

### Seguridad

- La raíz de la bóveda se valida antes de escribir; se rechazan archivos, rutas inexistentes, la raíz del sistema y cualquier carpeta dentro del repositorio.
- El snap `study-planner` es `strict` y no declara plugs de red; el updater de AppImage se rechaza en runtime cuando corre como snap.
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
- Repositorio remoto configurado en [`wulflo/StudyPlanner`](https://github.com/wulflo/StudyPlanner); `main` con commit inicial.
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
- El repositorio es local y remoto: [`wulflo/StudyPlanner`](https://github.com/wulflo/StudyPlanner) es público; no hay releases todavía.
- La demo de Fase 0 y la confirmación del usuario quedaron cerradas el 2026-11-08 (ver [No publicado]).

## [0.0.0] — 2026-11-08

- Punto de partida del proyecto. Sin código previo.
