//! Tareas: modelo mínimo y validación pura.
//!
//! La persistencia vive en `study-planner-store`; este módulo no conoce SQLite
//! ni Tauri.

use serde::Serialize;

/// Longitud máxima del título de una tarea, en caracteres Unicode.
pub const TASK_TITLE_MAX_CHARS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Normaliza y valida un título escrito por el usuario.
pub fn validate_task_title(raw: &str) -> Result<String, &'static str> {
    let title = raw.trim();

    if title.is_empty() {
        return Err("El título no puede estar vacío.");
    }

    if title.chars().count() > TASK_TITLE_MAX_CHARS {
        return Err("El título supera los 200 caracteres.");
    }

    Ok(title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_accepts_a_normal_title() {
        assert_eq!(
            validate_task_title("  Repasar matrices  ").unwrap(),
            "Repasar matrices"
        );
    }

    #[test]
    fn rejects_empty_titles() {
        assert!(validate_task_title("   ").is_err());
    }

    #[test]
    fn rejects_titles_over_the_limit() {
        let title = "a".repeat(TASK_TITLE_MAX_CHARS + 1);
        assert!(validate_task_title(&title).is_err());
    }

    #[test]
    fn status_round_trips_through_database_values() {
        for status in [
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Done,
        ] {
            assert_eq!(TaskStatus::from_db(status.as_str()), Some(status));
        }
        assert_eq!(TaskStatus::from_db("desconocido"), None);
    }
}
