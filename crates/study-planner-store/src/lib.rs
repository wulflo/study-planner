//! Persistencia SQLite de Study Planner.
//!
//! La base se abre con WAL, claves foráneas activas y sincronización completa.
//! Las migraciones son hacia adelante y se registran con checksum. El respaldo
//! previo a una migración corresponde a la capa de bóveda (Fase 1).

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use study_planner_core::{SCHEMA_VERSION, new_id};
use thiserror::Error;

mod academic;
mod backup;
mod tasks;

pub use academic::{SubjectInput, TermInput};
pub use backup::BackupReport;

const INITIAL_NAME: &str = "0001_initial";
const INITIAL_SQL: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error(
        "el esquema {found} es más nuevo que el soportado ({supported}); actualizá Study Planner"
    )]
    SchemaTooNew { found: i64, supported: i64 },
    #[error("metadato requerido ausente: {0}")]
    MissingMeta(&'static str),
    #[error("tarea inválida: {0}")]
    InvalidTask(&'static str),
    #[error("dato académico inválido: {0}")]
    InvalidAcademic(&'static str),
    #[error("tarea no encontrada")]
    TaskNotFound,
    #[error("año académico no encontrado")]
    AcademicYearNotFound,
    #[error("término no encontrado")]
    TermNotFound,
    #[error("materia no encontrada")]
    SubjectNotFound,
    #[error("{0}")]
    Conflict(&'static str),
    #[error("error de archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("backup inválido: {0}")]
    InvalidBackup(String),
}

/// Conexión a la base de Study Planner.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Abre una base en memoria. Útil para pruebas y para el autodiagnóstico.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        Ok(Self { conn })
    }

    /// Abre una base en disco y configura WAL y claves foráneas.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        Ok(Self { conn })
    }

    fn configure(conn: &Connection) -> Result<(), StoreError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        // En memoria SQLite responde "memory"; en disco pasa a "wal".
        let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        Ok(())
    }

    /// Aplica la migración inicial si la base está vacía.
    pub fn migrate(&mut self) -> Result<(), StoreError> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let supported = i64::from(SCHEMA_VERSION);

        if current > supported {
            return Err(StoreError::SchemaTooNew {
                found: current,
                supported,
            });
        }

        if current == supported {
            return Ok(());
        }

        let tx = self.conn.transaction()?;
        tx.execute_batch(INITIAL_SQL)?;
        tx.pragma_update(None, "user_version", supported)?;

        tx.execute(
            "INSERT INTO schema_migration(version, name, checksum) VALUES (?1, ?2, ?3)",
            params![supported, INITIAL_NAME, sha256_hex(INITIAL_SQL.as_bytes())],
        )?;

        tx.execute(
            "INSERT INTO app_meta(key, value) VALUES ('schema_version', ?1)
             ON CONFLICT(key) DO UPDATE SET
               value = excluded.value,
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![supported.to_string()],
        )?;

        tx.execute(
            "INSERT OR IGNORE INTO app_meta(key, value) VALUES ('vault_id', ?1)",
            params![new_id()],
        )?;

        tx.execute(
            "INSERT OR IGNORE INTO app_meta(key, value, updated_at)
             VALUES ('created_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Versión de esquema efectiva según SQLite.
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        let version = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        Ok(version)
    }

    /// Identificador estable de la bóveda. Se genera en la primera migración.
    pub fn vault_id(&self) -> Result<String, StoreError> {
        let value = self
            .conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'vault_id'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        value.ok_or(StoreError::MissingMeta("vault_id"))
    }

    /// Resultado de `PRAGMA integrity_check`.
    pub fn integrity_check(&self) -> Result<String, StoreError> {
        let result = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        Ok(result)
    }

    /// Fecha civil local en formato `AAAA-MM-DD`. Se usa para las snapshots.
    pub fn local_date(&self) -> Result<String, StoreError> {
        let value = self.conn.query_row(
            "SELECT strftime('%Y-%m-%d', 'now', 'localtime')",
            [],
            |row| row.get(0),
        )?;
        Ok(value)
    }

    /// Instante UTC en RFC3339 con milisegundos, coherente con el esquema.
    pub fn utc_now(&self) -> Result<String, StoreError> {
        let value =
            self.conn
                .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
                    row.get(0)
                })?;
        Ok(value)
    }

    /// Cantidad de violaciones de claves foráneas.
    pub fn foreign_key_violations(&self) -> Result<usize, StoreError> {
        let mut stmt = self.conn.prepare("PRAGMA foreign_key_check")?;
        let rows = stmt.query_map([], |_| Ok(()))?;
        let mut count = 0usize;
        for row in rows {
            row?;
            count += 1;
        }
        Ok(count)
    }

    /// Acceso de bajo nivel para la capa de repositorios (Fase 1).
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

// Necesario para `optional()` en `vault_id`.
use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_store() -> Store {
        let mut store = Store::open_in_memory().expect("abrir memoria");
        store.migrate().expect("migrar");
        store
    }

    #[test]
    fn migrates_fresh_database() {
        let store = migrated_store();
        assert_eq!(store.schema_version().unwrap(), i64::from(SCHEMA_VERSION));
        assert_eq!(store.integrity_check().unwrap(), "ok");
        assert_eq!(store.foreign_key_violations().unwrap(), 0);
        assert!(uuid::Uuid::parse_str(&store.vault_id().unwrap()).is_ok());
    }

    #[test]
    fn migration_is_idempotent() {
        let mut store = migrated_store();
        store.migrate().expect("segunda migración");
        assert_eq!(store.schema_version().unwrap(), 1);
    }

    #[test]
    fn creates_expected_tables_and_seeds_settings() {
        let store = migrated_store();
        for table in [
            "academic_year",
            "term",
            "subject",
            "class_slot",
            "exam",
            "task",
            "topic",
            "self_assessment",
            "grade_entry",
            "study_session",
            "note",
            "attachment",
            "notification",
            "setting",
        ] {
            let exists: i64 = store
                .connection()
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "falta la tabla {table}");
        }

        let accent: String = store
            .connection()
            .query_row(
                "SELECT value_json FROM setting WHERE key = 'theme.accent'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(accent, "\"#6750A4\"");

        let scope: String = store
            .connection()
            .query_row(
                "SELECT scope FROM setting WHERE key = 'paths.vault'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(scope, "machine");
    }

    #[test]
    fn fts5_is_available() {
        let store = migrated_store();
        store
            .connection()
            .execute(
                "INSERT INTO task_fts(task_id, title, description, tags)
                 VALUES ('t1', 'Álgebra lineal', 'Repasar matrices', 'mate')",
                [],
            )
            .unwrap();
        let count: i64 = store
            .connection()
            .query_row(
                "SELECT count(*) FROM task_fts WHERE task_fts MATCH 'algebra'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn rejects_newer_schema() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .connection()
            .pragma_update(None, "user_version", 99)
            .unwrap();
        let err = store.migrate().unwrap_err();
        assert!(matches!(
            err,
            StoreError::SchemaTooNew {
                found: 99,
                supported: 1
            }
        ));
    }
}
