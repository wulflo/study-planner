//! Repositorio de tareas, etiquetas, filtros guardados y búsqueda FTS.
//!
//! Las tareas usan borrado lógico. La tabla `task_fts` es derivada: se
//! reconstruye en cada alta, edición, cambio de etiquetas y borrado. El journal
//! de undo guarda el estado previo para poder deshacer un borrado.

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};
use serde::Deserialize;
use study_planner_core::{
    SavedFilter, Tag, Task, TaskFilter, TaskPriority, TaskRecurrence, TaskStatus, TaskType, new_id,
    normalize_date, normalize_task_description, normalize_time, validate_tag_name,
    validate_task_title,
};

use crate::{Store, StoreError};

const TASK_COLUMNS: &str = "id, subject_id, parent_task_id, type, title, description, \
     start_date, due_date, due_time, priority, important, estimate_minutes, status, \
     completed_at, recurrence_json, created_at, updated_at";
const TAG_COLUMNS: &str = "id, name, color, created_at, updated_at";
const FILTER_COLUMNS: &str = "id, screen, name, query_json, created_at, updated_at";

/// Datos editables de una tarea.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskInput {
    pub subject_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub task_type: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: Option<i64>,
    pub important: bool,
    pub estimate_minutes: Option<i64>,
    pub status: Option<String>,
    pub recurrence: Option<TaskRecurrence>,
    pub tag_ids: Vec<String>,
}

impl Store {
    /// Lista todas las tareas vivas con el orden habitual de la UI.
    pub fn list_tasks(&self) -> Result<Vec<Task>, StoreError> {
        self.search_tasks(&TaskFilter::default())
    }

    /// Busca tareas combinando texto FTS y filtros estructurados.
    pub fn search_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>, StoreError> {
        let mut sql = format!("SELECT {TASK_COLUMNS} FROM task WHERE deleted_at IS NULL");
        let mut values: Vec<Value> = Vec::new();

        if let Some(text) = filter.text.as_deref()
            && let Some(query) = fts_query(text)
        {
            sql.push_str(" AND id IN (SELECT task_id FROM task_fts WHERE task_fts MATCH ?)");
            values.push(Value::Text(query));
        }

        if let Some(status) = filter.status.as_deref() {
            if status == "open" {
                sql.push_str(" AND status <> 'done'");
            } else {
                sql.push_str(" AND status = ?");
                values.push(Value::Text(status.to_string()));
            }
        }

        if let Some(priority) = filter.priority {
            sql.push_str(" AND priority = ?");
            values.push(Value::Integer(priority));
        }

        if let Some(tag_id) = filter.tag_id.as_deref() {
            sql.push_str(
                " AND id IN (SELECT task_id FROM task_tag
                             WHERE tag_id = ? AND deleted_at IS NULL)",
            );
            values.push(Value::Text(tag_id.to_string()));
        }

        if let Some(subject_id) = filter.subject_id.as_deref() {
            sql.push_str(" AND subject_id = ?");
            values.push(Value::Text(subject_id.to_string()));
        }

        if let Some(due_from) = filter.due_from.as_deref() {
            sql.push_str(" AND due_date IS NOT NULL AND due_date >= ?");
            values.push(Value::Text(due_from.to_string()));
        }

        if let Some(due_to) = filter.due_to.as_deref() {
            sql.push_str(" AND due_date IS NOT NULL AND due_date <= ?");
            values.push(Value::Text(due_to.to_string()));
        }

        if let Some(parent_task_id) = filter.parent_task_id.as_deref() {
            sql.push_str(" AND parent_task_id = ?");
            values.push(Value::Text(parent_task_id.to_string()));
        } else if filter.roots_only {
            sql.push_str(" AND parent_task_id IS NULL");
        }

