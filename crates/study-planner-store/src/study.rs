//! Repositorio de estudio: autoevaluaciones, fortalezas, repaso SM-2,
//! sesiones y agregados para prioridad y estadísticas.
//!
//! Todo usa borrado lógico. Las consultas agregadas reutilizan las funciones
//! puras de `study-planner-core` para que la UI y los tests compartan la misma
//! semántica.

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};
use serde::Serialize;
use study_planner_core::{
    AssessmentSource, DEFAULT_PRIORITY_DECAY_DAYS, DEFAULT_STRONG_THRESHOLD_BP,
    DEFAULT_WEAK_THRESHOLD_BP, PriorityReason, REVIEW_ALGORITHM, REVIEW_ALGORITHM_VERSION,
    REVIEW_HORIZON_DAYS, ReviewItem, SelfAssessment, Sm2State, StrengthClassification,
    StrengthOverride, StudyPriorityInput, StudySession, StudySessionMode, StudyTotals,
    SubjectMinutes, TopicStrengthRow, UpcomingReview, WeakSubject, classification_from_score,
    effective_classification, minutes_by_subject, new_id, normalize_assessment_note,
    normalize_date, normalize_optional_text, session_totals, sm2_review, study_priority,
    summarize_weak_subjects, upcoming_reviews, validate_assessment_score,
    validate_strength_thresholds,
};

use crate::{Store, StoreError};

const ASSESSMENT_COLUMNS: &str =
    "id, topic_id, score, note, source, assessed_at, created_at, updated_at";
const OVERRIDE_COLUMNS: &str = "id, topic_id, classification, reason, created_at, updated_at";
const REVIEW_COLUMNS: &str = "id, topic_id, algorithm, algorithm_version, ease_factor_cp, \
     interval_days, repetitions, lapses, due_date, last_reviewed_at, created_at, updated_at";
const SESSION_COLUMNS: &str = "id, subject_id, topic_id, started_at, ended_at, planned_minutes, \
     actual_minutes, mode, interruptions, notes, created_at, updated_at";
const TOPIC_COLUMNS: &str = "id, subject_id, parent_topic_id, name, description, sort_order, \
     created_at, updated_at";

/// Fila de tema enriquecida con su estado de estudio.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicStudy {
    pub topic_id: String,
    pub subject_id: String,
    pub parent_topic_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub latest_assessment: Option<SelfAssessment>,
    pub derived_classification: StrengthClassification,
    pub override_classification: Option<StrengthClassification>,
    pub classification: StrengthClassification,
    pub override_reason: Option<String>,
    pub review_item: Option<ReviewItem>,
    pub next_exam_date: Option<String>,
}

/// Prioridad de estudio lista para la UI, con nombres resueltos.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPriorityView {
    pub topic_id: String,
    pub topic_name: String,
    pub subject_id: String,
    pub subject_name: String,
    pub score: i64,
    pub classification: StrengthClassification,
    pub reasons: Vec<PriorityReason>,
    pub review_due: Option<String>,
    pub next_exam_date: Option<String>,
}

/// Minutos por materia con nombre resuelto.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectMinutesView {
    pub subject_id: Option<String>,
    pub subject_name: Option<String>,
    pub minutes: i64,
    pub sessions: usize,
}

/// Materia con temas débiles y nombre resuelto.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakSubjectView {
    pub subject_id: String,
    pub subject_name: String,
    pub topic_count: usize,
    pub weak_topics: usize,
    pub average_score_bp: Option<i64>,
}

/// Próximo repaso con nombres resueltos.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingReviewView {
    pub topic_id: String,
    pub topic_name: String,
    pub subject_id: String,
    pub subject_name: String,
    pub due_date: String,
    pub days_until: i64,
}

/// Estadísticas de estudio listas para la UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyStats {
    pub totals: StudyTotals,
    pub minutes_by_subject: Vec<SubjectMinutesView>,
    pub weak_subjects: Vec<WeakSubjectView>,
    pub upcoming_reviews: Vec<UpcomingReviewView>,
    pub due_reviews: usize,
}

