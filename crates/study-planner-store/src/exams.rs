//! Repositorio de exámenes, temas y vínculos examen–tema.
//!
//! Los exámenes pertenecen a una materia y a un término. Los temas forman un
//! árbol por materia y nunca pueden crear ciclos. Todo usa borrado lógico.

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};
use serde::Deserialize;
use study_planner_core::{
    Exam, ExamResultStatus, ExamTopic, Topic, new_id, normalize_date, normalize_optional_text,
    topic_would_create_cycle, validate_basis_points, validate_exam_times, validate_exam_title,
    validate_grade_entry, validate_topic_name,
};

use crate::{Store, StoreError};

const EXAM_COLUMNS: &str = "id, subject_id, term_id, title, date, start_time, end_time, room, \
     weight_bp, result_score_cp, result_max_cp, result_status, notes, created_at, updated_at";
const TOPIC_COLUMNS: &str = "id, subject_id, parent_topic_id, name, description, sort_order, \
     created_at, updated_at";
const EXAM_TOPIC_COLUMNS: &str = "id, exam_id, topic_id, created_at, updated_at";

/// Datos editables de un examen.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExamInput {
    pub title: String,
    pub date: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub room: Option<String>,
    pub weight_bp: Option<i64>,
    pub result_score_cp: Option<i64>,
    pub result_max_cp: Option<i64>,
    pub result_status: Option<String>,
    pub notes: Option<String>,
}

/// Datos editables de un tema.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TopicInput {
    pub parent_topic_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: Option<i64>,
}