        sql.push_str(
            " ORDER BY CASE status
                         WHEN 'in_progress' THEN 0
                         WHEN 'pending' THEN 1
                         ELSE 2
                       END,
                       CASE WHEN due_date IS NULL THEN 1 ELSE 0 END,
                       due_date,
                       priority DESC,
                       created_at DESC",
        );

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), task_from_row)?;
        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        for task in &mut tasks {
            task.tags = self.load_task_tags(&task.id)?;
        }
        Ok(tasks)
    }

    /// Crea una tarea pendiente con solo título. Compatibilidad con Fase 1.
    pub fn create_task(&self, title: &str) -> Result<Task, StoreError> {
        self.create_task_full(&TaskInput {
            title: title.to_string(),
            ..TaskInput::default()
        })
    }

    /// Crea una tarea con todos sus campos.
    pub fn create_task_full(&self, input: &TaskInput) -> Result<Task, StoreError> {
        let values = self.validate_task_input(input)?;
        self.validate_task_parent(None, &values)?;

        let id = new_id();
        let completed_at = if values.status == TaskStatus::Done {
            Some(self.utc_now()?)
        } else {
            None
        };

        self.connection().execute(
            "INSERT INTO task
               (id, subject_id, parent_task_id, type, title, description, start_date,
                due_date, due_time, priority, important, estimate_minutes, status,
                completed_at, recurrence_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                id,
                values.subject_id,
                values.parent_task_id,
                values.task_type.as_str(),
                values.title,
                values.description,
                values.start_date,
                values.due_date,
                values.due_time,
                values.priority.as_i64(),
                values.important,
                values.estimate_minutes,
                values.status.as_str(),
                completed_at,
                values.recurrence_json,
            ],
        )?;

        self.set_task_tags(&id, &values.tag_ids)?;
        self.refresh_task_fts(&id)?;
        self.get_task(&id)?.ok_or(StoreError::TaskNotFound)
    }

    /// Actualiza todos los campos editables de una tarea viva.
    pub fn update_task(&self, id: &str, input: &TaskInput) -> Result<Task, StoreError> {
        let current = self.get_task(id)?.ok_or(StoreError::TaskNotFound)?;
        let values = self.validate_task_input(input)?;
        self.validate_task_parent(Some(id), &values)?;

        let completed_at = match (values.status, &current.completed_at) {
            (TaskStatus::Done, Some(value)) => Some(value.clone()),
            (TaskStatus::Done, None) => Some(self.utc_now()?),
            (_, _) => None,
        };

        let changed = self.connection().execute(
            "UPDATE task
             SET subject_id = ?1, parent_task_id = ?2, type = ?3, title = ?4,
                 description = ?5, start_date = ?6, due_date = ?7, due_time = ?8,
                 priority = ?9, important = ?10, estimate_minutes = ?11, status = ?12,
                 completed_at = ?13, recurrence_json = ?14,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?15 AND deleted_at IS NULL",
            params![
                values.subject_id,
                values.parent_task_id,
                values.task_type.as_str(),
                values.title,
                values.description,
                values.start_date,
                values.due_date,
                values.due_time,
                values.priority.as_i64(),
                values.important,
                values.estimate_minutes,
                values.status.as_str(),
                completed_at,
                values.recurrence_json,
                id,
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::TaskNotFound);
        }

        self.set_task_tags(id, &values.tag_ids)?;
        self.refresh_task_fts(id)?;
        self.get_task(id)?.ok_or(StoreError::TaskNotFound)
    }

    /// Marca una tarea como hecha o la reabre.
    pub fn set_task_done(&self, id: &str, done: bool) -> Result<Task, StoreError> {
        let status = if done {
            TaskStatus::Done
        } else {
            TaskStatus::Pending
        };
        self.set_task_status(id, status.as_str())
    }

    /// Cambia el estado de una tarea viva.
    pub fn set_task_status(&self, id: &str, status: &str) -> Result<Task, StoreError> {
        let status =
            TaskStatus::from_db(status).ok_or(StoreError::InvalidTask("Estado inválido."))?;
        let completed_at = if status == TaskStatus::Done {
            Some(self.utc_now()?)
        } else {
            None
        };

        let changed = self.connection().execute(
            "UPDATE task
             SET status = ?1, completed_at = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?3 AND deleted_at IS NULL",
            params![status.as_str(), completed_at, id],
        )?;
        if changed == 0 {
            return Err(StoreError::TaskNotFound);
        }
        self.refresh_task_fts(id)?;
        self.get_task(id)?.ok_or(StoreError::TaskNotFound)
    }

    /// Borra lógicamente una tarea, registra undo y quita su fila FTS.
    pub fn delete_task(&self, id: &str) -> Result<(), StoreError> {
        let task = self.get_task(id)?.ok_or(StoreError::TaskNotFound)?;
        let children: i64 = self.connection().query_row(
            "SELECT count(*) FROM task WHERE parent_task_id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        if children > 0 {
            return Err(StoreError::Conflict(
                "La tarea tiene subtareas vivas; borralas o movelas primero.",
            ));
        }

        let before = serde_json::to_string(&task)?;
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

        self.connection()
            .execute("DELETE FROM task_fts WHERE task_id = ?1", params![id])?;
        self.record_undo("delete_task", "task", id, Some(&before))?;
        Ok(())
    }

    /// Lista las etiquetas vivas.
    pub fn list_tags(&self) -> Result<Vec<Tag>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {TAG_COLUMNS} FROM tag WHERE deleted_at IS NULL ORDER BY name"
        ))?;
        let rows = statement.query_map([], tag_from_row)?;
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }

    /// Crea una etiqueta.
    pub fn create_tag(&self, name: &str, color: Option<&str>) -> Result<Tag, StoreError> {
        let name = validate_tag_name(name).map_err(StoreError::InvalidTask)?;
        let color = normalize_tag_color(color)?;
        let id = new_id();
        self.connection().execute(
            "INSERT INTO tag (id, name, color) VALUES (?1, ?2, ?3)",
            params![id, name, color],
        )?;
        self.get_tag(&id)?.ok_or(StoreError::TagNotFound)
    }

    /// Actualiza una etiqueta viva.
    pub fn update_tag(&self, id: &str, name: &str, color: Option<&str>) -> Result<Tag, StoreError> {
        let name = validate_tag_name(name).map_err(StoreError::InvalidTask)?;
        let color = normalize_tag_color(color)?;
        let changed = self.connection().execute(
            "UPDATE tag
             SET name = ?1, color = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?3 AND deleted_at IS NULL",
            params![name, color, id],
        )?;
        if changed == 0 {
            return Err(StoreError::TagNotFound);
        }
        self.get_tag(id)?.ok_or(StoreError::TagNotFound)
    }

    /// Borra lógicamente una etiqueta y sus vínculos.
    pub fn delete_tag(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE tag
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::TagNotFound);
        }

        self.connection().execute(
            "UPDATE task_tag
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE tag_id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        self.refresh_all_task_fts()?;
        Ok(())
    }

    /// Reemplaza las etiquetas vivas de una tarea.
    pub fn set_task_tags(&self, task_id: &str, tag_ids: &[String]) -> Result<(), StoreError> {
        for tag_id in tag_ids {
            let exists: i64 = self.connection().query_row(
                "SELECT count(*) FROM tag WHERE id = ?1 AND deleted_at IS NULL",
                params![tag_id],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(StoreError::TagNotFound);
            }
        }

        let mut existing = self
            .connection()
            .prepare("SELECT tag_id FROM task_tag WHERE task_id = ?1 AND deleted_at IS NULL")?;
        let current: Vec<String> = existing
            .query_map(params![task_id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;

        for tag_id in &current {
            if !tag_ids.contains(tag_id) {
                self.connection().execute(
                    "UPDATE task_tag
                     SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE task_id = ?1 AND tag_id = ?2 AND deleted_at IS NULL",
                    params![task_id, tag_id],
                )?;
            }
        }

        for tag_id in tag_ids {
            if current.contains(tag_id) {
                continue;
            }
            let revived = self.connection().execute(
                "UPDATE task_tag
                 SET deleted_at = NULL,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE task_id = ?1 AND tag_id = ?2 AND deleted_at IS NOT NULL",
                params![task_id, tag_id],
            )?;
            if revived == 0 {
                self.connection().execute(
                    "INSERT INTO task_tag (id, task_id, tag_id) VALUES (?1, ?2, ?3)",
                    params![new_id(), task_id, tag_id],
                )?;
            }
        }

        self.refresh_task_fts(task_id)?;
        Ok(())
    }

    /// Lista filtros guardados, opcionalmente por pantalla.
    pub fn list_saved_filters(&self, screen: Option<&str>) -> Result<Vec<SavedFilter>, StoreError> {
        let mut sql = format!("SELECT {FILTER_COLUMNS} FROM saved_filter WHERE deleted_at IS NULL");
        let mut values: Vec<Value> = Vec::new();
        if let Some(screen) = screen {
            sql.push_str(" AND screen = ?");
            values.push(Value::Text(screen.to_string()));
        }
        sql.push_str(" ORDER BY name");

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), filter_from_row)?;
        let mut filters = Vec::new();
        for row in rows {
            filters.push(row?);
        }
        Ok(filters)
    }

    /// Guarda un filtro de tareas validado.
    pub fn create_saved_filter(
        &self,
        screen: &str,
        name: &str,
        query_json: &str,
    ) -> Result<SavedFilter, StoreError> {
        validate_saved_filter(screen, name, query_json)?;
        let id = new_id();
        self.connection().execute(
            "INSERT INTO saved_filter (id, screen, name, query_json) VALUES (?1, ?2, ?3, ?4)",
            params![id, screen.trim(), name.trim(), query_json],
        )?;
        self.get_saved_filter(&id)?
            .ok_or(StoreError::SavedFilterNotFound)
    }

    /// Actualiza un filtro guardado vivo.
    pub fn update_saved_filter(
        &self,
        id: &str,
        name: &str,
        query_json: &str,
    ) -> Result<SavedFilter, StoreError> {
        let current = self
            .get_saved_filter(id)?
            .ok_or(StoreError::SavedFilterNotFound)?;
        validate_saved_filter(&current.screen, name, query_json)?;
        let changed = self.connection().execute(
            "UPDATE saved_filter
             SET name = ?1, query_json = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?3 AND deleted_at IS NULL",
            params![name.trim(), query_json, id],
        )?;
        if changed == 0 {
            return Err(StoreError::SavedFilterNotFound);
        }
        self.get_saved_filter(id)?
            .ok_or(StoreError::SavedFilterNotFound)
    }

    /// Borra lógicamente un filtro guardado.
    pub fn delete_saved_filter(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE saved_filter
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::SavedFilterNotFound);
        }
        Ok(())
    }

    /// Reconstruye la fila FTS de una tarea viva.
    pub(crate) fn refresh_task_fts(&self, task_id: &str) -> Result<(), StoreError> {
        self.connection()
            .execute("DELETE FROM task_fts WHERE task_id = ?1", params![task_id])?;
        let Some(task) = self.get_task(task_id)? else {
            return Ok(());
        };
        let tags = task
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        self.connection().execute(
            "INSERT INTO task_fts (task_id, title, description, tags) VALUES (?1, ?2, ?3, ?4)",
            params![
                task_id,
                task.title,
                task.description.unwrap_or_default(),
                tags,
            ],
        )?;
        Ok(())
    }

    fn refresh_all_task_fts(&self) -> Result<(), StoreError> {
        let ids: Vec<String> = {
            let mut statement = self
                .connection()
                .prepare("SELECT id FROM task WHERE deleted_at IS NULL")?;
            statement
                .query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<String>>>()?
        };
        self.connection().execute("DELETE FROM task_fts", [])?;
        for id in ids {
            self.refresh_task_fts(&id)?;
        }
        Ok(())
    }

    fn get_task(&self, id: &str) -> Result<Option<Task>, StoreError> {
        let mut task = self
            .connection()
            .query_row(
                &format!("SELECT {TASK_COLUMNS} FROM task WHERE id = ?1 AND deleted_at IS NULL"),
                params![id],
                task_from_row,
            )
            .optional()?;
        if let Some(task) = &mut task {
            task.tags = self.load_task_tags(&task.id)?;
        }
        Ok(task)
    }

    fn load_task_tags(&self, task_id: &str) -> Result<Vec<Tag>, StoreError> {
        let mut statement = self.connection().prepare(
            "SELECT t.id, t.name, t.color, t.created_at, t.updated_at
             FROM task_tag tt
             JOIN tag t ON t.id = tt.tag_id
             WHERE tt.task_id = ?1
               AND tt.deleted_at IS NULL
               AND t.deleted_at IS NULL
             ORDER BY t.name",
        )?;
        let rows = statement.query_map(params![task_id], tag_from_row)?;
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }

    fn get_tag(&self, id: &str) -> Result<Option<Tag>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!("SELECT {TAG_COLUMNS} FROM tag WHERE id = ?1 AND deleted_at IS NULL"),
                params![id],
                tag_from_row,
            )
            .optional()?)
    }

    fn get_saved_filter(&self, id: &str) -> Result<Option<SavedFilter>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!(
                    "SELECT {FILTER_COLUMNS} FROM saved_filter
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                filter_from_row,
            )
            .optional()?)
    }

    fn validate_task_input(&self, input: &TaskInput) -> Result<TaskValues, StoreError> {
        let title = validate_task_title(&input.title).map_err(StoreError::InvalidTask)?;
        let description = input
            .description
            .as_deref()
            .map(normalize_task_description)
            .transpose()
            .map_err(StoreError::InvalidTask)?
            .flatten();
        let start_date = normalize_optional_date(input.start_date.as_deref())?;
        let due_date = normalize_optional_date(input.due_date.as_deref())?;
        let due_time = match input
            .due_time
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => Some(normalize_time(value).map_err(StoreError::InvalidTask)?),
            None => None,
        };

        if let (Some(start), Some(due)) = (&start_date, &due_date)
            && start > due
        {
            return Err(StoreError::InvalidTask(
                "La fecha de inicio no puede ser posterior al vencimiento.",
            ));
        }

        let task_type = match input.task_type.as_deref() {
            None | Some("") => TaskType::Task,
            Some(value) => TaskType::from_db(value)
                .ok_or(StoreError::InvalidTask("Tipo de tarea inválido."))?,
        };
        let status = match input.status.as_deref() {
            None | Some("") => TaskStatus::Pending,
            Some(value) => {
                TaskStatus::from_db(value).ok_or(StoreError::InvalidTask("Estado inválido."))?
            }
        };
        let priority = match input.priority {
            None => TaskPriority::Low,
            Some(value) => TaskPriority::from_i64(value)
                .ok_or(StoreError::InvalidTask("Prioridad inválida."))?,
        };

        if let Some(minutes) = input.estimate_minutes
            && minutes < 0
        {
            return Err(StoreError::InvalidTask(
                "La estimación no puede ser negativa.",
            ));
        }

        let recurrence = match &input.recurrence {
            Some(recurrence) => {
                if recurrence.interval == 0 {
                    return Err(StoreError::InvalidTask(
                        "La recurrencia necesita un intervalo mayor que cero.",
                    ));
                }
                if due_date.is_none() {
                    return Err(StoreError::InvalidTask(
                        "Una tarea recurrente necesita fecha de vencimiento.",
                    ));
                }
                let until = match recurrence.until.as_deref().map(str::trim) {
                    Some(value) if !value.is_empty() => {
                        Some(normalize_date(value).map_err(StoreError::InvalidTask)?)
                    }
                    _ => None,
                };
                Some(TaskRecurrence {
                    kind: recurrence.kind,
                    interval: recurrence.interval,
                    until,
                })
            }
            None => None,
        };

        let subject_id = clean_optional_id(input.subject_id.as_deref());
        let parent_task_id = clean_optional_id(input.parent_task_id.as_deref());

        if let Some(subject_id) = &subject_id {
            let exists: i64 = self.connection().query_row(
                "SELECT count(*) FROM subject WHERE id = ?1 AND deleted_at IS NULL",
                params![subject_id],
                |row| row.get(0),
            )?;
            if exists == 0 {
                return Err(StoreError::SubjectNotFound);
            }
        }

        Ok(TaskValues {
            subject_id,
            parent_task_id,
            task_type,
            title,
            description,
            start_date,
            due_date,
            due_time,
            priority,
            important: input.important,
            estimate_minutes: input.estimate_minutes,
            status,
            recurrence_json: recurrence.as_ref().map(serde_json::to_string).transpose()?,
            tag_ids: input.tag_ids.clone(),
        })
    }

    fn validate_task_parent(
        &self,
        task_id: Option<&str>,
        values: &TaskValues,
    ) -> Result<(), StoreError> {
        let Some(parent_id) = &values.parent_task_id else {
            return Ok(());
        };
        if task_id == Some(parent_id.as_str()) {
            return Err(StoreError::InvalidTask(
                "Una tarea no puede ser su propio padre.",
            ));
        }
        let parent = self.get_task(parent_id)?.ok_or(StoreError::TaskNotFound)?;

        if let (Some(child_subject), Some(parent_subject)) =
            (&values.subject_id, &parent.subject_id)
            && child_subject != parent_subject
        {
            return Err(StoreError::InvalidTask(
                "La subtarea pertenece a otra materia que su tarea padre.",
            ));
        }

        let mut current = Some(parent_id.clone());
        let mut visited = 0usize;
        while let Some(candidate) = current {
            if task_id == Some(candidate.as_str()) {
                return Err(StoreError::InvalidTask(
                    "La jerarquía de tareas crearía un ciclo.",
                ));
            }
            if visited > 10_000 {
                return Err(StoreError::InvalidTask(
                    "La jerarquía de tareas tiene un ciclo.",
                ));
            }
            visited += 1;
            let task = self.get_task(&candidate)?.ok_or(StoreError::TaskNotFound)?;
            current = task.parent_task_id;
        }
        Ok(())
    }
}

