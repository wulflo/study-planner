# Updater aislado

> **Estado:** implementado en `crates/study-planner-updater` y conectado a la vista «Ajustes».
> La comprobación es opt-in, la URL HTTPS es fija y la firma minisign es obligatoria.
> **Clave pública:** configurada en `crates/study-planner-updater/updater.pub` (2026-11-09).
> **Pendiente:** custodiar la passphrase/copias offline y publicar la primera release firmada.

El updater es la **única** excepción a la regla de «Study Planner no usa red». Debe ser pequeño, auditable, opt-in y separado del resto del código.

## Principios

1. **Sin bindings JS de red.** No se registra `tauri-plugin-updater` en la webview ni se concede permiso `updater:*`. La UI solo invoca comandos Tauri sin parámetros de red: no puede pasar URL, firma, hash ni ruta.
2. **URL fija en compilación.** El endpoint HTTPS es una constante de Rust. No se lee de settings, base de datos, variables de entorno ni argumentos.
3. **Firma obligatoria.** Un manifiesto o artefacto sin firma válida se rechaza. No hay modo «skip signature».
4. **Sin datos de usuario.** La petición solo lleva el manifiesto y un `User-Agent` con la versión de Study Planner. Nunca `vault_id`, rutas, materias, notas ni identificadores de máquina.
5. **Opt-in.** La comprobación automática está desactivada por defecto (`updater.auto_check = false`). El usuario puede activarla o buscar manualmente.
6. **Linux AppImage únicamente.** Tauri updater no soporta Android/iOS; Windows/macOS no son objetivo inicial. Bajo snap, el updater de AppImage se desactiva y actualiza snapd.
7. **Toda la red se registra** en `network_event` (tipo, host, bytes, resultado y detalle técnico acotado), sin contenido de datos.

## Diseño implementado

```text
vista «Ajustes»
  → comando Rust `get_updater_status`  (clave configurada, versión, AppImage, rollback)
  → comando Rust `set_updater_auto_check`  (opt-in persistido en `setting`)
  → comando Rust `check_updates`
       study-planner-updater::check_for_update_with_size()
       GET <URL fija> (HTTPS, timeout, host permitido, tamaño máximo)
       parsear manifiesto
       comparar versión (semver)
       devolver UpdateInfo a la UI (versión, fecha, notas)
       registrar network_event
  → usuario confirma
  → comando Rust `install_update`  (sin argumentos)
       volver a consultar el manifiesto fijo
       descargar AppImage con verificación de host y tamaño
       verificar SHA-256, firma minisign y `version:<versión>` firmada
       reemplazar el AppImage actual de forma atómica
       conservar el anterior como `.old`
       registrar network_event
       pedir reinicio
  → comando Rust `rollback_update`  (restaura `.old` y conserva el fallido como `.failed`)
  → comando Rust `list_network_events`  (auditoría de solo metadatos)
```

La UI solo ve `UpdateInfo` (versión, fecha, notas) y un estado. No ve URLs internas, firmas ni rutas temporales.

## Publicación

El AppImage se compila y firma en GitHub Actions (`.github/workflows/release.yml`).
`scripts/package-release.mjs` firma con `tauri signer sign --app-version <versión>`
—para que el trusted comment incluya `version:<versión>`—, renombra el artefacto,
calcula su SHA-256 y genera `latest.json` en `dist-release/`.

Antes de publicar, el workflow exige que `updater.pub` no sea el placeholder y
verifica la firma contra esa clave con:

```bash
cargo run -q -p study-planner-updater --example verify_release -- \
  <artefacto> <firma> <versión>
```

Detalle completo del pipeline, secrets y release offline en `docs/RELEASE.md`.

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
- `url` debe ser HTTPS y pertenecer a `github.com`, `objects.githubusercontent.com` o `release-assets.githubusercontent.com`.
- `pub_date` y `notes` son informativos.
- La firma se verifica con la clave pública embebida en el binario.
- El artefacto se verifica por SHA-256 **y** firma; el hash solo no alcanza.
- El trusted comment de la firma debe incluir `version:<versión>` y coincidir con el manifiesto.

Decisión cerrada: se firma el **artefacto** con minisign y el manifiesto incluye su SHA-256. La firma debe declarar la versión publicada; si falta o no coincide, la instalación se rechaza. El manifiesto en sí no se firma por separado: un manifiesto manipulado no puede producir un artefacto que pase la verificación.

## Custodia de claves

Ver `docs/KEYS.md`. Resumen:

- Clave privada minisign + passphrase, custodiadas fuera del repo.
- Clave pública embebida en el binario y versionada.
- Firma preferentemente **offline**; si se firma en CI, la clave va como secret y se acepta el riesgo.
- Rotación documentada y simulacro de recuperación.

## Endpoint

Constante inmutable, fijada al compilar:

```rust
pub const UPDATE_URL: &str =
    "https://github.com/wulflo/study-planner/releases/latest/download/latest.json";
pub const ALLOWED_HOSTS: [&str; 3] = [
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];
```

Reglas:

- No se permite `http://`.
- No se permite redirect a otro host (máximo 5 redirecciones).
- No se permite URL configurable en runtime.
- El host se valida antes de descargar el artefacto.
- La clave pública se embebe desde `crates/study-planner-updater/updater.pub`.

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

- El AppImage anterior se guarda como `Study Planner.AppImage.old` junto al actual.
- `rollback_update` mueve el binario fallido a `.failed`, restaura `.old` y lo marca ejecutable.
- Si el nuevo no arranca, el usuario puede restaurar el anterior desde «Ajustes» o manualmente.
- El updater no intenta «auto-repararse» descargando otra cosa.

## Ejecución como snap

Cuando Study Planner corre como snap, la variable `SNAP` está definida y el
updater de AppImage queda deshabilitado:

- `get_updater_status` devuelve `managedBySnap: true` y `runningAsAppImage: false`.
- `set_updater_auto_check`, `check_updates`, `install_update` y
  `rollback_update` rechazan la operación con `UpdateError::ManagedBySnap`.
- La comprobación automática al iniciar no se ejecuta.
- La vista «Ajustes» muestra el aviso «Esta instalación es un snap» y deshabilita
  los controles.

La razón es de seguridad y de modelo de distribución: dentro del confinamiento
estricto la app no puede reemplazar su propio binario, y snapd ya gestiona la
actualización desde la Snap Store. Si `APPIMAGE` y `SNAP` coexistieran, gana
`SNAP`. Ver [`docs/SNAP.md`](SNAP.md).

## Tests de Fase 5

- [x] Manifiesto válido y firmado: detecta versión nueva.
- [x] Firma inválida: rechazo.
- [x] Hash inválido: rechazo sin instalar.
- [x] Versión menor o igual: no instala.
- [x] URL `http://`, host ajeno, credenciales embebidas y fragmentos: rechazo.
- [x] Redirección a otro host: rechazo.
- [x] `network_event` registra el intento sin datos de usuario.
- [x] El binario actual queda intacto ante cualquier fallo.
- [x] Instalación atómica y rollback con tests de sistema de archivos.
- [x] Detección de Snap: tiene prioridad sobre AppImage y bloquea el updater.
- [ ] Simulación con una release real firmada (requiere clave de producción y publicación).

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