impl Store {
    /// Lista autoevaluaciones vivas de un tema, de la más reciente a la más vieja.
    pub fn list_self_assessments(&self, topic_id: &str) -> Result<Vec<SelfAssessment>, StoreError> {
        let sql = format!(
            "SELECT {ASSESSMENT_COLUMNS} FROM self_assessment
             WHERE topic_id = ?1 AND deleted_at IS NULL
             ORDER BY assessed_at DESC, created_at DESC, id DESC"
        );
        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params![topic_id], assessment_from_row)?;
        let mut assessments = Vec::new();
        for row in rows {
            assessments.push(row?);
        }
        Ok(assessments)
    }

    /// Crea una autoevaluación validada (1–5).
    pub fn create_self_assessment(
        &self,
        topic_id: &str,
        score: i64,
        note: Option<&str>,
        source: &str,
    ) -> Result<SelfAssessment, StoreError> {
        self.ensure_live_topic(topic_id)?;
        let score = validate_assessment_score(score).map_err(StoreError::InvalidStudy)?;
        let source = AssessmentSource::from_db(source).ok_or(StoreError::InvalidStudy(
            "origen de autoevaluación inválido",
        ))?;
        let note = match note {
            Some(value) => normalize_assessment_note(value).map_err(StoreError::InvalidStudy)?,
            None => None,
        };

        let id = new_id();
        self.connection().execute(
            "INSERT INTO self_assessment (id, topic_id, score, note, source)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, topic_id, i64::from(score), note, source.as_str()],
        )?;
        self.get_self_assessment(&id)?
            .ok_or(StoreError::SelfAssessmentNotFound)
    }

    /// Borra lógicamente una autoevaluación.
    pub fn delete_self_assessment(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE self_assessment SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::SelfAssessmentNotFound);
        }
        Ok(())
    }

    /// Devuelve el override activo de un tema.
    pub fn get_strength_override(
        &self,
        topic_id: &str,
    ) -> Result<Option<StrengthOverride>, StoreError> {
        let sql = format!(
            "SELECT {OVERRIDE_COLUMNS} FROM strength_override
             WHERE topic_id = ?1 AND deleted_at IS NULL"
        );
        self.connection()
            .query_row(&sql, params![topic_id], override_from_row)
            .optional()
            .map_err(StoreError::from)
    }

    /// Lista overrides activos.
    pub fn list_strength_overrides(&self) -> Result<Vec<StrengthOverride>, StoreError> {
        let sql = format!(
            "SELECT {OVERRIDE_COLUMNS} FROM strength_override
             WHERE deleted_at IS NULL ORDER BY topic_id"
        );
        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map([], override_from_row)?;
        let mut overrides = Vec::new();
        for row in rows {
            overrides.push(row?);
        }
        Ok(overrides)
    }

    /// Crea o actualiza el override manual de un tema.
    pub fn set_strength_override(
        &self,
        topic_id: &str,
        classification: &str,
        reason: Option<&str>,
    ) -> Result<StrengthOverride, StoreError> {
        self.ensure_live_topic(topic_id)?;
        let classification = StrengthClassification::from_db(classification)
            .ok_or(StoreError::InvalidStudy("clasificación inválida"))?;
        let reason = match reason {
            Some(value) => normalize_optional_text(value).map_err(StoreError::InvalidStudy)?,
            None => None,
        };

        let existing: Option<String> = self
            .connection()
            .query_row(
                "SELECT id FROM strength_override WHERE topic_id = ?1",
                params![topic_id],
                |row| row.get(0),
            )
            .optional()?;

        match existing {
            Some(id) => {
                self.connection().execute(
                    "UPDATE strength_override
                     SET classification = ?2, reason = ?3,
                         deleted_at = NULL,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    params![id, classification.as_str(), reason],
                )?;
            }
            None => {
                self.connection().execute(
                    "INSERT INTO strength_override (id, topic_id, classification, reason)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![new_id(), topic_id, classification.as_str(), reason],
                )?;
            }
        }

        self.get_strength_override(topic_id)?
            .ok_or(StoreError::StrengthOverrideNotFound)
    }

    /// Borra lógicamente el override de un tema.
    pub fn clear_strength_override(&self, topic_id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE strength_override SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?1 AND deleted_at IS NULL",
            params![topic_id],
        )?;
        if changed == 0 {
            return Err(StoreError::StrengthOverrideNotFound);
        }
        Ok(())
    }

    /// Lista ítems de repaso, opcionalmente filtrados por materia.
    pub fn list_review_items(
        &self,
        subject_id: Option<&str>,
    ) -> Result<Vec<ReviewItem>, StoreError> {
        let mut sql = format!(
            "SELECT {REVIEW_COLUMNS} FROM review_item
             WHERE deleted_at IS NULL"
        );
        let mut values: Vec<Value> = Vec::new();
        if let Some(subject_id) = subject_id {
            sql.push_str(
                " AND topic_id IN (SELECT id FROM topic WHERE subject_id = ? AND deleted_at IS NULL)",
            );
            values.push(Value::Text(subject_id.to_string()));
        }
        sql.push_str(" ORDER BY due_date IS NULL, due_date, topic_id");

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), review_from_row)?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// Devuelve el ítem de repaso activo de un tema.
    pub fn get_review_item(&self, topic_id: &str) -> Result<Option<ReviewItem>, StoreError> {
        let sql = format!(
            "SELECT {REVIEW_COLUMNS} FROM review_item
             WHERE topic_id = ?1 AND deleted_at IS NULL"
        );
        self.connection()
            .query_row(&sql, params![topic_id], review_from_row)
            .optional()
            .map_err(StoreError::from)
    }

    /// Aplica un repaso SM-2 y registra la autoevaluación correspondiente.
    pub fn review_topic(
        &self,
        topic_id: &str,
        quality: i64,
        today: &str,
    ) -> Result<ReviewItem, StoreError> {
        self.ensure_live_topic(topic_id)?;
        let quality = validate_assessment_score(quality).map_err(StoreError::InvalidStudy)?;
        let today = normalize_date(today).map_err(StoreError::InvalidStudy)?;

        let current = self.get_review_item(topic_id)?;
        let state = current
            .as_ref()
            .map(|item| Sm2State {
                ease_factor_cp: item.ease_factor_cp,
                interval_days: item.interval_days,
                repetitions: item.repetitions,
                lapses: item.lapses,
            })
            .unwrap_or_default();
        let result = sm2_review(&state, quality, &today).map_err(StoreError::InvalidStudy)?;
        let now = self.utc_now()?;
        let existing_id: Option<String> = self
            .connection()
            .query_row(
                "SELECT id FROM review_item WHERE topic_id = ?1",
                params![topic_id],
                |row| row.get(0),
            )
            .optional()?;

        match existing_id {
            Some(id) => {
                self.connection().execute(
                    "UPDATE review_item
                     SET algorithm = ?2, algorithm_version = ?3, ease_factor_cp = ?4,
                         interval_days = ?5, repetitions = ?6, lapses = ?7,
                         due_date = ?8, last_reviewed_at = ?9, deleted_at = NULL,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    params![
                        id,
                        REVIEW_ALGORITHM,
                        REVIEW_ALGORITHM_VERSION,
                        result.ease_factor_cp,
                        result.interval_days,
                        result.repetitions,
                        result.lapses,
                        result.due_date,
                        now,
                    ],
                )?;
            }
            None => {
                self.connection().execute(
                    "INSERT INTO review_item
                       (id, topic_id, algorithm, algorithm_version, ease_factor_cp,
                        interval_days, repetitions, lapses, due_date, last_reviewed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        new_id(),
                        topic_id,
                        REVIEW_ALGORITHM,
                        REVIEW_ALGORITHM_VERSION,
                        result.ease_factor_cp,
                        result.interval_days,
                        result.repetitions,
                        result.lapses,
                        result.due_date,
                        now,
                    ],
                )?;
            }
        }

        self.create_self_assessment(topic_id, i64::from(quality), None, "review")?;
        self.get_review_item(topic_id)?
            .ok_or(StoreError::ReviewItemNotFound)
    }

    /// Programa la fecha de un repaso sin aplicar SM-2.
    pub fn set_review_due(&self, topic_id: &str, due_date: &str) -> Result<ReviewItem, StoreError> {
        self.ensure_live_topic(topic_id)?;
        let due_date = normalize_date(due_date).map_err(StoreError::InvalidStudy)?;
        let existing: Option<String> = self
            .connection()
            .query_row(
                "SELECT id FROM review_item WHERE topic_id = ?1",
                params![topic_id],
                |row| row.get(0),
            )
            .optional()?;

        match existing {
            Some(id) => {
                self.connection().execute(
                    "UPDATE review_item SET due_date = ?2, deleted_at = NULL,
                       updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    params![id, due_date],
                )?;
            }
            None => {
                self.connection().execute(
                    "INSERT INTO review_item
                       (id, topic_id, algorithm, algorithm_version, ease_factor_cp,
                        interval_days, repetitions, lapses, due_date)
                     VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, ?6)",
                    params![
                        new_id(),
                        topic_id,
                        REVIEW_ALGORITHM,
                        REVIEW_ALGORITHM_VERSION,
                        study_planner_core::EASE_DEFAULT_CP,
                        due_date,
                    ],
                )?;
            }
        }

        self.get_review_item(topic_id)?
            .ok_or(StoreError::ReviewItemNotFound)
    }

    /// Borra lógicamente el ítem de repaso de un tema.
    pub fn delete_review_item(&self, topic_id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE review_item SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?1 AND deleted_at IS NULL",
            params![topic_id],
        )?;
        if changed == 0 {
            return Err(StoreError::ReviewItemNotFound);
        }
        Ok(())
    }

    /// Lista sesiones de estudio vivas, con filtros opcionales por fecha y materia.
    pub fn list_study_sessions(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        subject_id: Option<&str>,
    ) -> Result<Vec<StudySession>, StoreError> {
        let mut sql =
            format!("SELECT {SESSION_COLUMNS} FROM study_session WHERE deleted_at IS NULL");
        let mut values: Vec<Value> = Vec::new();
        if let Some(from) = from {
            sql.push_str(" AND substr(started_at, 1, 10) >= ?");
            values.push(Value::Text(
                normalize_date(from).map_err(StoreError::InvalidStudy)?,
            ));
        }
        if let Some(to) = to {
            sql.push_str(" AND substr(started_at, 1, 10) <= ?");
            values.push(Value::Text(
                normalize_date(to).map_err(StoreError::InvalidStudy)?,
            ));
        }
        if let Some(subject_id) = subject_id {
            sql.push_str(" AND subject_id = ?");
            values.push(Value::Text(subject_id.to_string()));
        }
        sql.push_str(" ORDER BY started_at DESC, id DESC");

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), session_from_row)?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    /// Inicia una sesión de estudio. Si hay tema, hereda su materia.
    pub fn start_study_session(
        &self,
        subject_id: Option<&str>,
        topic_id: Option<&str>,
        mode: &str,
        planned_minutes: Option<i64>,
    ) -> Result<StudySession, StoreError> {
        let mode = StudySessionMode::from_db(mode)
            .ok_or(StoreError::InvalidStudy("modo de sesión inválido"))?;
        if let Some(minutes) = planned_minutes
            && minutes < 0
        {
            return Err(StoreError::InvalidStudy(
                "los minutos planificados no pueden ser negativos",
            ));
        }

        let topic = match topic_id {
            Some(topic_id) => self.get_topic(topic_id)?,
            None => None,
        };
        if topic_id.is_some() && topic.is_none() {
            return Err(StoreError::TopicNotFound);
        }
        if let Some(subject_id) = subject_id {
            self.ensure_live_subject(subject_id)?;
        }
        if let (Some(subject_id), Some(topic)) = (subject_id, topic.as_ref())
            && subject_id != topic.subject_id
        {
            return Err(StoreError::InvalidStudy(
                "el tema no pertenece a la materia indicada",
            ));
        }

        let resolved_subject = subject_id
            .map(str::to_string)
            .or_else(|| topic.as_ref().map(|topic| topic.subject_id.clone()));
        let id = new_id();
        let started_at = self.utc_now()?;
        self.connection().execute(
            "INSERT INTO study_session
               (id, subject_id, topic_id, started_at, mode, planned_minutes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                resolved_subject,
                topic_id,
                started_at,
                mode.as_str(),
                planned_minutes,
            ],
        )?;
        self.get_study_session(&id)?
            .ok_or(StoreError::StudySessionNotFound)
    }

    /// Cierra una sesión de estudio con sus minutos reales.
    pub fn finish_study_session(
        &self,
        id: &str,
        actual_minutes: Option<i64>,
        interruptions: Option<i64>,
        notes: Option<&str>,
    ) -> Result<StudySession, StoreError> {
        let session = self
            .get_study_session(id)?
            .ok_or(StoreError::StudySessionNotFound)?;
        if session.ended_at.is_some() {
            return Err(StoreError::InvalidStudy("la sesión ya está cerrada"));
        }
        let actual_minutes = actual_minutes
            .or(session.planned_minutes)
            .unwrap_or(0)
            .max(0);
        let interruptions = interruptions.unwrap_or(session.interruptions).max(0);
        let notes = match notes {
            Some(value) => normalize_optional_text(value).map_err(StoreError::InvalidStudy)?,
            None => session.notes,
        };

        self.connection().execute(
            "UPDATE study_session
             SET ended_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 actual_minutes = ?2, interruptions = ?3, notes = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id, actual_minutes, interruptions, notes],
        )?;
        self.get_study_session(id)?
            .ok_or(StoreError::StudySessionNotFound)
    }

    /// Borra lógicamente una sesión.
    pub fn delete_study_session(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE study_session SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::StudySessionNotFound);
        }
        Ok(())
    }

    /// Lista temas con su última autoevaluación, override y repaso.
    pub fn list_topic_studies(
        &self,
        subject_id: Option<&str>,
        today: &str,
    ) -> Result<Vec<TopicStudy>, StoreError> {
        let today = normalize_date(today).map_err(StoreError::InvalidStudy)?;
        let mut sql = format!("SELECT {TOPIC_COLUMNS} FROM topic WHERE deleted_at IS NULL");
        let mut values: Vec<Value> = Vec::new();
        if let Some(subject_id) = subject_id {
            sql.push_str(" AND subject_id = ?");
            values.push(Value::Text(subject_id.to_string()));
        }
        sql.push_str(" ORDER BY subject_id, sort_order, name, id");

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), topic_row)?;
        let mut topics = Vec::new();
        for row in rows {
            topics.push(row?);
        }

        let (weak_bp, strong_bp) = self.study_thresholds()?;
        let mut studies = Vec::with_capacity(topics.len());
        for topic in topics {
            let assessments = self.list_self_assessments(&topic.id)?;
            let latest = assessments.into_iter().next();
            let derived = classification_from_score(
                latest.as_ref().map(|item| item.score),
                weak_bp,
                strong_bp,
            );
            let override_item = self.get_strength_override(&topic.id)?;
            let classification = effective_classification(
                derived,
                override_item.as_ref().map(|item| item.classification),
            );
            let next_exam_date = self.next_exam_date(&topic.id, &today)?;

            studies.push(TopicStudy {
                topic_id: topic.id.clone(),
                subject_id: topic.subject_id.clone(),
                parent_topic_id: topic.parent_topic_id.clone(),
                name: topic.name.clone(),
                description: topic.description.clone(),
                sort_order: topic.sort_order,
                latest_assessment: latest,
                derived_classification: derived,
                override_classification: override_item.as_ref().map(|item| item.classification),
                classification,
                override_reason: override_item.and_then(|item| item.reason),
                review_item: self.get_review_item(&topic.id)?,
                next_exam_date,
            });
        }
        Ok(studies)
    }

    /// Calcula la cola de prioridad explicable para hoy.
    pub fn list_study_priorities(&self, today: &str) -> Result<Vec<StudyPriorityView>, StoreError> {
        let today = normalize_date(today).map_err(StoreError::InvalidStudy)?;
        let decay_days = self.study_decay_days()?;
        let subjects = self.subject_names()?;
        let studies = self.list_topic_studies(None, &today)?;
        let mut priorities = Vec::with_capacity(studies.len());

        for study in studies {
            let priority = study_priority(&StudyPriorityInput {
                topic_id: &study.topic_id,
                classification: study.classification,
                review_due: study
                    .review_item
                    .as_ref()
                    .and_then(|item| item.due_date.as_deref()),
                exam_date: study.next_exam_date.as_deref(),
                today: &today,
                decay_days,
            });
            priorities.push(StudyPriorityView {
                topic_id: study.topic_id.clone(),
                topic_name: study.name.clone(),
                subject_id: study.subject_id.clone(),
                subject_name: subjects
                    .iter()
                    .find(|(id, _)| id == &study.subject_id)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_default(),
                score: priority.score,
                classification: priority.classification,
                reasons: priority.reasons,
                review_due: study
                    .review_item
                    .as_ref()
                    .and_then(|item| item.due_date.clone()),
                next_exam_date: study.next_exam_date.clone(),
            });
        }

        // `study_priority` ya es determinista; el orden final lo fija el core.
        priorities.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.topic_id.cmp(&right.topic_id))
        });
        Ok(priorities)
    }

    /// Calcula estadísticas de estudio y repaso.
    pub fn study_stats(&self, from: Option<&str>, today: &str) -> Result<StudyStats, StoreError> {
        let today = normalize_date(today).map_err(StoreError::InvalidStudy)?;
        let sessions = self.list_study_sessions(from, None, None)?;
        let totals = session_totals(&sessions, &today);
        let subject_names = self.subject_names()?;

        let by_subject: Vec<SubjectMinutes> = minutes_by_subject(&sessions);
        let minutes_by_subject = by_subject
            .into_iter()
            .map(|item| SubjectMinutesView {
                subject_name: item.subject_id.as_ref().and_then(|id| {
                    subject_names
                        .iter()
                        .find(|(subject_id, _)| subject_id == id)
                        .map(|(_, name)| name.clone())
                }),
                subject_id: item.subject_id,
                minutes: item.minutes,
                sessions: item.sessions,
            })
            .collect();

        let studies = self.list_topic_studies(None, &today)?;
        let topic_rows: Vec<TopicStrengthRow> = studies
            .iter()
            .map(|study| TopicStrengthRow {
                topic_id: study.topic_id.clone(),
                subject_id: study.subject_id.clone(),
                classification: study.classification,
                latest_score: study.latest_assessment.as_ref().map(|item| item.score),
            })
            .collect();
        let weak_subjects: Vec<WeakSubject> = summarize_weak_subjects(&topic_rows);
        let weak_subjects = weak_subjects
            .into_iter()
            .map(|item| WeakSubjectView {
                subject_name: subject_names
                    .iter()
                    .find(|(id, _)| id == &item.subject_id)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_default(),
                subject_id: item.subject_id,
                topic_count: item.topic_count,
                weak_topics: item.weak_topics,
                average_score_bp: item.average_score_bp,
            })
            .collect();

        let review_items = self.list_review_items(None)?;
        let due_pairs: Vec<(String, String)> = review_items
            .iter()
            .filter_map(|item| {
                item.due_date
                    .as_ref()
                    .map(|due_date| (item.topic_id.clone(), due_date.clone()))
            })
            .collect();
        let due_reviews = due_pairs
            .iter()
            .filter(|(_, due_date)| {
                study_planner_core::days_between(&today, due_date).is_some_and(|days| days <= 0)
            })
            .count();
        let upcoming: Vec<UpcomingReview> =
            upcoming_reviews(&due_pairs, &today, REVIEW_HORIZON_DAYS);
        let upcoming_reviews = upcoming
            .into_iter()
            .map(|review| {
                let study = studies.iter().find(|item| item.topic_id == review.topic_id);
                UpcomingReviewView {
                    topic_name: study
                        .map(|item| item.name.clone())
                        .unwrap_or_else(|| review.topic_id.clone()),
                    subject_id: study
                        .map(|item| item.subject_id.clone())
                        .unwrap_or_default(),
                    subject_name: study
                        .and_then(|item| {
                            subject_names
                                .iter()
                                .find(|(id, _)| id == &item.subject_id)
                                .map(|(_, name)| name.clone())
                        })
                        .unwrap_or_default(),
                    topic_id: review.topic_id,
                    due_date: review.due_date,
                    days_until: review.days_until,
                }
            })
            .collect();

        Ok(StudyStats {
            totals,
            minutes_by_subject,
            weak_subjects,
            upcoming_reviews,
            due_reviews,
        })
    }

    fn get_self_assessment(&self, id: &str) -> Result<Option<SelfAssessment>, StoreError> {
        let sql = format!("SELECT {ASSESSMENT_COLUMNS} FROM self_assessment WHERE id = ?1");
        self.connection()
            .query_row(&sql, params![id], assessment_from_row)
            .optional()
            .map_err(StoreError::from)
    }

    fn get_study_session(&self, id: &str) -> Result<Option<StudySession>, StoreError> {
        let sql = format!(
            "SELECT {SESSION_COLUMNS} FROM study_session
             WHERE id = ?1 AND deleted_at IS NULL"
        );
        self.connection()
            .query_row(&sql, params![id], session_from_row)
            .optional()
            .map_err(StoreError::from)
    }

    fn next_exam_date(&self, topic_id: &str, today: &str) -> Result<Option<String>, StoreError> {
        self.connection()
            .query_row(
                "SELECT min(exam.date)
                 FROM exam
                 JOIN exam_topic ON exam_topic.exam_id = exam.id
                 WHERE exam_topic.topic_id = ?1
                   AND exam_topic.deleted_at IS NULL
                   AND exam.deleted_at IS NULL
                   AND exam.date >= ?2",
                params![topic_id, today],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.flatten())
            .map_err(StoreError::from)
    }

    fn subject_names(&self) -> Result<Vec<(String, String)>, StoreError> {
        let mut statement = self
            .connection()
            .prepare("SELECT id, name FROM subject WHERE deleted_at IS NULL ORDER BY name, id")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut names = Vec::new();
        for row in rows {
            names.push(row?);
        }
        Ok(names)
    }

    fn study_thresholds(&self) -> Result<(i64, i64), StoreError> {
        let weak = self.setting_basis_points("study.weak_threshold", DEFAULT_WEAK_THRESHOLD_BP);
        let strong =
            self.setting_basis_points("study.strong_threshold", DEFAULT_STRONG_THRESHOLD_BP);
        if validate_strength_thresholds(weak, strong).is_err() {
            return Ok((DEFAULT_WEAK_THRESHOLD_BP, DEFAULT_STRONG_THRESHOLD_BP));
        }
        Ok((weak, strong))
    }

    fn study_decay_days(&self) -> Result<i64, StoreError> {
        let value: Option<String> = self
            .connection()
            .query_row(
                "SELECT value_json FROM setting WHERE key = 'study.priority_decay_days'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|days| *days > 0)
            .unwrap_or(DEFAULT_PRIORITY_DECAY_DAYS))
    }

    fn setting_basis_points(&self, key: &str, default: i64) -> i64 {
        let value: Option<String> = self
            .connection()
            .query_row(
                "SELECT value_json FROM setting WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();
        let Some(raw) = value else {
            return default;
        };
        let Ok(number) = raw.parse::<f64>() else {
            return default;
        };
        let bp = if (0.0..=1.0).contains(&number) {
            (number * 10_000.0).round() as i64
        } else {
            number.round() as i64
        };
        if (0..=10_000).contains(&bp) {
            bp
        } else {
            default
        }
    }
}

