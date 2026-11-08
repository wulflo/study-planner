//! Núcleo de dominio de Study Planner.
//!
//! Este crate no depende de Tauri, de la red ni del sistema de archivos.
//! Contiene constantes compartidas, identificadores y, más adelante, las
//! reglas puras del dominio (recurrencias, notas, prioridad de estudio).

use serde::Serialize;
use uuid::Uuid;

mod academic;
mod task;

pub use academic::{
    AcademicYear, AttendanceRecord, AttendanceStatus, AttendanceSummary, ClassException, ClassSlot,
    DEFAULT_SUBJECT_COLOR, ExceptionKind, Holiday, NAME_MAX_CHARS, OccurrenceKind, Recurrence,
    RotationConfig, RotationMode, SHORT_TEXT_MAX_CHARS, ScheduleOccurrence, ScheduleSlot,
    SlotOverlap, Subject, Term, TermKind, add_days, attendance_summary, date_within, days_between,
    find_overlaps, minutes_to_time, next_class, normalize_color, normalize_date, normalize_name,
    normalize_optional_short, normalize_optional_text, normalize_time, parse_date, slot_occurs_on,
    time_to_minutes, validate_basis_points, validate_date_range, validate_time_range,
    validate_weekday, week_occurrences, week_start_of, weekday_of, weekdays_between,
};
pub use task::{TASK_TITLE_MAX_CHARS, Task, TaskStatus, validate_task_title};

/// Nombre visible de la aplicación.
pub const APP_NAME: &str = "Study Planner";

/// Versión de la aplicación tomada del paquete Cargo.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versión del esquema SQLite. Se incrementa con cada migración.
pub const SCHEMA_VERSION: u32 = 1;

/// Versión del formato de exportación `.txt`.
pub const EXPORT_FORMAT_VERSION: u32 = 1;

/// Información estable que la UI puede mostrar sin tocar la bóveda.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub schema_version: u32,
    pub export_format_version: u32,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            name: APP_NAME,
            version: APP_VERSION,
            schema_version: SCHEMA_VERSION,
            export_format_version: EXPORT_FORMAT_VERSION,
        }
    }
}

/// Información de la app para el frontend.
pub fn app_info() -> AppInfo {
    AppInfo::default()
}

/// Genera un identificador UUIDv7, ordenable por tiempo.
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_is_stable() {
        let info = AppInfo::default();
        assert_eq!(info.name, "Study Planner");
        assert_eq!(info.schema_version, SCHEMA_VERSION);
        assert_eq!(info.export_format_version, EXPORT_FORMAT_VERSION);
    }

    #[test]
    fn ids_are_uuid_v7_and_unique() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        let parsed = Uuid::parse_str(&a).expect("uuid válido");
        assert_eq!(parsed.get_version_num(), 7);
    }
}
