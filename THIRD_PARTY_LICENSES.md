# Licencias de terceros

> **Estado del inventario:** 2026-11-08, Fase 1 en curso.
> **Licencia del proyecto:** MIT OR Apache-2.0.

Study Planner no copia código GPL/AGPL. Se usan dependencias MIT, Apache-2.0, BSD, ISC, Zlib y Unicode, con las excepciones documentadas más abajo. Este archivo es un resumen; ante una distribución pública hay que conservar los textos de licencia completos de cada dependencia.

## Cómo regenerar el inventario

```bash
# npm: paquetes instalados y su licencia declarada
node -e '
const fs=require("fs"),path=require("path");
const lock=JSON.parse(fs.readFileSync("package-lock.json","utf8"));
for(const [p,m] of Object.entries(lock.packages)){
  if(!p.startsWith("node_modules/"))continue;
  let lic=m.license;
  if(!lic){try{lic=JSON.parse(fs.readFileSync(path.join(p,"package.json"),"utf8")).license}catch{}}
  console.log(p.replace(/^node_modules\//,"")+"\t"+(lic||"UNKNOWN"));
}' | sort

# Rust: grafo resuelto y licencias declaradas
cargo metadata --format-version 1 \
  | jq -r '.packages[] | [.name,.version,(.license//"UNKNOWN")] | @tsv' \
  | sort -u
```

Para una distribución formal, usar `cargo-about` y una herramienta equivalente para npm, y archivar los textos completos.

Verificación rápida y reproducible del grafo Cargo (sin red):

```bash
./scripts/audit-licenses.sh
```

El script falla ante GPL/AGPL/LGPL/SSPL/CPAL sin decisión registrada y lista las
excepciones MPL-2.0 aceptadas en ADR-017.

## Dependencias directas de npm

| Paquete | Licencia |
|---|---|
| `@material/material-color-utilities` | Apache-2.0 |
| `@tauri-apps/api` | Apache-2.0 OR MIT |
| `date-fns` | MIT |
| `i18next` | MIT |
| `react` | MIT |
| `react-dom` | MIT |
| `react-i18next` | MIT |
| `zustand` | MIT |
| `@biomejs/biome` (dev) | MIT OR Apache-2.0 |
| `@tauri-apps/cli` (dev) | Apache-2.0 OR MIT |
| `@testing-library/jest-dom` (dev) | MIT |
| `@testing-library/react` (dev) | MIT |
| `@testing-library/user-event` (dev) | MIT |
| `@types/node` (dev) | MIT |
| `@types/react` (dev) | MIT |
| `@types/react-dom` (dev) | MIT |
| `@vitejs/plugin-react` (dev) | MIT |
| `jsdom` (dev) | MIT |
| `typescript` (dev) | Apache-2.0 |
| `vite` (dev) | MIT |
| `vitest` (dev) | MIT |

Inventario npm resuelto: **152 paquetes** (incluye binarios opcionales por plataforma que no se instalan todos en Linux).

## Dependencias directas de Rust

| Crate | Licencia |
|---|---|
| `tauri` | MIT OR Apache-2.0 |
| `tauri-build` | MIT OR Apache-2.0 |
| `serde` | MIT OR Apache-2.0 |
| `serde_json` | MIT OR Apache-2.0 |
| `rusqlite` | MIT |
| `uuid` | Apache-2.0 OR MIT |
| `thiserror` | MIT OR Apache-2.0 |
| `sha2` | MIT OR Apache-2.0 |
| `hex` | MIT OR Apache-2.0 |
| `base64` | MIT OR Apache-2.0 |
| `semver` | MIT OR Apache-2.0 |
| `minisign-verify` | MIT |
| `reqwest` (solo updater) | MIT OR Apache-2.0 |
| `rustls` (transitivo, solo updater) | Apache-2.0 OR ISC OR MIT |
| `chrono` (notificaciones) | MIT OR Apache-2.0 |
| `zbus` (notificaciones) | MIT |
| `libsqlite3-sys` (transitivo) | MIT; SQLite es dominio público |

Inventario Cargo resuelto: **499 paquetes** (incluye dependencias de todas las plataformas y de build; no todos se compilan en Linux).

## Resumen del grafo Cargo

