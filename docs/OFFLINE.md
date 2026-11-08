# Trabajo sin internet

Omega debe poder retomarse sin conexión. Este documento explica qué funciona offline, qué no, y cómo evitar romper el proyecto cuando no hay red.

## Requisito previo

La primera instalación y el primer build requieren internet para descargar crates y paquetes npm. Después de eso, el repo debe compilar con las cachés locales.

## Qué se necesita tener

| Recurso | Ubicación típica | Si falta |
|---|---|---|
| `node_modules/` | raíz del repo | `npm ci --offline` o esperar internet |
| Caché npm | `~/.npm` | esperar internet |
| Caché Cargo | `~/.cargo/registry` y `~/.cargo/git` | esperar internet |
| `target/` | raíz del repo y `src-tauri/` | se puede reconstruir con caché Cargo |
| WebKitGTK/GTK | paquetes del sistema | no se puede instalar offline |
| Node, npm, Rust | sistema | no se puede instalar offline |

## Comandos offline

```bash
# Frontend
npm ci --offline
npm run typecheck
npm run lint
npm test
npm run build

# Rust
CARGO_NET_OFFLINE=true cargo test -p omega-core -p omega-store -p omega-transfer
CARGO_NET_OFFLINE=true cargo check -p omega

# App completa (sin red; Vite sirve local)
npm run tauri dev
```

Si `npm ci --offline` falla por un paquete faltante, **no borres** `package-lock.json` ni `node_modules` a ciegas. Esperá a tener red y ejecutá `npm ci`.

Si Cargo falla por un crate faltante:

```bash
# Con internet, una sola vez:
cargo fetch

# Sin internet:
CARGO_NET_OFFLINE=true cargo build
```

## Qué no hacer offline

- `npm update`, `npm install <paquete>`, `npm audit fix`.
- `cargo update`, `cargo add`, `cargo install`.
- `npm create tauri-app`, `tauri init`, `tauri icon` (pueden buscar recursos).
- Cambiar versiones en `package.json` o `Cargo.toml` sin lockfile resuelto.
- Borrar `Cargo.lock`, `package-lock.json`, `node_modules` o `~/.cargo/registry`.
- Ejecutar CI o workflows de release.
- Generar claves del updater (Fase 5; requiere herramientas y cuidado, no red, pero no es una tarea offline improvisada).

## Cómo retomar el proyecto

1. Abrir `docs/CHECKLIST.md`.
2. Leer **Estado actual** y **Reglas para no romper**.
3. Ejecutar:

   ```bash
   ./scripts/check.sh
   ./scripts/audit-network.sh
   ```

4. Si todo está verde, tomar el primer ítem pendiente de la fase actual.
5. Si algo falla, reparar antes de avanzar. No agregar features sobre una base roja.
6. Al terminar la sesión, actualizar el checklist y commitear localmente.

## Verificación de que no hay red

`scripts/audit-network.sh` revisa estáticamente:

- Permisos prohibidos en `src-tauri/capabilities/`.
- `connect-src` de la CSP de producción.
- URLs remotas en `src/`, `index.html` y `public/`.
- `withGlobalTauri`.
- Dependencias de red en `omega-updater`.

No reemplaza una prueba dinámica con el monitor del sistema, pero detecta regresiones obvias.

## Si se corta internet a mitad de una tarea

- Los cambios locales no se pierden.
- No ejecutes comandos que requieran descargar dependencias.
- Los tests que ya corrían siguen funcionando.
- Si un build quedó a medias, borrar `dist/` es seguro; borrar `target/` es seguro solo si la caché Cargo está completa (puede tardar en reconstruir).
- Anotá en el checklist dónde quedaste.

## Al volver internet

1. `git status` para ver cambios locales.
2. `npm ci` solo si cambió el lockfile o faltan paquetes.
3. `cargo fetch` si faltan crates.
4. `./scripts/check.sh`.
5. Revisar si hay avisos de seguridad sin aplicar cambios automáticos.
6. Actualizar el checklist y commitear.

## Copias de seguridad del proyecto

El código vive en git local. Los datos de una bóveda real (Fase 1+) viven en `/omega` y sus backups en `/loglife`, fuera del repo. Nunca agregues la bóveda al repositorio.

## Limitaciones

- No hay vendor de Cargo ni de npm todavía. Si las cachés se pierden, hace falta internet.
- No hay espejo local de paquetes del sistema.
- El CI no corre offline; es una verificación remota.
- La primera compilación de Tauri es lenta incluso con caché; no la interpretes como un cuelgue.
