# Updater aislado

> **Estado:** diseño. No implementado. `crates/study-planner-updater` solo contiene `UPDATER_IMPLEMENTED = false` y no tiene dependencias de red.

El updater es la **única** excepción a la regla de «Study Planner no usa red». Debe ser pequeño, auditable, opt-in y separado del resto del código.

## Principios

1. **Sin bindings JS.** El frontend no puede iniciar, configurar ni consultar el updater. No se registra `tauri-plugin-updater` en la webview ni se concede permiso `updater:*`.
2. **URL fija en compilación.** El endpoint HTTPS es una constante de Rust. No se lee de settings, base de datos, variables de entorno ni argumentos.
3. **Firma obligatoria.** Un manifiesto sin firma válida se rechaza. No hay modo «skip signature».
4. **Sin datos de usuario.** La petición solo lleva el manifiesto y un `User-Agent` con la versión de Study Planner. Nunca `vault_id`, rutas, materias, notas ni identificadores de máquina.
5. **Opt-in.** La comprobación automática está desactivada por defecto (`updater.auto_check = false`). El usuario puede activarla o buscar manualmente.
6. **Linux AppImage únicamente.** Tauri updater no soporta Android/iOS; Windows/macOS no son objetivo inicial.
7. **Toda la red se registra** en `network_event` (host, bytes, resultado, error), sin contenido de datos.

## Diseño propuesto

```text
botón «Buscar actualización» (UI)
  → comando Rust `check_for_update`
  → study-planner-updater::check()
       GET <URL fija> (HTTPS, timeout, tamaño máximo)
       parsear manifiesto
       verificar firma minisign con clave pública embebida
       comparar versión (semver)
       devolver UpdateInfo a la UI (sin ejecutar nada)
  → usuario confirma
  → comando Rust `install_update`
       descargar AppImage a temp
       verificar SHA-256 y firma del artefacto
       reemplazar el AppImage actual de forma atómica
       conservar el anterior como respaldo
       registrar network_event
       pedir reinicio
```

La UI solo ve `UpdateInfo` (versión, fecha, notas) y un estado. No ve URLs internas ni rutas temporales.

## Manifiesto

Formato compatible con la convención de Tauri, pero verificado por nuestro código:

```json
{
  "version": "0.2.0",
  "pub_date": "2026-12-01T00:00:00Z",
  "notes": "Cambios de la versión",
  "platforms": {
    "linux-x86_64": {
      "url": "https://.../Study Planner_0.2.0_amd64.AppImage",
      "signature": "<firma minisign del artefacto o del manifiesto>",
      "sha256": "<hex>"
    }
  }
}
```

Reglas:

- `version` debe ser semver mayor que la actual; no se instalan downgrades salvo modo explícito de recuperación.
- `url` debe ser HTTPS y pertenecer al host permitido en compilación.
- `pub_date` y `notes` son informativos.
- La firma se verifica con la clave pública embebida en el binario.
- El artefacto se verifica por SHA-256 **y** firma; el hash solo no alcanza.

La decisión exacta entre firmar el manifiesto, el artefacto o ambos se cierra en Fase 5. Mínimo aceptable: firma del artefacto + hash del manifiesto dentro de un documento firmado.

## Custodia de claves

Ver `docs/KEYS.md`. Resumen:

- Clave privada minisign + passphrase, custodiadas fuera del repo.
- Clave pública embebida en el binario y versionada.
- Firma preferentemente **offline**; si se firma en CI, la clave va como secret y se acepta el riesgo.
- Rotación documentada y simulacro de recuperación.

## Endpoint

Placeholder hasta que el usuario decida el repositorio:

```rust
// Fase 5: constante inmutable, fijada al compilar.
// Repositorio tentativo; confirmar antes de la primera release.
const UPDATE_URL: &str =
    "https://github.com/wulflo/study-planner/releases/latest/download/latest.json";
```

Reglas:

- No se permite `http://`.
- No se permite redirect a otro host.
- No se permite URL configurable en runtime.
- El host se valida antes de descargar el artefacto.

## Manejo de fallos

| Caso | Comportamiento |
|---|---|
| Sin red | Error claro; no se reintenta en bucle |
| HTTP 4xx/5xx | Error; se registra; no se instala |
| JSON inválido | Error; no se instala |
| Firma inválida | Error de seguridad; se registra; no se instala |
| Hash inválido | Error; se borra el temporal; no se instala |
| Versión igual o menor | «Ya estás al día»; no se instala |
| Descarga interrumpida | Temporal borrado; AppImage actual intacto |
| Reemplazo falla | Se conserva el AppImage anterior |

Nunca se ejecuta un instalador descargado sin verificar. Nunca se borra el binario anterior antes de que el nuevo esté en su lugar.

## Rollback

- Guardar el AppImage anterior como `Study Planner.AppImage.old` (o en el directorio de datos).
- Si el nuevo no arranca, el usuario puede restaurar el anterior desde la UI de recuperación o manualmente.
- El updater no intenta «auto-repararse» descargando otra cosa.

## Tests de Fase 5

- [ ] Manifiesto válido y firmado: detecta versión nueva.
- [ ] Firma inválida: rechazo.
- [ ] Hash inválido: rechazo y limpieza.
- [ ] Versión menor: no instala.
- [ ] Timeout y sin red: error controlado.
- [ ] Redirección a otro host: rechazo.
- [ ] `network_event` registra el intento sin datos de usuario.
- [ ] El binario actual queda intacto ante cualquier fallo.

## Amenazas y mitigaciones

| Amenaza | Mitigación |
|---|---|
| Repositorio GitHub comprometido | Firma obligatoria; sin clave no hay update |
| Manifiesto manipulado | Firma + HTTPS + host fijo |
| Artefacto manipulado | SHA-256 + firma |
| Clave privada robada | Passphrase, custodia offline, rotación |
| Fuga de datos en la petición | Sin identificadores; solo versión |
| Actualización silenciosa | Opt-in y confirmación explícita |
| Downgrade malicioso | Rechazo de versiones menores |

## No objetivos

- Actualizaciones en Android/iOS.
- Actualizaciones delta.
- Canales beta/alpha.
- Telemetría de adopción.
- Instalación silenciosa sin confirmación.
