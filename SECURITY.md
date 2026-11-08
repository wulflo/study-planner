# Seguridad

Study Planner es una aplicación local que maneja datos académicos y, en el futuro, la clave de firma de sus actualizaciones. Si encontrás una vulnerabilidad, **no abras un issue público**.

## Cómo reportar

Usá el reporte privado de GitHub:

**https://github.com/wulflo/study-planner/security/advisories/new**

También podés contactar al mantenedor por los medios indicados en su perfil de GitHub. Incluí:

- Versión de Study Planner y sistema operativo.
- Pasos mínimos para reproducir.
- Impacto esperado.
- Si aplica, una prueba de concepto sin datos reales.

No incluyas en el reporte:

- Claves privadas, passphrases ni tokens.
- Contenido de una bóveda real.
- Datos personales de terceros.

## Alcance

Se consideran relevantes:

- Ejecución de código o comandos desde la webview.
- Requests de red no autorizados.
- Lectura/escritura de archivos fuera de la bóveda.
- Bypass de la verificación de firma del updater.
- Corrupción o pérdida de datos en migraciones, backups o exportación.
- Filtración de datos en logs, exportaciones o mensajes de error.
- Permisos de Tauri más amplios que los declarados (`core:default` + controles de ventana).

Quedan fuera de alcance:

- Ataques que requieren acceso físico o root al equipo.
- Ingeniería social.
- Vulnerabilidades en WebKitGTK/GTK no explotables desde Study Planner.
- Falta de cifrado de la bóveda: es una decisión explícita del proyecto (ver `docs/DECISIONS.md`).

## Versiones soportadas

| Versión | Soporte |
|---|---|
| 0.1.x (Fase 0) | Desarrollo; sin release pública todavía |
| Anteriores | No soportadas |

## Respuesta

El mantenedor intentará confirmar el reporte, evaluar el impacto y publicar un parche y un aviso. No hay SLA formal; es un proyecto personal.

## Divulgación

- Coordinada: se acuerda una fecha antes de publicar detalles.
- Se acredita al reportero si lo desea.
- No se divulgan datos de la bóveda ni secretos en el aviso.
