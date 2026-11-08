//! Núcleo de dominio de Study Planner.
//!
//! Este crate no depende de Tauri, de la red ni del sistema de archivos.
//! Contiene constantes compartidas, identificadores y, más adelante, las
//! reglas puras del dominio (recurrencias, notas, prioridad de estudio).

use serde::Serialize;
use uuid::Uuid;

mod academic;
mod exam;
mod grade;
mod study;
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
pub use exam::{
    EXAM_TITLE_MAX_CHARS, Exam, ExamResultStatus, ExamTopic, TOPIC_NAME_MAX_CHARS, Topic,
    normalize_topic_description, topic_would_create_cycle, validate_exam_times,
    validate_exam_title, validate_topic_name,
};
pub use grade::{
    CategoryGrade, GradeCategory, GradeEntry, GradeScale, SubjectGrade, entry_percent_bp,
    grade_category_result, scale_cp_from_percent, subject_grade, validate_grade_entry,
    validate_grade_scale, validate_grade_weight,
};
pub use study::{
    ASSESSMENT_MAX, ASSESSMENT_MIN, AssessmentSource, DEFAULT_BREAK_MINUTES,
    DEFAULT_POMODORO_MINUTES, DEFAULT_PRIORITY_DECAY_DAYS, DEFAULT_STRONG_THRESHOLD_BP,
    DEFAULT_WEAK_THRESHOLD_BP, EASE_DEFAULT_CP, EASE_MIN_CP, PriorityReason, REVIEW_ALGORITHM,
    REVIEW_ALGORITHM_VERSION, REVIEW_HORIZON_DAYS, ReviewItem, SelfAssessment, Sm2Result, Sm2State,
    StrengthClassification, StrengthOverride, StudyPriority, StudyPriorityInput, StudySession,
    StudySessionMode, StudyTotals, SubjectMinutes, TopicStrengthRow, UpcomingReview, WeakSubject,
    classification_from_score, effective_classification, effective_session_minutes,
    minutes_by_subject, normalize_assessment_note, session_totals, sm2_review,
    sort_study_priorities, streak_days, study_priority, summarize_weak_subjects, upcoming_reviews,
    validate_assessment_score, validate_strength_thresholds,
};
pub use task::{
    SavedFilter, TAG_NAME_MAX_CHARS, TASK_DESCRIPTION_MAX_CHARS, TASK_TITLE_MAX_CHARS, Tag, Task,
    TaskFilter, TaskPriority, TaskRecurrence, TaskRecurrenceKind, TaskStatus, TaskType, add_months,
    normalize_task_description, validate_tag_name, validate_task_title,
};

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
