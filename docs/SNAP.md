# Empaquetado Snap

> **Estado:** empaquetado y publicación manual a la Snap Store implementados
> (2026-11-09). El nombre `study-planner` está registrado a la cuenta `wulflo`.
> El workflow compila el `.snap`, lo deja como artefacto y, al lanzarlo a mano, lo
> publica en el canal elegido (`stable` por defecto, `edge` para pruebas). El
> secreto `SNAPCRAFT_STORE_CREDENTIALS` actual solo permite `edge`: hay que rotarlo
> con los canales `stable,edge` antes de publicar en `stable`. Los builds de
> `pull_request` solo compilan. Falta probar la instalación desde la tienda en
> Ubuntu 24.04 limpio.

## Decisión

Study Planner se distribuye como snap `study-planner` con `base: core24` y
`confinement: strict`. La app mantiene su regla de «sin red»: el snap **no
declara** los plugs `network` ni `network-status`.

Bajo snap, las actualizaciones las gestiona snapd desde la Snap Store. El updater
de AppImage se deshabilita en runtime cuando existe la variable `SNAP`; ver
[`docs/UPDATER.md`](UPDATER.md).

## Confinamiento y permisos

| Elemento | Valor |
|---|---|
| Nombre | `study-planner` |
| Base | `core24` |
| Confinamiento | `strict` |
| Extensión | `gnome` (WebKitGTK/GTK, fuentes y temas del escritorio) |
| Plugs | `home`, `removable-media` |
| Red | Sin `network` ni `network-status` |
| Actualizaciones | snapd / Snap Store |

`home` permite abrir bóvedas dentro del directorio personal.
`removable-media` permite bóvedas en unidades externas, pero **no se conecta
automáticamente**; el usuario debe ejecutar:

```bash
sudo snap connect study-planner:removable-media
```

El snap no incluye la clave privada del updater ni ningún secreto. La clave
pública minisign solo se usa en el build del AppImage; en snap esa ruta queda
inactiva.

## Archivos

- `snap/snapcraft.yaml` — definición del paquete.
- `.snapcraftignore` — excluye artefactos de build, `node_modules`, `target` y
  binarios ya empaquetados.
- `snap/gui/study-planner.png` — ícono de la ficha de la Snap Store.
- `.github/workflows/snap.yml` — compilación en CI y publicación manual a `stable` o `edge`.
- `scripts/check-snap-version.sh` — exige que la versión del snap coincida con
  `src-tauri/tauri.conf.json`.

## Build local

El build se hace con Snapcraft 8 dentro de un contenedor para no ensuciar el
sistema. Desde la raíz del repo:

```bash
mkdir -p /tmp/opencode/snap-build
podman run --rm --userns=keep-id \
  -v "$PWD:/work:Z" \
  -v /tmp/opencode/snap-build:/work/build:Z \
  -w /work \
  ghcr.io/canonical/snapcraft:8_core24 \
  snapcraft pack --destructive-mode --output /work/build
```

El `.snap` queda en `/tmp/opencode/snap-build/`. Si preferís Snapcraft instalado
en el host, alcanza con `snapcraft pack` desde la raíz.

> No uses `snapcore/snapcraft:stable`: esa imagen trae Snapcraft 4.4.4 (2020) y no
> es compatible con `core24` ni con las credenciales actuales.

## CI: build y publicación manual

`.github/workflows/snap.yml` se dispara a mano (`workflow_dispatch`) o con cambios
en `snap/**`. Pasos:

1. Checkout.
2. `bash scripts/check-snap-version.sh`.
3. `snapcore/action-build@v1`.
4. Subida del `.snap` como artefacto `study-planner-snap`.
5. `snapcore/action-publish@v1` al canal elegido (`stable` por defecto, `edge`
   para pruebas), **solo** en `workflow_dispatch`.

Los builds de `pull_request` nunca publican: no reciben secretos y el paso de
publicación está condicionado al disparo manual. Para publicar una revisión hay
que lanzar el workflow «Snap» a mano desde la pestaña Actions y elegir el canal.

## Credenciales de Snapcraft

> **Estado actual:** nombre `study-planner` registrado a la cuenta `wulflo` y
> `SNAPCRAFT_STORE_CREDENTIALS` cargado en GitHub. El token actual está limitado a
> `study-planner` y al canal `edge`, y expira el 2027-11-08. Para publicar en
> `stable` hay que rotarlo con `--channels stable,edge` (comando abajo).

El token debe limitarse a `study-planner`, a los canales `stable` y `edge` y a los
ACLs de paquete. El registro del nombre y el login con contraseña + 2FA los hace
el usuario; el token se exporta a un archivo temporal y se guarda como secreto de
GitHub sin imprimirlo.

