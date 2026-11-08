# Custodia de claves

> **Estado:** clave de producción generada el 2026-11-09 y clave pública versionada en
> `crates/study-planner-updater/updater.pub`. El backup cifrado y la passphrase están
> bajo custodia del usuario fuera del repositorio; la copia offline y la confirmación
> de la passphrase siguen pendientes del usuario.
> **Regla de oro:** la clave privada y su passphrase nunca pasan por el chat, el repositorio, un issue, una captura de pantalla ni un archivo junto a la clave.

## Qué se protege

| Elemento | Función | Sensibilidad |
|---|---|---|
| Clave privada minisign | Firmar actualizaciones de Study Planner | Crítica |
| Passphrase de la clave | Descifrar la clave privada | Crítica |
| Clave pública minisign | Verificar actualizaciones | Pública, se versiona |
| `vault_id` | Identificar la bóveda | Baja, no es secreto |
| Bóveda SQLite | Datos académicos | Alta, sin cifrar |

La bóveda **no** se cifra (decisión del usuario). La confidencialidad depende del cifrado del disco del sistema y de la custodia física del equipo.

## Custodia acordada

| Copia | Medio | Contenido |
|---|---|---|
| Primaria | Volumen cifrado offline | Clave privada + clave pública + backup `.gpg` |
| Secundaria | Copia offline en un medio separado | Clave privada + clave pública |
| Terciaria | Almacenamiento remoto cifrado | Backup `.gpg` cifrado simétricamente |
| Passphrase | Gestor de contraseñas + papel en un lugar separado | Solo la passphrase |

Reglas:

- La passphrase **no** se guarda en el mismo medio ni en la misma carpeta que la clave.
- El papel con la passphrase no se guarda junto al volumen cifrado.
- No se sube la clave al almacenamiento remoto sin cifrar. El único archivo aceptado
  es el `.gpg` cifrado con una passphrase; nunca el `.key`, un ZIP ni un `tar` en claro.
- El backup remoto usa la misma passphrase que la clave minisign (decisión del
  usuario), guardada en el gestor de contraseñas y en papel.
- El acceso al almacenamiento remoto y al gestor de contraseñas usa 2FA TOTP.

## Protocolo de generación (Fase 5)

El asistente puede ejecutar estos pasos localmente. **No debe imprimir la passphrase en el chat.** Puede dejarla en un archivo temporal con permisos `600` para que el usuario la respalde y luego borrarla.

1. Verificar que el volumen cifrado esté montado y con espacio.
2. Crear un directorio temporal fuera del repo:

   ```bash
   umask 077
   workdir="$(mktemp -d "${TMPDIR:-/tmp}/study-planner-keygen.XXXXXX")"
   ```

3. Generar una passphrase aleatoria de alta entropía **sin mostrarla**:

   ```bash
   # Ejemplo; la longitud y el formato se fijan en Fase 5.
   openssl rand -base64 48 > "$workdir/passphrase.txt"
   chmod 600 "$workdir/passphrase.txt"
   ```

4. Generar el par de claves con la CLI de Tauri (minisign). Consultar la sintaxis exacta con:

   ```bash
   npm run tauri signer generate -- --help
   ```

   Guardar la clave privada en `$workdir/study-planner.key` y la pública en `$workdir/study-planner.key.pub`.
   La CLI pedirá la passphrase; se toma del archivo, nunca de un argumento visible en `ps`.

5. Verificar el par firmando un archivo de prueba y validando la firma con la clave pública:

   ```bash
   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat "$workdir/passphrase.txt")"
   npm run tauri signer sign -- --app-version 0.1.0 \
     --private-key-path "$workdir/study-planner.key" "$workdir/archivo-de-prueba"
   unset TAURI_SIGNING_PRIVATE_KEY_PASSWORD
   cargo run -q -p study-planner-updater --example verify_release -- \
     "$workdir/archivo-de-prueba" "$workdir/archivo-de-prueba.sig" 0.1.0 \
     "$workdir/study-planner.key.pub"
   ```

6. Copiar la clave privada y pública al volumen cifrado y a la copia offline, y generar el backup `.gpg` para el almacenamiento remoto (ver «Backup cifrado con GPG»).

7. Pedir al usuario que guarde la passphrase en el gestor de contraseñas y en papel. Confirmar que la copió antes de borrar el temporal.

8. Borrar el temporal:

   ```bash
   shred -u "$workdir/passphrase.txt" "$workdir/study-planner.key" 2>/dev/null || rm -f "$workdir/passphrase.txt" "$workdir/study-planner.key"
   rmdir "$workdir"
   unset workdir
   ```

   Nota: en SSD, CoW o sistemas con snapshots, `shred` no garantiza borrado físico. La mitigación real es no haber escrito la clave en el disco equivocado y mantener el disco cifrado.

9. Confirmar que no quedaron rastros en el repo:

   ```bash
   git status --ignored
   rg -n "BEGIN|PRIVATE|minisign|passphrase" . --glob '!node_modules/**' --glob '!target/**'
   ```

10. Registrar en `docs/CHECKLIST.md` que la clave existe, sin registrar su contenido ni la passphrase.

## Backup cifrado con GPG

El par minisign se respalda en un único archivo `.tar.gz.gpg` cifrado
simétricamente con la passphrase de la clave. No se usa una clave OpenPGP del
keyring: el cifrado es `gpg --symmetric`, y la passphrase se pasa por archivo,
nunca por `argv`.

