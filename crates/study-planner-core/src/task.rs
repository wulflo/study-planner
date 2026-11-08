//! Tareas: modelos, validaciones puras y recurrencia.
//!
//! La persistencia vive en `study-planner-store`; este módulo no conoce SQLite
//! ni Tauri.

use serde::{Deserialize, Serialize};

use crate::academic::{add_days, parse_date};

/// Longitud máxima del título de una tarea, en caracteres Unicode.
pub const TASK_TITLE_MAX_CHARS: usize = 200;

/// Longitud máxima de una etiqueta.
pub const TAG_NAME_MAX_CHARS: usize = 40;

/// Longitud máxima de la descripción de una tarea.
pub const TASK_DESCRIPTION_MAX_CHARS: usize = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
}

impl TaskStatus {
    pub const ALL: [Self; 3] = [Self::Pending, Self::InProgress, Self::Done];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Task,
    Practical,
    Project,
    Personal,
}

impl TaskType {
    pub const ALL: [Self; 4] = [Self::Task, Self::Practical, Self::Project, Self::Personal];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Practical => "practical",
            Self::Project => "project",
            Self::Personal => "personal",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "task" => Some(Self::Task),
            "practical" => Some(Self::Practical),
            "project" => Some(Self::Project),
            "personal" => Some(Self::Personal),
            _ => None,
        }
    }
}

/// Prioridad de 0 a 3; se expone como texto para la UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TaskPriority {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Urgent];

    pub fn as_i64(self) -> i64 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Urgent => 3,
        }
    }

    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::Low),
            1 => Some(Self::Medium),
            2 => Some(Self::High),
            3 => Some(Self::Urgent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRecurrenceKind {
    Daily,
    Weekly,
    Monthly,
}

impl TaskRecurrenceKind {
    pub const ALL: [Self; 3] = [Self::Daily, Self::Weekly, Self::Monthly];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecurrence {
    pub kind: TaskRecurrenceKind,
    pub interval: u32,
    pub until: Option<String>,
}

impl TaskRecurrence {
    /// Próxima fecha de vencimiento después de `from`.
    pub fn next_date(&self, from: &str) -> Option<String> {
        let interval = self.interval.max(1);
        let next = match self.kind {
            TaskRecurrenceKind::Daily => add_days(from, i64::from(interval)),
            TaskRecurrenceKind::Weekly => add_days(from, i64::from(interval) * 7),
            TaskRecurrenceKind::Monthly => add_months(from, interval),
        }?;

        if let Some(until) = &self.until
            && next.as_str() > until.as_str()
        {
            return None;
        }

        Some(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub subject_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub task_type: TaskType,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: TaskPriority,
    pub important: bool,
    pub estimate_minutes: Option<i64>,
    pub status: TaskStatus,
    pub completed_at: Option<String>,
    pub recurrence: Option<TaskRecurrence>,
    pub tags: Vec<Tag>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedFilter {
    pub id: String,
    pub screen: String,
    pub name: String,
    pub query_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Filtro de tareas compartido entre la UI y el repositorio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskFilter {
    /// Texto libre; si no está vacío se usa FTS.
    pub text: Option<String>,
    /// `open`, `pending`, `in_progress` o `done`.
    pub status: Option<String>,
    /// 0 a 3.
    pub priority: Option<i64>,
    pub tag_id: Option<String>,
    pub subject_id: Option<String>,
    pub due_from: Option<String>,
    pub due_to: Option<String>,
    /// Si es `Some`, solo tareas hijas directas de esa tarea.
    pub parent_task_id: Option<String>,
    /// Si es `true`, solo tareas raíz (sin padre).
    pub roots_only: bool,
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

/// Normaliza y valida el nombre de una etiqueta.
pub fn validate_tag_name(raw: &str) -> Result<String, &'static str> {
    let name = raw.trim();

    if name.is_empty() {
        return Err("La etiqueta no puede estar vacía.");
    }

    if name.chars().count() > TAG_NAME_MAX_CHARS {
        return Err("La etiqueta supera los 40 caracteres.");
    }

    Ok(name.to_string())
}

/// Normaliza una descripción opcional de tarea.
pub fn normalize_task_description(raw: &str) -> Result<Option<String>, &'static str> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > TASK_DESCRIPTION_MAX_CHARS {
        return Err("La descripción supera los 4000 caracteres.");
    }
    Ok(Some(value.to_string()))
}

/// Suma meses a una fecha civil, ajustando el día al último del mes destino.
pub fn add_months(date: &str, months: u32) -> Option<String> {
    let (year, month, day) = parse_date(date)?;
    let total = i64::from(year) * 12 + i64::from(month) - 1 + i64::from(months);
    let new_year = total.div_euclid(12) as i32;
    let new_month = total.rem_euclid(12) as u32 + 1;
    let new_day = day.min(days_in_month(new_year, new_month));
    Some(format!("{new_year:04}-{new_month:02}-{new_day:02}"))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 30,
    }
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
        for status in TaskStatus::ALL {
            assert_eq!(TaskStatus::from_db(status.as_str()), Some(status));
        }
        assert_eq!(TaskStatus::from_db("desconocido"), None);
    }

    #[test]
    fn task_type_and_priority_round_trip() {
        for task_type in TaskType::ALL {
            assert_eq!(TaskType::from_db(task_type.as_str()), Some(task_type));
        }
        for priority in TaskPriority::ALL {
            assert_eq!(TaskPriority::from_i64(priority.as_i64()), Some(priority));
        }
        assert_eq!(TaskPriority::from_i64(9), None);
    }

    #[test]
    fn recurrence_advances_by_interval_and_stops_at_until() {
        let daily = TaskRecurrence {
            kind: TaskRecurrenceKind::Daily,
            interval: 2,
            until: Some("2026-03-05".to_string()),
        };
        assert_eq!(daily.next_date("2026-03-01").as_deref(), Some("2026-03-03"));
        assert_eq!(daily.next_date("2026-03-04"), None);

        let weekly = TaskRecurrence {
            kind: TaskRecurrenceKind::Weekly,
            interval: 1,
            until: None,
        };
        assert_eq!(
            weekly.next_date("2026-03-01").as_deref(),
            Some("2026-03-08")
        );
    }

    #[test]
    fn monthly_recurrence_clamps_the_day() {
        let monthly = TaskRecurrence {
            kind: TaskRecurrenceKind::Monthly,
            interval: 1,
            until: None,
        };
        assert_eq!(
            monthly.next_date("2026-01-31").as_deref(),
            Some("2026-02-28")
        );
        assert_eq!(
            monthly.next_date("2024-01-31").as_deref(),
            Some("2024-02-29")
        );
        assert_eq!(
            monthly.next_date("2026-12-15").as_deref(),
            Some("2027-01-15")
        );
    }

    #[test]
    fn tag_and_description_are_validated() {
        assert_eq!(validate_tag_name("  mate  ").unwrap(), "mate");
        assert!(validate_tag_name(" ").is_err());
        assert!(validate_tag_name(&"a".repeat(TAG_NAME_MAX_CHARS + 1)).is_err());
        assert_eq!(
            normalize_task_description("  algo  ").unwrap(),
            Some("algo".to_string())
        );
        assert_eq!(normalize_task_description("  ").unwrap(), None);
    }
}