impl Store {
    /// Lista exámenes vivos, opcionalmente por término o materia.
    pub fn list_exams(
        &self,
        term_id: Option<&str>,
        subject_id: Option<&str>,
    ) -> Result<Vec<Exam>, StoreError> {
        let mut sql = format!("SELECT {EXAM_COLUMNS} FROM exam WHERE deleted_at IS NULL");
        let mut values: Vec<Value> = Vec::new();

        if let Some(term_id) = term_id {
            sql.push_str(" AND term_id = ?");
            values.push(Value::Text(term_id.to_string()));
        }
        if let Some(subject_id) = subject_id {
            sql.push_str(" AND subject_id = ?");
            values.push(Value::Text(subject_id.to_string()));
        }
        sql.push_str(" ORDER BY date, start_time, title");

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), exam_from_row)?;
        let mut exams = Vec::new();
        for row in rows {
            exams.push(row?);
        }
        Ok(exams)
    }

    /// Crea un examen validado.
    pub fn create_exam(
        &self,
        subject_id: &str,
        term_id: &str,
        input: &ExamInput,
    ) -> Result<Exam, StoreError> {
        self.ensure_live_subject(subject_id)?;
        self.ensure_live_term(term_id)?;
        let values = validate_exam_input(input)?;

        let id = new_id();
        self.connection().execute(
            "INSERT INTO exam
               (id, subject_id, term_id, title, date, start_time, end_time, room,
                weight_bp, result_score_cp, result_max_cp, result_status, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                subject_id,
                term_id,
                values.title,
                values.date,
                values.start_time,
                values.end_time,
                values.room,
                values.weight_bp,
                values.result_score_cp,
                values.result_max_cp,
                values.result_status,
                values.notes,
            ],
        )?;

        self.get_exam(&id)?.ok_or(StoreError::ExamNotFound)
    }

    /// Actualiza un examen vivo.
    pub fn update_exam(&self, id: &str, input: &ExamInput) -> Result<Exam, StoreError> {
        let current = self.get_exam(id)?.ok_or(StoreError::ExamNotFound)?;
        self.ensure_live_subject(&current.subject_id)?;
        self.ensure_live_term(&current.term_id)?;
        let values = validate_exam_input(input)?;

        let changed = self.connection().execute(
            "UPDATE exam
             SET title = ?1, date = ?2, start_time = ?3, end_time = ?4, room = ?5,
                 weight_bp = ?6, result_score_cp = ?7, result_max_cp = ?8,
                 result_status = ?9, notes = ?10,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?11 AND deleted_at IS NULL",
            params![
                values.title,
                values.date,
                values.start_time,
                values.end_time,
                values.room,
                values.weight_bp,
                values.result_score_cp,
                values.result_max_cp,
                values.result_status,
                values.notes,
                id,
            ],
        )?;

        if changed == 0 {
            return Err(StoreError::ExamNotFound);
        }
        self.get_exam(id)?.ok_or(StoreError::ExamNotFound)
    }

    /// Borra lógicamente un examen y sus vínculos con temas.
    pub fn delete_exam(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE exam
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::ExamNotFound);
        }

        self.connection().execute(
            "UPDATE exam_topic
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        Ok(())
    }

    /// Lista los temas vivos de una materia, en orden de presentación.
    pub fn list_topics(&self, subject_id: &str) -> Result<Vec<Topic>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {TOPIC_COLUMNS}
             FROM topic
             WHERE subject_id = ?1 AND deleted_at IS NULL
             ORDER BY sort_order, name"
        ))?;
        let rows = statement.query_map(params![subject_id], topic_from_row)?;
        let mut topics = Vec::new();
        for row in rows {
            topics.push(row?);
        }
        Ok(topics)
    }

    /// Crea un tema y valida su jerarquía.
    pub fn create_topic(&self, subject_id: &str, input: &TopicInput) -> Result<Topic, StoreError> {
        self.ensure_live_subject(subject_id)?;
        let values = validate_topic_input(input)?;
        if let Some(parent_id) = &values.parent_topic_id {
            self.ensure_live_topic(parent_id)?;
            let topics = self.list_topics(subject_id)?;
            let parent = topics
                .iter()
                .find(|topic| topic.id == *parent_id)
                .ok_or(StoreError::TopicNotFound)?;
            if parent.subject_id != subject_id {
                return Err(StoreError::InvalidExam(
                    "El tema padre pertenece a otra materia.",
                ));
            }
        }

        let id = new_id();
        self.connection().execute(
            "INSERT INTO topic (id, subject_id, parent_topic_id, name, description, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                subject_id,
                values.parent_topic_id,
                values.name,
                values.description,
                values.sort_order,
            ],
        )?;

        self.get_topic(&id)?.ok_or(StoreError::TopicNotFound)
    }

    /// Actualiza un tema vivo sin permitir ciclos.
    pub fn update_topic(&self, id: &str, input: &TopicInput) -> Result<Topic, StoreError> {
        let current = self.get_topic(id)?.ok_or(StoreError::TopicNotFound)?;
        let values = validate_topic_input(input)?;

        if let Some(parent_id) = &values.parent_topic_id {
            if parent_id == id {
                return Err(StoreError::InvalidExam(
                    "Un tema no puede ser su propio padre.",
                ));
            }
            let parent = self
                .get_topic(parent_id)?
                .ok_or(StoreError::TopicNotFound)?;
            if parent.subject_id != current.subject_id {
                return Err(StoreError::InvalidExam(
                    "El tema padre pertenece a otra materia.",
                ));
            }
            let topics = self.list_topics(&current.subject_id)?;
            if topic_would_create_cycle(&topics, id, parent_id) {
                return Err(StoreError::InvalidExam(
                    "El cambio de jerarquía crearía un ciclo.",
                ));
            }
        }

        let changed = self.connection().execute(
            "UPDATE topic
             SET parent_topic_id = ?1, name = ?2, description = ?3, sort_order = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?5 AND deleted_at IS NULL",
            params![
                values.parent_topic_id,
                values.name,
                values.description,
                values.sort_order,
                id,
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::TopicNotFound);
        }
        self.get_topic(id)?.ok_or(StoreError::TopicNotFound)
    }

    /// Borra lógicamente un tema sin hijos vivos.
    pub fn delete_topic(&self, id: &str) -> Result<(), StoreError> {
        let children: i64 = self.connection().query_row(
            "SELECT count(*) FROM topic WHERE parent_topic_id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        if children > 0 {
            return Err(StoreError::Conflict(
                "El tema tiene subtemas vivos; borralos o movelos primero.",
            ));
        }

        let changed = self.connection().execute(
            "UPDATE topic
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::TopicNotFound);
        }

        self.connection().execute(
            "UPDATE exam_topic
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        Ok(())
    }

    /// Lista los vínculos vivos de un examen con temas.
    pub fn list_exam_topics(&self, exam_id: &str) -> Result<Vec<ExamTopic>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {EXAM_TOPIC_COLUMNS}
             FROM exam_topic
             WHERE exam_id = ?1 AND deleted_at IS NULL
             ORDER BY created_at"
        ))?;
        let rows = statement.query_map(params![exam_id], |row| {
            Ok(ExamTopic {
                id: row.get(0)?,
                exam_id: row.get(1)?,
                topic_id: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut links = Vec::new();
        for row in rows {
            links.push(row?);
        }
        Ok(links)
    }

    /// Vincula un examen con un tema de la misma materia.
    pub fn link_exam_topic(&self, exam_id: &str, topic_id: &str) -> Result<ExamTopic, StoreError> {
        let exam = self.get_exam(exam_id)?.ok_or(StoreError::ExamNotFound)?;
        let topic = self.get_topic(topic_id)?.ok_or(StoreError::TopicNotFound)?;
        if exam.subject_id != topic.subject_id {
            return Err(StoreError::InvalidExam(
                "El tema y el examen pertenecen a materias distintas.",
            ));
        }

        let existing: Option<String> = self
            .connection()
            .query_row(
                "SELECT id FROM exam_topic
                 WHERE exam_id = ?1 AND topic_id = ?2 AND deleted_at IS NULL",
                params![exam_id, topic_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            return self
                .get_exam_topic(&id)?
                .ok_or(StoreError::ExamTopicNotFound);
        }

        let id = new_id();
        self.connection().execute(
            "INSERT INTO exam_topic (id, exam_id, topic_id) VALUES (?1, ?2, ?3)",
            params![id, exam_id, topic_id],
        )?;
        self.get_exam_topic(&id)?
            .ok_or(StoreError::ExamTopicNotFound)
    }

    /// Quita el vínculo vivo entre un examen y un tema.
    pub fn unlink_exam_topic(&self, exam_id: &str, topic_id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE exam_topic
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = ?1 AND topic_id = ?2 AND deleted_at IS NULL",
            params![exam_id, topic_id],
        )?;
        if changed == 0 {
            return Err(StoreError::ExamTopicNotFound);
        }
        Ok(())
    }

    fn get_exam(&self, id: &str) -> Result<Option<Exam>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!("SELECT {EXAM_COLUMNS} FROM exam WHERE id = ?1 AND deleted_at IS NULL"),
                params![id],
                exam_from_row,
            )
            .optional()?)
    }

    fn get_topic(&self, id: &str) -> Result<Option<Topic>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!("SELECT {TOPIC_COLUMNS} FROM topic WHERE id = ?1 AND deleted_at IS NULL"),
                params![id],
                topic_from_row,
            )
            .optional()?)
    }

    fn get_exam_topic(&self, id: &str) -> Result<Option<ExamTopic>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!(
                    "SELECT {EXAM_TOPIC_COLUMNS} FROM exam_topic
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                |row| {
                    Ok(ExamTopic {
                        id: row.get(0)?,
                        exam_id: row.get(1)?,
                        topic_id: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    fn ensure_live_term(&self, term_id: &str) -> Result<(), StoreError> {
        let exists: i64 = self.connection().query_row(
            "SELECT count(*) FROM term WHERE id = ?1 AND deleted_at IS NULL",
            params![term_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(StoreError::TermNotFound);
        }
        Ok(())
    }

    fn ensure_live_topic(&self, topic_id: &str) -> Result<(), StoreError> {
        self.get_topic(topic_id)?
            .map(|_| ())
            .ok_or(StoreError::TopicNotFound)
    }
}

struct ExamValues {
    title: String,
    date: String,
    start_time: Option<String>,
    end_time: Option<String>,
    room: Option<String>,
    weight_bp: Option<i64>,
    result_score_cp: Option<i64>,
    result_max_cp: Option<i64>,
    result_status: String,
    notes: Option<String>,
}

fn validate_exam_input(input: &ExamInput) -> Result<ExamValues, StoreError> {
    let title = validate_exam_title(&input.title).map_err(StoreError::InvalidExam)?;
    let date = normalize_date(&input.date).map_err(StoreError::InvalidExam)?;
    let (start_time, end_time) =
        validate_exam_times(input.start_time.as_deref(), input.end_time.as_deref())
            .map_err(StoreError::InvalidExam)?;
    let room = input
        .room
        .as_deref()
        .map(normalize_optional_text)
        .transpose()
        .map_err(StoreError::InvalidExam)?
        .flatten();
    let notes = input
        .notes
        .as_deref()
        .map(normalize_optional_text)
        .transpose()
        .map_err(StoreError::InvalidExam)?
        .flatten();

    let weight_bp = match input.weight_bp {
        Some(value) => Some(validate_basis_points(value).map_err(StoreError::InvalidExam)?),
        None => None,
    };

    let status = match input.result_status.as_deref() {
        None | Some("") => ExamResultStatus::Pending,
        Some(value) => ExamResultStatus::from_db(value)
            .ok_or(StoreError::InvalidExam("Estado de resultado inválido."))?,
    };

    let (result_score_cp, result_max_cp) = match (input.result_score_cp, input.result_max_cp) {
        (None, None) => (None, None),
        (Some(score), Some(max)) => {
            validate_grade_entry(score, max).map_err(StoreError::InvalidExam)?;
            (Some(score), Some(max))
        }
        _ => {
            return Err(StoreError::InvalidExam(
                "El resultado necesita nota y máximo, o ninguno.",
            ));
        }
    };

    Ok(ExamValues {
        title,
        date,
        start_time,
        end_time,
        room,
        weight_bp,
        result_score_cp,
        result_max_cp,
        result_status: status.as_str().to_string(),
        notes,
    })
}

struct TopicValues {
    parent_topic_id: Option<String>,
    name: String,
    description: Option<String>,
    sort_order: i64,
}

fn validate_topic_input(input: &TopicInput) -> Result<TopicValues, StoreError> {
    let name = validate_topic_name(&input.name).map_err(StoreError::InvalidExam)?;
    let description = input
        .description
        .as_deref()
        .map(normalize_optional_text)
        .transpose()
        .map_err(StoreError::InvalidExam)?
        .flatten();
    let sort_order = input.sort_order.unwrap_or(0);
    if sort_order < 0 {
        return Err(StoreError::InvalidExam(
            "El orden del tema no puede ser negativo.",
        ));
    }
    let parent_topic_id = input
        .parent_topic_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Ok(TopicValues {
        parent_topic_id,
        name,
        description,
        sort_order,
    })
}

fn exam_from_row(row: &Row<'_>) -> rusqlite::Result<Exam> {
    let status: String = row.get(11)?;
    Ok(Exam {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        term_id: row.get(2)?,
        title: row.get(3)?,
        date: row.get(4)?,
        start_time: row.get(5)?,
        end_time: row.get(6)?,
        room: row.get(7)?,
        weight_bp: row.get(8)?,
        result_score_cp: row.get(9)?,
        result_max_cp: row.get(10)?,
        result_status: ExamResultStatus::from_db(&status).unwrap_or(ExamResultStatus::Pending),
        notes: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn topic_from_row(row: &Row<'_>) -> rusqlite::Result<Topic> {
    Ok(Topic {
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
    use super::*;
    use crate::tests_support::migrated_store_with_subject;

    fn exam_input(title: &str, date: &str) -> ExamInput {
        ExamInput {
            title: title.to_string(),
            date: date.to_string(),
            ..ExamInput::default()
        }
    }

    #[test]
    fn creates_updates_and_deletes_exams() {
        let (store, subject_id, term_id) = migrated_store_with_subject();

        let exam = store
            .create_exam(
                &subject_id,
                &term_id,
                &ExamInput {
                    weight_bp: Some(4_000),
                    start_time: Some("08:00".to_string()),
                    end_time: Some("10:00".to_string()),
                    ..exam_input("  Parcial 1  ", "2026-05-10")
                },
            )
            .unwrap();

        assert_eq!(exam.title, "Parcial 1");
        assert_eq!(exam.weight_bp, Some(4_000));
        assert_eq!(store.list_exams(Some(&term_id), None).unwrap().len(), 1);

        let updated = store
            .update_exam(
                &exam.id,
                &ExamInput {
                    result_score_cp: Some(8_500),
                    result_max_cp: Some(10_000),
                    result_status: Some("passed".to_string()),
                    ..exam_input("Parcial 1 bis", "2026-05-11")
                },
            )
            .unwrap();
        assert_eq!(updated.title, "Parcial 1 bis");
        assert_eq!(updated.result_status, ExamResultStatus::Passed);

        store.delete_exam(&exam.id).unwrap();
        assert!(store.list_exams(None, None).unwrap().is_empty());
    }

    #[test]
    fn validates_exam_results_and_times() {
        let (store, subject_id, term_id) = migrated_store_with_subject();

        let error = store
            .create_exam(
                &subject_id,
                &term_id,
                &ExamInput {
                    result_score_cp: Some(10),
                    ..exam_input("Parcial", "2026-05-10")
                },
            )
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidExam(_)));

        let error = store
            .create_exam(
                &subject_id,
                &term_id,
                &ExamInput {
                    start_time: Some("08:00".to_string()),
                    ..exam_input("Parcial", "2026-05-10")
                },
            )
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidExam(_)));
    }

    #[test]
    fn manages_topics_without_cycles() {
        let (store, subject_id, _) = migrated_store_with_subject();

        let root = store
            .create_topic(
                &subject_id,
                &TopicInput {
                    name: "Unidad 1".to_string(),
                    ..TopicInput::default()
                },
            )
            .unwrap();
        let child = store
            .create_topic(
                &subject_id,
                &TopicInput {
                    parent_topic_id: Some(root.id.clone()),
                    name: "Tema 1.1".to_string(),
                    ..TopicInput::default()
                },
            )
            .unwrap();

        let error = store
            .update_topic(
                &root.id,
                &TopicInput {
                    parent_topic_id: Some(child.id.clone()),
                    name: "Unidad 1".to_string(),
                    ..TopicInput::default()
                },
            )
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidExam(_)));

        let error = store.delete_topic(&root.id).unwrap_err();
        assert!(matches!(error, StoreError::Conflict(_)));

        store.delete_topic(&child.id).unwrap();
        store.delete_topic(&root.id).unwrap();
        assert!(store.list_topics(&subject_id).unwrap().is_empty());
    }

    #[test]
    fn links_exams_and_topics() {
        let (store, subject_id, term_id) = migrated_store_with_subject();
        let exam = store
            .create_exam(&subject_id, &term_id, &exam_input("Parcial", "2026-05-10"))
            .unwrap();
        let topic = store
            .create_topic(
                &subject_id,
                &TopicInput {
                    name: "Unidad 1".to_string(),
                    ..TopicInput::default()
                },
            )
            .unwrap();

        let link = store.link_exam_topic(&exam.id, &topic.id).unwrap();
        assert_eq!(store.list_exam_topics(&exam.id).unwrap().len(), 1);
        let again = store.link_exam_topic(&exam.id, &topic.id).unwrap();
        assert_eq!(again.id, link.id);

        store.unlink_exam_topic(&exam.id, &topic.id).unwrap();
        assert!(store.list_exam_topics(&exam.id).unwrap().is_empty());
    }
}