fn assessment_from_row(row: &Row<'_>) -> rusqlite::Result<SelfAssessment> {
    let score: i64 = row.get(2)?;
    let source: String = row.get(4)?;
    Ok(SelfAssessment {
        id: row.get(0)?,
        topic_id: row.get(1)?,
        score: score.clamp(1, 5) as u8,
        note: row.get(3)?,
        source: AssessmentSource::from_db(&source).unwrap_or(AssessmentSource::Manual),
        assessed_at: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn override_from_row(row: &Row<'_>) -> rusqlite::Result<StrengthOverride> {
    let classification: String = row.get(2)?;
    Ok(StrengthOverride {
        id: row.get(0)?,
        topic_id: row.get(1)?,
        classification: StrengthClassification::from_db(&classification)
            .unwrap_or(StrengthClassification::Neutral),
        reason: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn review_from_row(row: &Row<'_>) -> rusqlite::Result<ReviewItem> {
    Ok(ReviewItem {
        id: row.get(0)?,
        topic_id: row.get(1)?,
        algorithm: row.get(2)?,
        algorithm_version: row.get(3)?,
        ease_factor_cp: row.get(4)?,
        interval_days: row.get(5)?,
        repetitions: row.get(6)?,
        lapses: row.get(7)?,
        due_date: row.get(8)?,
        last_reviewed_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<StudySession> {
    let mode: String = row.get(7)?;
    Ok(StudySession {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        topic_id: row.get(2)?,
        started_at: row.get(3)?,
        ended_at: row.get(4)?,
        planned_minutes: row.get(5)?,
        actual_minutes: row.get(6)?,
        mode: StudySessionMode::from_db(&mode).unwrap_or(StudySessionMode::Free),
        interruptions: row.get(8)?,
        notes: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn topic_row(row: &Row<'_>) -> rusqlite::Result<study_planner_core::Topic> {
    Ok(study_planner_core::Topic {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        parent_topic_id: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        sort_order: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use crate::tests_support::migrated_store_with_subject;
    use crate::{Store, TopicInput};

    fn seed_topic(store: &Store, subject_id: &str, name: &str) -> String {
        store
            .create_topic(
                subject_id,
                &TopicInput {
                    name: name.to_string(),
                    ..TopicInput::default()
                },
            )
            .expect("crear tema")
            .id
    }

    #[test]
    fn self_assessments_classify_and_override() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let topic_id = seed_topic(&store, &subject_id, "Unidad 1");

        let assessment = store
            .create_self_assessment(&topic_id, 2, Some("  flojo  "), "manual")
            .expect("crear autoevaluación");
        assert_eq!(assessment.score, 2);
        assert_eq!(assessment.note.as_deref(), Some("flojo"));

        let studies = store
            .list_topic_studies(Some(&subject_id), "2026-03-02")
            .expect("listar estudios");
        assert_eq!(studies.len(), 1);
        assert_eq!(
            studies[0].classification,
            study_planner_core::StrengthClassification::Weak
        );

        let override_item = store
            .set_strength_override(&topic_id, "strength", Some("la practiqué mucho"))
            .expect("crear override");
        assert_eq!(
            override_item.classification,
            study_planner_core::StrengthClassification::Strength
        );
        let studies = store
            .list_topic_studies(Some(&subject_id), "2026-03-02")
            .unwrap();
        assert_eq!(
            studies[0].classification,
            study_planner_core::StrengthClassification::Strength
        );
        assert_eq!(
            studies[0].override_classification,
            Some(study_planner_core::StrengthClassification::Strength)
        );

        store.clear_strength_override(&topic_id).expect("limpiar");
        assert!(store.get_strength_override(&topic_id).unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_scores_and_sources() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let topic_id = seed_topic(&store, &subject_id, "Unidad 1");

        assert!(matches!(
            store.create_self_assessment(&topic_id, 0, None, "manual"),
            Err(crate::StoreError::InvalidStudy(_))
        ));
        assert!(matches!(
            store.create_self_assessment(&topic_id, 3, None, "otro"),
            Err(crate::StoreError::InvalidStudy(_))
        ));
        assert!(matches!(
            store.set_strength_override(&topic_id, "fuerte", None),
            Err(crate::StoreError::InvalidStudy(_))
        ));
    }

    #[test]
    fn review_topic_applies_sm2_and_records_assessment() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let topic_id = seed_topic(&store, &subject_id, "Unidad 1");

        let first = store
            .review_topic(&topic_id, 4, "2026-03-01")
            .expect("primer repaso");
        assert_eq!(first.repetitions, 1);
        assert_eq!(first.interval_days, 1);
        assert_eq!(first.due_date.as_deref(), Some("2026-03-02"));
        assert_eq!(first.algorithm, "sm2");
        assert_eq!(first.algorithm_version, "1");

        let second = store
            .review_topic(&topic_id, 4, "2026-03-02")
            .expect("segundo repaso");
        assert_eq!(second.repetitions, 2);
        assert_eq!(second.interval_days, 6);

        let assessments = store.list_self_assessments(&topic_id).unwrap();
        assert_eq!(assessments.len(), 2);
        assert_eq!(
            assessments[0].source,
            study_planner_core::AssessmentSource::Review
        );

        let failed = store
            .review_topic(&topic_id, 1, "2026-03-08")
            .expect("repaso fallido");
        assert_eq!(failed.repetitions, 0);
        assert_eq!(failed.lapses, 1);
        assert_eq!(failed.interval_days, 1);
    }

    #[test]
    fn schedules_and_deletes_review_items() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let topic_id = seed_topic(&store, &subject_id, "Unidad 1");

        let item = store
            .set_review_due(&topic_id, "2026-03-10")
            .expect("programar");
        assert_eq!(item.due_date.as_deref(), Some("2026-03-10"));
        assert_eq!(store.list_review_items(Some(&subject_id)).unwrap().len(), 1);

        store.delete_review_item(&topic_id).expect("borrar");
        assert!(store.get_review_item(&topic_id).unwrap().is_none());
        assert!(matches!(
            store.delete_review_item(&topic_id),
            Err(crate::StoreError::ReviewItemNotFound)
        ));
    }

    #[test]
    fn sessions_inherit_subject_and_close_once() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let topic_id = seed_topic(&store, &subject_id, "Unidad 1");

        let session = store
            .start_study_session(None, Some(&topic_id), "pomodoro", Some(25))
            .expect("iniciar");
        assert_eq!(session.subject_id.as_deref(), Some(subject_id.as_str()));
        assert_eq!(session.mode, study_planner_core::StudySessionMode::Pomodoro);
        assert!(session.ended_at.is_none());

        let closed = store
            .finish_study_session(&session.id, Some(20), Some(1), Some("  buena  "))
            .expect("cerrar");
        assert_eq!(closed.actual_minutes, Some(20));
        assert_eq!(closed.interruptions, 1);
        assert_eq!(closed.notes.as_deref(), Some("buena"));
        assert!(closed.ended_at.is_some());

        assert!(matches!(
            store.finish_study_session(&session.id, Some(5), None, None),
            Err(crate::StoreError::InvalidStudy(_))
        ));

        assert_eq!(
            store
                .list_study_sessions(None, None, Some(&subject_id))
                .unwrap()
                .len(),
            1
        );
        store.delete_study_session(&session.id).expect("borrar");
        assert!(
            store
                .list_study_sessions(None, None, None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn sessions_validate_planned_minutes_and_topic_subject() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let topic_id = seed_topic(&store, &subject_id, "Unidad 1");

        assert!(matches!(
            store.start_study_session(None, None, "pomodoro", Some(-1)),
            Err(crate::StoreError::InvalidStudy(_))
        ));
        assert!(matches!(
            store.start_study_session(None, None, "otro", None),
            Err(crate::StoreError::InvalidStudy(_))
        ));
        assert!(matches!(
            store.start_study_session(Some("otra-materia"), Some(&topic_id), "free", None),
            Err(crate::StoreError::SubjectNotFound)
        ));
    }

    #[test]
    fn priority_queue_and_stats_are_deterministic() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let weak_topic = seed_topic(&store, &subject_id, "Unidad débil");
        let strong_topic = seed_topic(&store, &subject_id, "Unidad fuerte");

        store
            .create_self_assessment(&weak_topic, 1, None, "manual")
            .unwrap();
        store
            .create_self_assessment(&strong_topic, 5, None, "manual")
            .unwrap();
        store
            .set_review_due(&weak_topic, "2026-03-01")
            .expect("programar repaso");

        let priorities = store
            .list_study_priorities("2026-03-02")
            .expect("prioridades");
        assert_eq!(priorities.len(), 2);
        assert_eq!(priorities[0].topic_id, weak_topic);
        assert_eq!(priorities[0].reasons[0].code, "weak");
        assert_eq!(priorities[0].reasons[1].code, "review_overdue");

        let session = store
            .start_study_session(Some(&subject_id), None, "free", Some(50))
            .expect("iniciar sesión");
        store
            .finish_study_session(&session.id, Some(45), None, None)
            .expect("cerrar sesión");

        let stats = store.study_stats(None, "2026-03-02").expect("estadísticas");
        assert_eq!(stats.totals.session_count, 1);
        assert_eq!(stats.totals.total_minutes, 45);
        assert_eq!(stats.minutes_by_subject.len(), 1);
        assert_eq!(stats.minutes_by_subject[0].minutes, 45);
        assert_eq!(stats.weak_subjects.len(), 1);
        assert_eq!(stats.weak_subjects[0].subject_id, subject_id);
        assert_eq!(stats.due_reviews, 1);
        assert_eq!(stats.upcoming_reviews.len(), 1);
        assert_eq!(stats.upcoming_reviews[0].topic_id, weak_topic);
    }
}