```bash
# Passphrase en un archivo 600, fuera del repo.
gpg --batch --yes --symmetric --cipher-algo AES256 \
  --s2k-digest-algo SHA512 --s2k-mode 3 --s2k-count 65011712 \
  --pinentry-mode loopback --passphrase-file /ruta/passphrase.txt \
  --output study-planner-minisign-YYYY-MM-DD.tar.gz.gpg \
  study-planner-minisign-YYYY-MM-DD.tar.gz
```

Contenido del `tar.gz`: `study-planner.key`, `study-planner.key.pub` y un
`README.txt` con las instrucciones de restauración. La passphrase **no** va
dentro del archivo.

Verificación antes de subir:

```bash
gpg --batch --yes --pinentry-mode loopback --passphrase-file /ruta/passphrase.txt \
  --output /tmp/restored.tar.gz --decrypt study-planner-minisign-YYYY-MM-DD.tar.gz.gpg
tar -C /tmp/restored -xzf /tmp/restored.tar.gz
cmp /tmp/original/study-planner.key /tmp/restored/study-planner.key
cmp /tmp/original/study-planner.key.pub /tmp/restored/study-planner.key.pub
```

Subida y verificación:

```bash
rclone copy estudio.gpg remoto:study-planner-backups/
rclone check <carpeta-local> remoto:study-planner-backups/ --include '*.gpg' --one-way
```

Estado actual:

| Archivo | Ubicación | SHA-256 |
|---|---|---|
| `study-planner-minisign-2026-11-09.tar.gz.gpg` | Volumen cifrado y almacenamiento remoto | `1b836fa2b85643f1796075fbbdbbf1d2b0254e1d9704bb2415efe7cef040ffa6` |

El SHA-256 no es secreto: permite comprobar que la copia remota es la misma
que la local. La restauración desde el almacenamiento remoto queda pendiente en el simulacro de
recuperación.

## Clave pública

- Se versiona en `crates/study-planner-updater/updater.pub`.
- Se embebe en el binario para verificar actualizaciones.
- Se publica en las releases para que un tercero pueda verificar.
- No es secreta; puede ir en el README de releases.
- El release workflow se niega a publicar mientras ese archivo siga siendo un
  placeholder (solo comentarios).

## GitHub Actions

> El repositorio es público: [`wulflo/study-planner`](https://github.com/wulflo/study-planner). Estos pasos son para cuando exista la clave.

El workflow `.github/workflows/release.yml` compila el AppImage y lo firma con la
CLI de Tauri usando `--app-version` para que el trusted comment incluya
`version:<versión>`. Después verifica la firma contra la clave pública embebida
antes de publicar.

```bash
# Ejecutar desde archivos locales, nunca pegando secretos en el chat.
gh secret set TAURI_SIGNING_PRIVATE_KEY < /ruta/segura/study-planner.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < /ruta/segura/passphrase.txt
```

Reglas:

- El YAML solo referencia `secrets.*`; nunca contiene la clave ni la passphrase.
- El secret de la clave debe ser el contenido exacto que espera Tauri, no una ruta.
- La clave pública versionada debe corresponder a la privada de los secrets; si no,
  `verify_release` falla y la release no se publica.
- Restringir workflows, revisar permisos del token y exigir revisión para `release`.
- Si el repo es público, recordar que los secretos no se exponen en logs, pero un workflow comprometido sí puede exfiltrarlos.
- Preferible: firma offline en una máquina local y subida manual de artefactos
  (ver `docs/RELEASE.md`).

## Rotación

1. Generar un par nuevo con el mismo protocolo.
2. Publicar una release que incluya la clave pública nueva y firme con **ambas** claves durante la transición.
3. Esperar a que la mayoría de instalaciones acepten la nueva clave.
4. Retirar la clave vieja y revocar su confianza.
5. Actualizar `docs/UPDATER.md`, `docs/KEYS.md` y el checklist.
6. Conservar la clave vieja solo para verificar versiones antiguas, nunca para firmar nuevas.

## Recuperación

Simulacro obligatorio antes de publicar la primera release:

- [ ] Restaurar la clave privada desde el volumen cifrado en una máquina limpia.
- [ ] Restaurar la passphrase desde el gestor de contraseñas.
- [ ] Firmar un archivo de prueba.
- [ ] Verificar la firma con la clave pública del repo.
- [ ] Restaurar desde la copia offline.
- [ ] Restaurar desde el backup `.gpg` del almacenamiento remoto y comprobar el SHA-256.
- [ ] Confirmar que la clave no quedó en el repo ni en el historial de shell.

## Qué hacer si se compromete la clave

1. Detener la publicación de updates.
2. Asumir que un atacante puede firmar versiones maliciosas.
3. Publicar un aviso con la clave pública nueva y un procedimiento manual de recuperación.
4. Rotar la clave y firmar con ambas durante la transición.
5. Revisar `network_event` y logs de release.
6. Cambiar passphrases y revisar accesos a GitHub y a los medios de custodia.

## Prohibiciones

- No commitear `*.key`, `*.pem`, `minisign.key`, `.env`, bases ni backups.
- No pasar la passphrase por `argv` (visible en `ps`).
- No usar la misma passphrase que el gestor de contraseñas, el almacenamiento remoto o el usuario del sistema.
- No guardar la passphrase en el portapapeles más tiempo del necesario.
- No enviar la clave ni la passphrase por chat, correo o issue.
- No desactivar la verificación de firma «para probar».
