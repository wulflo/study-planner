//! Repositorio de tareas sobre SQLite.
//!
//! Las tareas usan borrado lógico (`deleted_at`) y estados
//! `pending` / `in_progress` / `done`.

use rusqlite::{OptionalExtension, Row, params};
use study_planner_core::{Task, TaskStatus, new_id, validate_task_title};

use crate::{Store, StoreError};

const TASK_COLUMNS: &str = "id, title, status, created_at, completed_at";

impl Store {
    /// Lista las tareas vivas, con las pendientes primero.
    pub fn list_tasks(&self) -> Result<Vec<Task>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {TASK_COLUMNS}
             FROM task
             WHERE deleted_at IS NULL
             ORDER BY CASE status
                        WHEN 'in_progress' THEN 0
                        WHEN 'pending' THEN 1
                        ELSE 2
                      END,
                      created_at DESC"
        ))?;

        let rows = statement.query_map([], task_from_row)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    /// Crea una tarea pendiente y devuelve su representación persistida.
    pub fn create_task(&self, title: &str) -> Result<Task, StoreError> {
        let title = validate_task_title(title).map_err(StoreError::InvalidTask)?;
        let id = new_id();

        self.connection().execute(
            "INSERT INTO task (id, title, status) VALUES (?1, ?2, 'pending')",
            params![id, title],
        )?;

        self.get_task(&id)?.ok_or(StoreError::TaskNotFound)
    }

    /// Marca una tarea como hecha o la reabre.
    pub fn set_task_done(&self, id: &str, done: bool) -> Result<Task, StoreError> {
        let status = if done {
            TaskStatus::Done.as_str()
        } else {
            TaskStatus::Pending.as_str()
        };

        let changed = self.connection().execute(
            "UPDATE task
             SET status = ?1,
                 completed_at = CASE
                   WHEN ?2 THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                   ELSE NULL
                 END,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?3 AND deleted_at IS NULL",
            params![status, done, id],
        )?;

        if changed == 0 {
            return Err(StoreError::TaskNotFound);
        }

        self.get_task(id)?.ok_or(StoreError::TaskNotFound)
    }

    /// Borra lógicamente una tarea.
    pub fn delete_task(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE task
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;

        if changed == 0 {
            return Err(StoreError::TaskNotFound);
        }

        Ok(())
    }

    fn get_task(&self, id: &str) -> Result<Option<Task>, StoreError> {
        let task = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {TASK_COLUMNS}
                     FROM task
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                task_from_row,
            )
            .optional()?;
        Ok(task)
    }
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    let status: String = row.get(2)?;

    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status: TaskStatus::from_db(&status).unwrap_or(TaskStatus::Pending),
        created_at: row.get(3)?,
        completed_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    fn migrated_store() -> Store {
        let mut store = Store::open_in_memory().expect("abrir memoria");
        store.migrate().expect("migrar");
        store
    }

    #[test]
    fn creates_lists_completes_and_deletes_tasks() {
        let store = migrated_store();

        let first = store.create_task("  Repasar matrices  ").unwrap();
        let second = store.create_task("Entregar práctico").unwrap();

        assert_eq!(first.title, "Repasar matrices");
        assert_eq!(first.status, TaskStatus::Pending);
        assert_eq!(store.list_tasks().unwrap().len(), 2);

        let completed = store.set_task_done(&first.id, true).unwrap();
        assert_eq!(completed.status, TaskStatus::Done);
        assert!(completed.completed_at.is_some());

        let reopened = store.set_task_done(&first.id, false).unwrap();
        assert_eq!(reopened.status, TaskStatus::Pending);
        assert!(reopened.completed_at.is_none());

        store.delete_task(&second.id).unwrap();
        let tasks = store.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, first.id);
    }

    #[test]
    fn rejects_empty_titles() {
        let store = migrated_store();
        let error = store.create_task("   ").unwrap_err();
        assert!(matches!(error, StoreError::InvalidTask(_)));
    }

    #[test]
    fn unknown_tasks_return_not_found() {
        let store = migrated_store();
        let error = store.set_task_done("inexistente", true).unwrap_err();
        assert!(matches!(error, StoreError::TaskNotFound));
    }
}
