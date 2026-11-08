//! Backups verificados con la API de backup online de SQLite.
//!
//! Un backup nunca se considera válido solo por existir: se vuelve a abrir,
//! se ejecuta `integrity_check` y `foreign_key_check`, y se calcula su SHA-256.
//! La escritura usa un archivo `.partial` y un renombre atómico para no dejar
//! un backup corrupto con el nombre definitivo.

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use rusqlite::{Connection, backup::Backup};
use sha2::{Digest, Sha256};

use crate::{Store, StoreError};

const PAGES_PER_STEP: i32 = 256;
const PAUSE_BETWEEN_STEPS: Duration = Duration::from_millis(10);

/// Resultado verificable de un backup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReport {
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub schema_version: i64,
    pub integrity: String,
    pub foreign_key_violations: usize,
    pub modified_at_millis: u64,
}

impl Store {
    /// Copia la base viva a `destination` usando la API de backup online.
    ///
    /// El destino se escribe primero como `.partial`, se sincroniza a disco y
    /// se verifica. Solo si la verificación pasa se renombra sobre el destino
    /// definitivo, de modo que un backup inválido nunca reemplaza al anterior.
    pub fn backup_to(&self, destination: &Path) -> Result<BackupReport, StoreError> {
        let file_name = destination
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StoreError::InvalidBackup("nombre de archivo inválido".to_string()))?
            .to_string();

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp = destination.with_file_name(format!(".{file_name}.partial"));
        let _ = fs::remove_file(&temp);

        let result = (|| -> Result<BackupReport, StoreError> {
            let mut target = Connection::open(&temp)?;
            {
                let backup = Backup::new(&self.conn, &mut target)?;
                backup.run_to_completion(PAGES_PER_STEP, PAUSE_BETWEEN_STEPS, None)?;
            }
            drop(target);

            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&temp)?
                .sync_all()?;

            // La verificación ocurre antes del renombre: si falla, el backup
            // anterior en `destination` queda intacto.
            let report = Self::verify_backup(&temp)?;
            fs::rename(&temp, destination)?;
            Ok(report)
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        let mut report = result?;

        if let Some(parent) = destination.parent() {
            let _ = File::open(parent).and_then(|directory| directory.sync_all());
        }

        report.file_name = file_name;
        Ok(report)
    }

    /// Reabre un backup y comprueba esquema, integridad, claves foráneas y hash.
    pub fn verify_backup(path: &Path) -> Result<BackupReport, StoreError> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StoreError::InvalidBackup("nombre de archivo inválido".to_string()))?
            .to_string();

        let metadata = fs::metadata(path)?;
        let modified_at_millis = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |elapsed| elapsed.as_millis() as u64);

        let conn = Connection::open(path)?;
        let schema_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

        let foreign_key_violations = {
            let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
            let mut rows = statement.query([])?;
            let mut count = 0usize;
            while rows.next()?.is_some() {
                count += 1;
            }
            count
        };
        drop(conn);

        if integrity != "ok" {
            return Err(StoreError::InvalidBackup(format!(
                "integrity_check devolvió «{integrity}»"
            )));
        }
        if foreign_key_violations != 0 {
            return Err(StoreError::InvalidBackup(format!(
                "{foreign_key_violations} violaciones de claves foráneas"
            )));
        }

        Ok(BackupReport {
            file_name,
            sha256: sha256_file(path)?,
            size_bytes: metadata.len(),
            schema_version,
            integrity,
            foreign_key_violations,
            modified_at_millis,
        })
    }
}

fn sha256_file(path: &Path) -> Result<String, StoreError> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("study-planner-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("crear directorio temporal");
        path
    }

    #[test]
    fn backup_is_verified_and_contains_tasks() {
        let directory = temp_dir("backup");
        let live = directory.join("study-planner.db");
        let destination = directory.join("backup").join("study-planner.db");

        let mut store = Store::open(&live).expect("abrir base viva");
        store.migrate().expect("migrar");
        store.create_task("Repasar matrices").expect("crear tarea");

        let report = store.backup_to(&destination).expect("crear backup");

        assert_eq!(report.integrity, "ok");
        assert_eq!(report.foreign_key_violations, 0);
        assert_eq!(report.schema_version, 1);
        assert_eq!(report.file_name, "study-planner.db");
        assert_eq!(report.sha256.len(), 64);
        assert!(report.size_bytes > 0);
        assert!(destination.exists());
        assert!(
            !destination
                .with_file_name(".study-planner.db.partial")
                .exists()
        );

        let restored = Store::open(&destination).expect("abrir backup");
        let tasks = restored.list_tasks().expect("listar tareas del backup");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Repasar matrices");

        drop(restored);
        drop(store);
        fs::remove_dir_all(directory).expect("limpiar directorio temporal");
    }

    #[test]
    fn backup_overwrites_atomically() {
        let directory = temp_dir("backup-overwrite");
        let live = directory.join("study-planner.db");
        let destination = directory.join("backup.db");

        let mut store = Store::open(&live).expect("abrir base viva");
        store.migrate().expect("migrar");
        store.create_task("Primera").expect("crear tarea");
        let first = store.backup_to(&destination).expect("primer backup");

        store.create_task("Segunda").expect("crear tarea");
        let second = store.backup_to(&destination).expect("segundo backup");

        assert_ne!(first.sha256, second.sha256);
        let restored = Store::open(&destination).expect("abrir backup");
        assert_eq!(restored.list_tasks().expect("listar tareas").len(), 2);

        drop(restored);
        drop(store);
        fs::remove_dir_all(directory).expect("limpiar directorio temporal");
    }

    #[test]
    fn verify_backup_rejects_a_file_that_is_not_a_database() {
        let directory = temp_dir("backup-garbage");
        let path = directory.join("no-es-base.db");
        fs::write(&path, b"esto no es sqlite").expect("escribir archivo inválido");

        assert!(Store::verify_backup(&path).is_err());

        fs::remove_dir_all(directory).expect("limpiar directorio temporal");
    }
}