struct TaskValues {
    subject_id: Option<String>,
    parent_task_id: Option<String>,
    task_type: TaskType,
    title: String,
    description: Option<String>,
    start_date: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    priority: TaskPriority,
    important: bool,
    estimate_minutes: Option<i64>,
    status: TaskStatus,
    recurrence_json: Option<String>,
    tag_ids: Vec<String>,
}

fn clean_optional_id(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_optional_date(raw: Option<&str>) -> Result<Option<String>, StoreError> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Ok(Some(
            normalize_date(value).map_err(StoreError::InvalidTask)?,
        )),
        None => Ok(None),
    }
}

fn normalize_tag_color(color: Option<&str>) -> Result<Option<String>, StoreError> {
    match color.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Ok(Some(
            study_planner_core::normalize_color(value).map_err(StoreError::InvalidTask)?,
        )),
        None => Ok(None),
    }
}

fn validate_saved_filter(screen: &str, name: &str, query_json: &str) -> Result<(), StoreError> {
    if screen.trim().is_empty() {
        return Err(StoreError::InvalidTask(
            "La pantalla del filtro no puede estar vacía.",
        ));
    }
    if name.trim().is_empty() {
        return Err(StoreError::InvalidTask(
            "El nombre del filtro no puede estar vacío.",
        ));
    }
    serde_json::from_str::<TaskFilter>(query_json)
        .map_err(|_| StoreError::InvalidTask("El filtro guardado no tiene un formato válido."))?;
    Ok(())
}

