//! Exámenes, temas y vínculos examen–tema.
//!
//! Reglas puras: validación de títulos, horarios y jerarquía de temas. La
//! persistencia vive en `study-planner-store`.

use serde::Serialize;

use crate::academic::{normalize_optional_text, normalize_time, validate_time_range};

/// Longitud máxima del título de un examen.
pub const EXAM_TITLE_MAX_CHARS: usize = 200;

/// Longitud máxima del nombre de un tema.
pub const TOPIC_NAME_MAX_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExamResultStatus {
    Pending,
    Passed,
    Failed,
    Absent,
}

impl ExamResultStatus {
    pub const ALL: [Self; 4] = [Self::Pending, Self::Passed, Self::Failed, Self::Absent];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Absent => "absent",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "passed" => Some(Self::Passed),
            "failed" => Some(Self::Failed),
            "absent" => Some(Self::Absent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    pub id: String,
    pub subject_id: String,
    pub term_id: String,
    pub title: String,
    pub date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub room: Option<String>,
    pub weight_bp: Option<i64>,
    pub result_score_cp: Option<i64>,
    pub result_max_cp: Option<i64>,
    pub result_status: ExamResultStatus,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    pub id: String,
    pub subject_id: String,
    pub parent_topic_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamTopic {
    pub id: String,
    pub exam_id: String,
    pub topic_id: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Normaliza y valida el título de un examen.
pub fn validate_exam_title(raw: &str) -> Result<String, &'static str> {
    let title = raw.trim();

    if title.is_empty() {
        return Err("El título del examen no puede estar vacío.");
    }

    if title.chars().count() > EXAM_TITLE_MAX_CHARS {
        return Err("El título del examen supera los 200 caracteres.");
    }

    Ok(title.to_string())
}

/// Normaliza y valida el nombre de un tema.
pub fn validate_topic_name(raw: &str) -> Result<String, &'static str> {
    let name = raw.trim();

    if name.is_empty() {
        return Err("El nombre del tema no puede estar vacío.");
    }

    if name.chars().count() > TOPIC_NAME_MAX_CHARS {
        return Err("El nombre del tema supera los 120 caracteres.");
    }

    Ok(name.to_string())
}

/// Normaliza una descripción opcional de tema.
pub fn normalize_topic_description(raw: &str) -> Result<Option<String>, &'static str> {
    normalize_optional_text(raw)
}

/// Valida el horario opcional de un examen: ambos extremos o ninguno.
pub fn validate_exam_times(
    start: Option<&str>,
    end: Option<&str>,
) -> Result<(Option<String>, Option<String>), &'static str> {
    let start = start.map(str::trim).filter(|value| !value.is_empty());
    let end = end.map(str::trim).filter(|value| !value.is_empty());

    match (start, end) {
        (None, None) => Ok((None, None)),
        (Some(start), Some(end)) => {
            let start = normalize_time(start)?;
            let end = normalize_time(end)?;
            validate_time_range(&start, &end)?;
            Ok((Some(start), Some(end)))
        }
        _ => Err("El examen necesita hora de inicio y de fin, o ninguna."),
    }
}

/// Indica si mover `topic_id` debajo de `parent_id` crearía un ciclo.
///
/// Se recorre la cadena de ancestros del padre propuesto; si aparece el propio
/// tema, el cambio es inválido. También es inválido que un tema sea su padre.
pub fn topic_would_create_cycle(topics: &[Topic], topic_id: &str, parent_id: &str) -> bool {
    if topic_id == parent_id {
        return true;
    }

    let mut current = Some(parent_id);
    let mut visited = 0usize;

    while let Some(candidate) = current {
        if candidate == topic_id {
            return true;
        }

        visited += 1;
        if visited > topics.len() {
            // Datos ya corruptos: se corta para no entrar en un bucle.
            return true;
        }

        current = topics
            .iter()
            .find(|topic| topic.id == candidate)
            .and_then(|topic| topic.parent_topic_id.as_deref());
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(id: &str, parent: Option<&str>) -> Topic {
        Topic {
            id: id.to_string(),
            subject_id: "subject".to_string(),
            parent_topic_id: parent.map(str::to_string),
            name: id.to_string(),
            description: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn exam_status_round_trips() {
        for status in ExamResultStatus::ALL {
            assert_eq!(ExamResultStatus::from_db(status.as_str()), Some(status));
        }
        assert_eq!(ExamResultStatus::from_db("otro"), None);
    }

    #[test]
    fn validates_titles_and_times() {
        assert_eq!(validate_exam_title("  Parcial 1  ").unwrap(), "Parcial 1");
        assert!(validate_exam_title("   ").is_err());
        assert!(validate_exam_title(&"a".repeat(EXAM_TITLE_MAX_CHARS + 1)).is_err());

        assert_eq!(
            validate_exam_times(Some(" 08:00 "), Some("10:00")).unwrap(),
            (Some("08:00".to_string()), Some("10:00".to_string()))
        );
        assert_eq!(validate_exam_times(None, None).unwrap(), (None, None));
        assert!(validate_exam_times(Some("08:00"), None).is_err());
        assert!(validate_exam_times(Some("10:00"), Some("09:00")).is_err());
    }

    #[test]
    fn detects_topic_cycles() {
        let topics = vec![
            topic("root", None),
            topic("child", Some("root")),
            topic("grandchild", Some("child")),
        ];

        assert!(topic_would_create_cycle(&topics, "root", "grandchild"));
        assert!(topic_would_create_cycle(&topics, "child", "child"));
        assert!(!topic_would_create_cycle(&topics, "grandchild", "root"));
        assert!(!topic_would_create_cycle(&topics, "root", "nuevo"));
    }
}