```bash
mkdir -p /tmp/opencode/snap
chmod 700 /tmp/opencode/snap

podman run --rm -it --userns=keep-id -e HOME=/work \
  -v /tmp/opencode/snap:/work:Z -w /work \
  --entrypoint bash \
  ghcr.io/canonical/snapcraft:8_core24 \
  -lc 'snapcraft register study-planner'
```

Si el nombre ya está registrado a la cuenta, el registro falla y se sigue igual.

> **Keyring:** las imágenes de Snapcraft 8.x usan el keyring del sistema y fallan
> en contenedores sin sesión de escritorio (`No keyring found`). Para registrar el
> nombre alcanza con `snapcore/snapcraft:stable` (4.4.4), que guarda la sesión en
> `~/.snapcraft`. El token restringido se sigue exportando con Snapcraft 8.x,
> porque `export-login` usa modo efímero y no necesita keyring.

```bash
podman run --rm -it --userns=keep-id -e HOME=/work \
  -v /tmp/opencode/snap:/work:Z -w /work \
  --entrypoint bash \
  ghcr.io/canonical/snapcraft:8_core24 \
  -lc 'snapcraft export-login \
    --snaps study-planner \
    --channels stable,edge \
    --acls package_access,package_push,package_update,package_release \
    --expires 2027-11-09T00:00:00Z \
    /work/snap-login.txt'
```

El token queda limitado a `study-planner` y a los canales `stable` y `edge`. El
workflow elige el canal al lanzarse; no hace falta ampliar el token para publicar
en `stable`.

Guardar en GitHub Secrets:

```bash
gh secret set SNAPCRAFT_STORE_CREDENTIALS < /tmp/opencode/snap/snap-login.txt
gh secret list | grep SNAPCRAFT_STORE_CREDENTIALS
```

`gh secret list` solo muestra el nombre y la fecha. Nunca se hace `cat` del token
ni se pega en el repo, issues o chat.

Limpiar los temporales:

```bash
shred -u /tmp/opencode/snap/snap-login.txt
rm -rf /tmp/opencode/snap
```

## Instalación y prueba

El snap es `strict`, así que la instalación local **no** lleva `--classic`:

```bash
sudo snap install --dangerous /tmp/opencode/snap-build/study-planner_*.snap
study-planner
```

Para bóvedas en `/run/media` o discos externos:

```bash
sudo snap connect study-planner:removable-media
```

Verificar el confinamiento:

```bash
snap info study-planner | grep -E 'confinement|channels'
snap connections study-planner
```

Probar en una máquina Ubuntu 24.04 limpia antes de publicar: arranque, apertura de
bóveda en `$HOME`, bóveda en unidad externa, exportación/importación y que
«Ajustes» muestre que snapd gestiona las actualizaciones.

## Publicación (pendiente)

Cuando el usuario lo pida:

1. Confirmar que `SNAPCRAFT_STORE_CREDENTIALS` está cargado.
2. Agregar `snapcore/action-publish@v2` (o el paso equivalente) al workflow.
3. Elegir el canal (`edge` para la primera prueba).
4. Verificar con `snap info study-planner` y `snap install study-planner --edge`.
5. Promover a `beta`/`stable` solo tras la prueba en limpio.

El token actual no permite publicar fuera de `edge`.

## Riesgos y límites

- **Rust en el build:** `snapcraft.yaml` instala el toolchain con el
  `rustup-init` oficial dentro de `override-build`. El snap `rustup/latest`
  apunta a core26 y choca con la libc del SDK gnome-46 (core24), que falla con
  `undefined symbol: __nptl_change_stack_perm`.
- **WebKitGTK:** la extensión `gnome` aporta WebKitGTK/GTK; el `layout` apunta a
  `$SNAP/usr/lib/$CRAFT_ARCH_TRIPLET_BUILD_FOR/webkit2gtk-4.1`. Hay que revisar el
  arranque real en cada actualización de base o de la extensión.
- **Sin red:** cualquier función futura que necesite red (por ejemplo, un canal de
  actualizaciones propio) requerirá un plug explícito y una revisión de la
  auditoría `scripts/audit-network.sh`.
- **`removable-media`:** no se auto-conecta; documentado arriba.
- **Tamaño:** el snap incluye WebKitGTK y dependencias de escritorio; es más grande
  que el AppImage. Es el costo de un entorno reproducible y actualizable por snapd.
- **Licencias:** Snapcraft es una herramienta de build GPL-3.0 y no se distribuye
  dentro del snap. WebKitGTK/GTK siguen siendo LGPL enlazadas dinámicamente; ver
  [`THIRD_PARTY_LICENSES.md`](../THIRD_PARTY_LICENSES.md).

## Verificación

`./scripts/check.sh` valida además:

- versión del snap sincronizada con la app (`check-snap-version.sh`);
- `confinement: strict`;
- ausencia de plugs de red;
- presencia de `removable-media`.

La auditoría está en `scripts/audit-network.sh`, sección «Snap».