fn fts_query(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    let task_type: String = row.get(3)?;
    let priority: i64 = row.get(9)?;
    let status: String = row.get(12)?;
    let recurrence_json: Option<String> = row.get(14)?;
    let recurrence = recurrence_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<TaskRecurrence>(value).ok());

    Ok(Task {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        parent_task_id: row.get(2)?,
        task_type: TaskType::from_db(&task_type).unwrap_or(TaskType::Task),
        title: row.get(4)?,
        description: row.get(5)?,
        start_date: row.get(6)?,
        due_date: row.get(7)?,
        due_time: row.get(8)?,
        priority: TaskPriority::from_i64(priority).unwrap_or(TaskPriority::Low),
        important: row.get(10)?,
        estimate_minutes: row.get(11)?,
        status: TaskStatus::from_db(&status).unwrap_or(TaskStatus::Pending),
        completed_at: row.get(13)?,
        recurrence,
        tags: Vec::new(),
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn tag_from_row(row: &Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn filter_from_row(row: &Row<'_>) -> rusqlite::Result<SavedFilter> {
    Ok(SavedFilter {
        id: row.get(0)?,
        screen: row.get(1)?,
        name: row.get(2)?,
        query_json: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::migrated_store;

    #[test]
    fn creates_lists_completes_and_deletes_tasks() {
        let store = migrated_store();

        let first = store.create_task("  Repasar matrices  ").unwrap();
        let second = store.create_task("Entregar práctico").unwrap();

        assert_eq!(first.title, "Repasar matrices");
        assert_eq!(first.status, TaskStatus::Pending);
        assert_eq!(first.task_type, TaskType::Task);
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

    #[test]
    fn supports_tags_search_and_saved_filters() {
        let store = migrated_store();
        let tag = store.create_tag("  mate  ", Some("#6750A4")).unwrap();
        let task = store
            .create_task_full(&TaskInput {
                title: "Resolver integrales".to_string(),
                description: Some("Capítulo 4 de cálculo".to_string()),
                priority: Some(2),
                tag_ids: vec![tag.id.clone()],
                ..TaskInput::default()
            })
            .unwrap();
        assert_eq!(task.tags.len(), 1);

        let by_text = store
            .search_tasks(&TaskFilter {
                text: Some("integrales".to_string()),
                ..TaskFilter::default()
            })
            .unwrap();
        assert_eq!(by_text.len(), 1);
        assert_eq!(by_text[0].id, task.id);

        let by_tag = store
            .search_tasks(&TaskFilter {
                tag_id: Some(tag.id.clone()),
                ..TaskFilter::default()
            })
            .unwrap();
        assert_eq!(by_tag.len(), 1);

        let filter = store
            .create_saved_filter("tasks", "Prioridad alta", r#"{"priority":2}"#)
            .unwrap();
        assert_eq!(store.list_saved_filters(Some("tasks")).unwrap().len(), 1);

        store.delete_saved_filter(&filter.id).unwrap();
        assert!(store.list_saved_filters(Some("tasks")).unwrap().is_empty());
    }

    #[test]
    fn supports_subtasks_and_recurrence_validation() {
        let store = migrated_store();
        let parent = store.create_task("Trabajo práctico").unwrap();
        let child = store
            .create_task_full(&TaskInput {
                title: "Investigar".to_string(),
                parent_task_id: Some(parent.id.clone()),
                ..TaskInput::default()
            })
            .unwrap();
        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));

        let error = store.delete_task(&parent.id).unwrap_err();
        assert!(matches!(error, StoreError::Conflict(_)));

        let error = store
            .create_task_full(&TaskInput {
                title: "Repasar".to_string(),
                recurrence: Some(TaskRecurrence {
                    kind: study_planner_core::TaskRecurrenceKind::Daily,
                    interval: 0,
                    until: None,
                }),
                ..TaskInput::default()
            })
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidTask(_)));
    }
}
