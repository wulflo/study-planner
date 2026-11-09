# Política de privacidad de Study Planner

**Última actualización:** 2026-11-10

Study Planner es una aplicación de organización académica **estrictamente local**.
Esta política describe qué datos maneja la aplicación, dónde viven y qué se
envía (y qué no) por red.

## Resumen

- No hay cuentas de usuario ni inicio de sesión.
- No hay telemetría, analítica ni publicidad.
- No se recopilan datos personales ni académicos.
- Todas tus materias, horarios, exámenes, notas, tareas y apuntes viven en la
  bóveda local que elegís.
- La única comunicación de red opcional es el updater aislado, que consulta un
  manifiesto público y no envía datos de la bóveda.

## Datos que se guardan

Los únicos datos que la aplicación escribe son los que vos cargás:

- la estructura académica (años, términos, materias, horarios, rotaciones,
  feriados y asistencias);
- exámenes, temas, notas y categorías de calificación;
- tareas, etiquetas, filtros guardados y sesiones de estudio;
- repasos espaciados y autoevaluaciones;
- adjuntos que elijas asociar;
- copias de seguridad y snapshots de la base SQLite;
- preferencias locales (tema, color de acento, modo segundo plano y activación
  de recordatorios).

Todo se guarda en la carpeta de la bóveda que elegís al configurar la app, en
`<bóveda>/omega` (datos vivos y backup principal) y `<bóveda>/longlife`
(snapshots diarias). La aplicación no elige una ubicación oculta ni envía esa
ruta a ningún servidor.

## Datos que no se recopilan

- No se accede a contactos, ubicación, cámara, micrófono ni portapapeles.
- No se usan identificadores de publicidad ni de dispositivo.
- No se guardan contraseñas ni credenciales.
- No se construye ningún perfil de uso.
- No se comparten datos con terceros.

## Red y actualizaciones

La interfaz web no tiene permisos de red: no puede hacer solicitudes HTTP, leer
archivos ni ejecutar comandos. El único módulo que puede hablar con la red es el
updater aislado en Rust, y es **opt-in**:

- consulta una URL fija en GitHub (`latest.json`);
- envía únicamente un `User-Agent` con la versión de Study Planner;
- no envía el identificador de la bóveda, rutas, materias, notas ni ningún otro
  dato de tu contenido;
- puede desactivarse por completo; si se desactiva, no hace ninguna petición.

En la versión instalada desde Microsoft Store (MSIX) el updater está oculto y
deshabilitado: las actualizaciones las entrega la propia Store y la aplicación no
consulta ningún manifiesto.

Las notificaciones de clases, tareas, exámenes y repasos se generan y entregan
localmente por D-Bus. No salen del equipo.

## Exportaciones

Podés exportar toda la bóveda en un único `.txt` canónico. Ese archivo contiene
tus datos en texto plano (incluidos los adjuntos en base64) y es tu
responsabilidad custodiarlo. La aplicación no lo sube a ningún servicio.

## Seguridad de la bóveda

La bóveda no está cifrada por decisión explícita del proyecto. La confidencialidad
depende del cifrado de disco del sistema y de la custodia física del equipo. Si
necesitás confidencialidad, activá el cifrado de disco completo.

## Cambios en esta política

Si esta política cambia, se actualizará la fecha de la última actualización y el
cambio quedará registrado en el historial del repositorio. La versión vigente
siempre está en [`PRIVACY.md`](PRIVACY.md).

## Contacto

Si tenés preguntas sobre privacidad, abrí un issue en el repositorio público.
**No** reportes vulnerabilidades de seguridad en issues públicos: seguí
[`SECURITY.md`](SECURITY.md).
