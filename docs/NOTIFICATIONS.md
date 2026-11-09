# Notificaciones locales

> **Estado:** implementado. El cálculo vive en `study-planner-store` y la entrega
> en el host Tauri. No hay plugin de notificaciones, no hay permiso de webview y
> no hay red.

## Principios

1. **Sin plugin ni permisos de webview.** No se registra `tauri-plugin-notification`
   ni se concede `notification:*`. La UI solo invoca comandos Tauri y nunca recibe
   permisos del sistema.
2. **Cálculo en la bóveda.** `Store::due_reminders()` decide qué avisos corresponden
   según la hora local y los ajustes portables. Es determinista para una hora dada.
3. **Entrega directa por D-Bus.** El host usa `org.freedesktop.Notifications`
   (`Notify`) desde Rust. Si el escritorio no está disponible, el aviso se libera
   y se reintenta en la próxima pasada.
4. **Deduplicación persistente.** Cada recordatorio se reclama insertando su
   `dedupe_key` única en la tabla `notification`; luego se marca `fired`. Nunca se
   entrega dos veces el mismo aviso.
5. **Sin datos fuera del equipo.** El cuerpo del aviso muestra solo lo necesario
   (materia, título, fecha, aula). Nada se envía a la red.
6. **Sonido opcional.** Con `notifications.sound = true` se envía el hint
   `sound-name = message-new-instant`; el escritorio decide qué sonido reproducir.
   Si el sistema no tiene sonidos configurados, el aviso se muestra igual.

## Tipos de recordatorio

| Tipo | Condición | Clave por defecto |
|---|---|---|
| Clase | Empieza dentro de la anticipación configurada | `notifications.class_lead_minutes` = 10 min |
| Tarea | Vence dentro de la anticipación, salvo tareas `done` | `notifications.task_lead_minutes` = 60 min |
| Examen | Empieza dentro de la anticipación y sigue `pending` | `notifications.exam_lead_hours` = 24 h |
| Repaso | `review_item.due_date` ya venció | SM-2, sin anticipación |

Los ajustes se guardan en `setting` con alcance `portable` y viajan en la
exportación canónica v1. `notifications.enabled` existe desde el esquema v1; no
hubo migración. `notifications.sound` se agregó como ajuste portable con valor
por defecto `true`.

## Flujo

```text
planificador (cada 60 s, solo con la app abierta o en segundo plano y notificaciones activas)
  → Store::due_reminders()
  → por cada recordatorio: claim_notification() (INSERT ... ON CONFLICT DO NOTHING)
  → send_notification() por D-Bus (con `sound-name` si el sonido está activo)
       éxito → mark_notification_fired()
       fallo → release_notification() y reintento en la próxima pasada
```

La vista «Ajustes» permite activar/desactivar, ver las anticipaciones, forzar una
revisión (`run_notification_scan`), enviar una notificación de prueba
(`send_test_notification`) y consultar los últimos avisos entregados.

## Segundo plano

Con «Mantener en segundo plano» activo (por defecto), cerrar la ventana la oculta
en la bandeja del sistema si el escritorio ofrece una. El icono permite mostrar
la app, revisar recordatorios ahora o salir del todo. El planificador sigue
corriendo cada 60 s mientras el proceso vive.

Si el escritorio no permite crear la bandeja, la app lo informa en «Ajustes» y
cerrar la ventana termina el proceso, igual que sin modo segundo plano. La
preferencia se guarda en la bóveda cuando hay una; sin bóveda funciona durante
la sesión actual.

## Límites conocidos

- Solo Linux/escritorio con bus de sesión D-Bus; en otros entornos la entrega falla
  de forma controlada y no rompe la app.
- El planificador no corre con la app cerrada; no se instala un servicio del sistema.
- La tabla retiene los avisos entregados; no hay purga automática todavía.
- Cambiar las anticipaciones requiere editar el ajuste en la bóveda; la UI solo
  muestra los valores actuales.
