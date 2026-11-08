//! Journal de deshacer con expiración.
//!
//! Por ahora la única acción soportada es restaurar una tarea borrada
//! lógicamente. El journal guarda el estado previo y una fecha de expiración
//! para no acumular entradas indefinidamente.

use rusqlite::{OptionalExtension, Row, params};
use serde::Serialize;
use study_planner_core::{Task, new_id};

use crate::{Store, StoreError};

/// Minutos que una acción permanece disponible para deshacer.
pub const UNDO_TTL_MINUTES: i64 = 5;

const UNDO_COLUMNS: &str =
    "id, action, entity_type, entity_id, before_json, after_json, expires_at, created_at";

/// Entrada visible del journal de undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoEntry {
    pub id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub expires_at: String,
    pub created_at: String,
}

impl Store {
    /// Lista las acciones todavía deshacibles.
    pub fn list_undo_entries(&self) -> Result<Vec<UndoEntry>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {UNDO_COLUMNS}
             FROM undo_journal
             WHERE deleted_at IS NULL
               AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             ORDER BY created_at DESC"
        ))?;
        let rows = statement.query_map([], undo_from_row)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Deshace una entrada viva. Devuelve la tarea restaurada si corresponde.
    pub fn undo_entry(&self, id: &str) -> Result<Option<Task>, StoreError> {
        let entry = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {UNDO_COLUMNS}
                     FROM undo_journal
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                undo_from_row,
            )
            .optional()?
            .ok_or(StoreError::UndoNotFound)?;

        let not_expired: i64 = self.connection().query_row(
            "SELECT count(*) FROM undo_journal
             WHERE id = ?1
               AND expires_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![id],
            |row| row.get(0),
        )?;
        if not_expired == 0 {
            return Err(StoreError::UndoExpired);
        }

        let restored = if entry.action == "delete_task" && entry.entity_type == "task" {
            let exists: i64 = self.connection().query_row(
                "SELECT count(*) FROM task WHERE id = ?1",
                params![entry.entity_id],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(StoreError::Conflict(
                    "La tarea ya no existe en la base; no se puede deshacer.",
                ));
            }

            self.connection().execute(
                "UPDATE task
                 SET deleted_at = NULL,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                params![entry.entity_id],
            )?;
            self.refresh_task_fts(&entry.entity_id)?;
            self.get_task_for_undo(&entry.entity_id)?
        } else {
            None
        };

        self.connection().execute(
            "UPDATE undo_journal
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;

        Ok(restored)
    }

    /// Marca como usadas las entradas vencidas.
    pub fn purge_expired_undo(&self) -> Result<usize, StoreError> {
        let changed = self.connection().execute(
            "UPDATE undo_journal
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE deleted_at IS NULL
               AND expires_at <= strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            [],
        )?;
        Ok(changed)
    }

    pub(crate) fn record_undo(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: &str,
        before_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let id = new_id();
        self.connection().execute(
            "INSERT INTO undo_journal
               (id, action, entity_type, entity_id, before_json, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5,
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?6))",
            params![
                id,
                action,
                entity_type,
                entity_id,
                before_json,
                format!("+{UNDO_TTL_MINUTES} minutes"),
            ],
        )?;
        Ok(())
    }

    fn get_task_for_undo(&self, id: &str) -> Result<Option<Task>, StoreError> {
        // Se reutiliza la búsqueda para traer etiquetas ya hidratadas.
        Ok(self.list_tasks()?.into_iter().find(|task| task.id == id))
    }
}

fn undo_from_row(row: &Row<'_>) -> rusqlite::Result<UndoEntry> {
    Ok(UndoEntry {
        id: row.get(0)?,
        action: row.get(1)?,
        entity_type: row.get(2)?,
        entity_id: row.get(3)?,
        before_json: row.get(4)?,
        after_json: row.get(5)?,
        expires_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::migrated_store;

    #[test]
    fn undoes_a_task_deletion_until_it_expires() {
        let store = migrated_store();
        let task = store.create_task("Repasar").unwrap();

        store.delete_task(&task.id).unwrap();
        assert!(store.list_tasks().unwrap().is_empty());
        let entries = store.list_undo_entries().unwrap();
        assert_eq!(entries.len(), 1);

        let restored = store.undo_entry(&entries[0].id).unwrap().unwrap();
        assert_eq!(restored.id, task.id);
        assert_eq!(store.list_tasks().unwrap().len(), 1);
        assert!(store.list_undo_entries().unwrap().is_empty());
    }

    #[test]
    fn expired_entries_are_rejected_and_purged() {
        let store = migrated_store();
        let task = store.create_task("Repasar").unwrap();
        store.delete_task(&task.id).unwrap();
        let entry = store.list_undo_entries().unwrap().remove(0);

        store
            .connection()
            .execute(
                "UPDATE undo_journal
                 SET expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-1 minute')
                 WHERE id = ?1",
                params![entry.id],
            )
            .unwrap();

        let error = store.undo_entry(&entry.id).unwrap_err();
        assert!(matches!(error, StoreError::UndoExpired));
        assert_eq!(store.purge_expired_undo().unwrap(), 1);
    }
}