| Familia de licencia | Paquetes |
|---|---|
| MIT OR Apache-2.0 | 228 |
| MIT | 115 |
| Apache-2.0 OR MIT | 49 |
| Unicode-3.0 | 18 |
| Zlib OR Apache-2.0 OR MIT | 17 |
| MIT/Apache-2.0 | 16 |
| Unlicense OR MIT | 9 |
| MPL-2.0 | 5 |
| Otras combinaciones permisivas | resto |

No se detectaron paquetes GPL ni AGPL en el grafo resuelto.

## Excepción de plataforma: WebKitGTK y GTK (LGPL)

En Linux, Tauri usa WebKitGTK, GTK 3, GLib, libsoup y otras bibliotecas del sistema con licencia **LGPL-2.1-or-later** (algunas con licencias compatibles).

- Se enlazan **dinámicamente**; no se modifican ni se incluyen en el repositorio.
- Study Planner no es un trabajo derivado de esas bibliotecas.
- Al distribuir un binario, hay que:
  - incluir los avisos de copyright y licencia correspondientes;
  - permitir que el usuario reemplace las bibliotecas del sistema (enlace dinámico);
  - ofrecer el código fuente de las versiones LGPL utilizadas, o la vía para obtenerlo.
- La compilación estándar de Tauri ya usa enlace dinámico en Linux. No usar enlace estático de WebKitGTK/GTK sin revisar la LGPL.

## Excepción aceptada: MPL-2.0 transitiva

Se detectaron paquetes **MPL-2.0** en el grafo de Tauri. MPL-2.0 es copyleft a nivel de archivo: permite usar los archivos sin modificar en un programa mayor, pero exige conservar avisos y, si se modifica el archivo MPL, publicar esa modificación bajo MPL. La excepción fue revisada y aceptada en **ADR-017** (2026-11-08).

| Paquete | Uso | ¿Se distribuye en el binario? |
|---|---|---|
| `option-ext` 0.2.0 | Runtime, vía `dirs` → `tauri`/`wry` | Sí |
| `cssparser` 0.37.0 | Build, vía `dom_query` → `tauri-utils` | No (solo build) |
| `selectors` 0.38.0 | Build, vía `dom_query` → `tauri-utils` | No (solo build) |
| `dtoa-short` 0.3.5 | Build, vía `cssparser` | No (solo build) |
| `lightningcss` (npm) | Build, vía Vite | No (solo build) |

**Estado:** aceptada. No se modificó ninguno de esos paquetes. Al distribuir, se conservan los avisos de copyright y licencia; si se modifica un archivo MPL, se publica ese archivo bajo MPL-2.0. Esta excepción no se extiende a otras dependencias copyleft.

> Nota: `r-efi` declara `MIT OR Apache-2.0 OR LGPL-2.1-or-later`; se elige la opción MIT/Apache-2.0, así que no cuenta como excepción LGPL.

## Fuentes e íconos

- **No** se descargan fuentes remotas.
- La UI usa fuentes del sistema con fallback (`Roboto Flex`, `Roboto`, `Segoe UI`, `system-ui`). No se bundlea ninguna fuente.
- El logo `public/study-planner.png` y los íconos de navegación son propios, cubiertos por la licencia del proyecto.
- No se usan íconos de CDNs ni de paquetes con licencias adicionales.

## Dependencias de desarrollo

Las dependencias de desarrollo (Vitest, Testing Library, Biome, TypeScript, Vite, Tauri CLI) no se incluyen en el binario de producción, pero sus licencias se respetan en el repositorio. `lightningcss` (MPL-2.0) es la única excepción no permisiva del árbol npm y es solo de build.

## Obligaciones al distribuir

1. Incluir `LICENSE-MIT` y `LICENSE-APACHE` del proyecto.
2. Incluir este archivo y los textos completos de las licencias de terceros.
3. Conservar avisos de copyright de WebKitGTK/GTK y de los crates MPL usados.
4. No relicenciar código de terceros.
5. Si se modifica un archivo MPL, publicar ese archivo bajo MPL-2.0.
6. Si se distribuye el AppImage, incluir la oferta de código fuente de las bibliotecas LGPL o la referencia a los paquetes del sistema.

## Pendientes

- [x] Decidir si se acepta MPL-2.0 transitiva o se busca alternativa (ADR-017: aceptada).
- [ ] Generar los textos completos de licencias con `cargo-about` y una herramienta npm antes de la primera release pública.
- [ ] Revisar licencias de cualquier dependencia nueva antes de agregarla.
- [ ] Confirmar que el AppImage no enlace estáticamente WebKitGTK/GTK.
