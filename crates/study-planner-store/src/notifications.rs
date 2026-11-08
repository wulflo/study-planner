//! Recordatorios locales de clases, tareas, exámenes y repasos.
//!
//! El cálculo es determinista para una hora local dada. La tabla
//! `notification` se usa para deduplicar: cada recordatorio se reclama una
//! sola vez y queda auditado como `fired`. No hay red ni datos de usuario.

use chrono::{DateTime, Datelike, Duration, Local, LocalResult, NaiveDate, NaiveTime, TimeZone};
use rusqlite::{OptionalExtension, Row, params};
use serde::Serialize;
use study_planner_core::{ExamResultStatus, TaskStatus, new_id};

use crate::{Store, StoreError};

const NOTIFICATION_COLUMNS: &str = "id, kind, entity_type, entity_id, scheduled_at, fired_at, \
     state, dedupe_key, created_at, updated_at";

/// Recordatorio listo para entregar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub kind: String,
    pub entity_type: String,
    pub entity_id: String,
    /// Momento ideal de entrega, en UTC RFC 3339.
    pub scheduled_at: String,
    pub dedupe_key: String,
    pub title: String,
    pub body: String,
}

/// Notificación ya registrada en la bóveda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRecord {
    pub id: String,
    pub kind: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub scheduled_at: String,
    pub fired_at: Option<String>,
    pub state: String,
    pub dedupe_key: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Store {
    /// Recordatorios que deben entregarse ahora, según la hora local actual.
    pub fn due_reminders(&self) -> Result<Vec<Reminder>, StoreError> {
        self.due_reminders_at(Local::now())
    }

    /// Reclama un recordatorio para que ninguna otra pasada lo repita.
    ///
    /// Devuelve `None` si ya fue reclamado antes (misma `dedupe_key`).
    pub fn claim_notification(&self, reminder: &Reminder) -> Result<Option<String>, StoreError> {
        let id = new_id();
        let inserted = self.connection().execute(
            "INSERT INTO notification
               (id, kind, entity_type, entity_id, scheduled_at, state, dedupe_key)
             VALUES (?1, ?2, ?3, ?4, ?5, 'scheduled', ?6)
             ON CONFLICT(dedupe_key) DO NOTHING",
            params![
                id,
                reminder.kind,
                reminder.entity_type,
                reminder.entity_id,
                reminder.scheduled_at,
                reminder.dedupe_key
            ],
        )?;
        Ok((inserted > 0).then_some(id))
    }

    /// Marca como entregado un recordatorio previamente reclamado.
    pub fn mark_notification_fired(&self, id: &str) -> Result<(), StoreError> {
        self.connection().execute(
            "UPDATE notification
             SET state = 'fired',
                 fired_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Libera un reclamo que no se pudo entregar, para reintentarlo después.
    pub fn release_notification(&self, id: &str) -> Result<(), StoreError> {
        self.connection()
            .execute("DELETE FROM notification WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Lista las notificaciones entregadas más recientes.
    pub fn list_notifications(&self, limit: i64) -> Result<Vec<NotificationRecord>, StoreError> {
        let limit = limit.clamp(1, 200);
        let mut statement = self.connection().prepare(&format!(
            "SELECT {NOTIFICATION_COLUMNS} FROM notification
             WHERE deleted_at IS NULL AND state = 'fired'
             ORDER BY fired_at DESC, id DESC
             LIMIT ?1"
        ))?;
        let rows = statement.query_map(params![limit], notification_from_row)?;
        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(row?);
        }
        Ok(notifications)
    }

    fn due_reminders_at(&self, now: DateTime<Local>) -> Result<Vec<Reminder>, StoreError> {
        if !setting_bool(self, "notifications.enabled", true) {
            return Ok(Vec::new());
        }
        let class_lead = setting_i64(self, "notifications.class_lead_minutes", 10).max(0);
        let task_lead = setting_i64(self, "notifications.task_lead_minutes", 60).max(0);
        let exam_lead = setting_i64(self, "notifications.exam_lead_hours", 24).max(0);

        let mut reminders = Vec::new();
        reminders.extend(self.class_reminders(now, class_lead)?);
        reminders.extend(self.task_reminders(now, task_lead)?);
        reminders.extend(self.exam_reminders(now, exam_lead)?);
        reminders.extend(self.review_reminders(now)?);

        reminders.sort_by(|left, right| {
            left.scheduled_at
                .cmp(&right.scheduled_at)
                .then(left.dedupe_key.cmp(&right.dedupe_key))
        });
        reminders.truncate(10);
        Ok(reminders)
    }

    fn class_reminders(
        &self,
        now: DateTime<Local>,
        lead_minutes: i64,
    ) -> Result<Vec<Reminder>, StoreError> {
        let today = now.date_naive();
        let today_text = today.format("%Y-%m-%d").to_string();
        let week_start = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        let week_start_text = week_start.format("%Y-%m-%d").to_string();

        let mut reminders = Vec::new();
        for term in self.list_terms(None)? {
            if term.start_date > today_text || term.end_date < today_text {
                continue;
            }
            let schedule = self.week_schedule(&term.id, &week_start_text)?;
            for occurrence in schedule.occurrences {
                let Some(start) = local_datetime(&occurrence.date, Some(&occurrence.start_time))
                else {
                    continue;
                };
                if start < now - Duration::minutes(15) {
                    continue;
                }
                if start > now + Duration::minutes(lead_minutes) {
                    continue;
                }
                let scheduled = start - Duration::minutes(lead_minutes);
                reminders.push(Reminder {
                    kind: "class".to_string(),
                    entity_type: "class_slot".to_string(),
                    entity_id: occurrence.slot_id.clone(),
                    scheduled_at: utc_text(scheduled),
                    dedupe_key: format!(
                        "class:{}:{}:{}",
                        occurrence.slot_id, occurrence.date, occurrence.start_time
                    ),
                    title: format!("Clase: {}", occurrence.subject_name),
                    body: format!(
                        "{} {} · {}",
                        occurrence.date,
                        occurrence.start_time,
                        occurrence.room.as_deref().unwrap_or("sin aula")
                    ),
                });
            }
        }
        Ok(reminders)
    }

    fn task_reminders(
        &self,
        now: DateTime<Local>,
        lead_minutes: i64,
    ) -> Result<Vec<Reminder>, StoreError> {
        let mut reminders = Vec::new();
        for task in self.list_tasks()? {
            if task.status == TaskStatus::Done {
                continue;
            }
            let Some(due_date) = task.due_date.as_deref() else {
                continue;
            };
            let due_time = task.due_time.as_deref().unwrap_or("23:59");
            let Some(due) = local_datetime(due_date, Some(due_time)) else {
                continue;
            };
            if due < now - Duration::hours(24) || due > now + Duration::minutes(lead_minutes) {
                continue;
            }
            let scheduled = due - Duration::minutes(lead_minutes);
            reminders.push(Reminder {
                kind: "task".to_string(),
                entity_type: "task".to_string(),
                entity_id: task.id.clone(),
                scheduled_at: utc_text(scheduled),
                dedupe_key: format!("task:{}:{due_date}:{due_time}", task.id),
                title: format!("Tarea: {}", task.title),
                body: format!("Vence {due_date} {due_time}"),
            });
        }
        Ok(reminders)
    }

    fn exam_reminders(
        &self,
        now: DateTime<Local>,
        lead_hours: i64,
    ) -> Result<Vec<Reminder>, StoreError> {
        let mut reminders = Vec::new();
        for exam in self.list_exams(None, None)? {
            if exam.result_status != ExamResultStatus::Pending {
                continue;
            }
            let start_time = exam.start_time.as_deref().unwrap_or("09:00");
            let Some(start) = local_datetime(&exam.date, Some(start_time)) else {
                continue;
            };
            if start < now - Duration::hours(6) || start > now + Duration::hours(lead_hours) {
                continue;
            }
            let scheduled = start - Duration::hours(lead_hours);
            reminders.push(Reminder {
                kind: "exam".to_string(),
                entity_type: "exam".to_string(),
                entity_id: exam.id.clone(),
                scheduled_at: utc_text(scheduled),
                dedupe_key: format!("exam:{}:{}", exam.id, exam.date),
                title: format!("Examen: {}", exam.title),
                body: format!(
                    "{} {}{}",
                    exam.date,
                    start_time,
                    exam.room
                        .as_deref()
                        .map(|room| format!(" · {room}"))
                        .unwrap_or_default()
                ),
            });
        }
        Ok(reminders)
    }

    fn review_reminders(&self, now: DateTime<Local>) -> Result<Vec<Reminder>, StoreError> {
        let today = now.date_naive();
        let today_text = today.format("%Y-%m-%d").to_string();
        let mut reminders = Vec::new();
        for review in self.list_review_items(None)? {
            let Some(due_date) = review.due_date.as_deref() else {
                continue;
            };
            if due_date > today_text.as_str() {
                continue;
            }
            let Some(scheduled) = local_datetime(due_date, Some("09:00")) else {
                continue;
            };
            if scheduled > now {
                continue;
            }
            let topic = self
                .topic_name(&review.topic_id)
                .unwrap_or_else(|| "tema sin nombre".to_string());
            reminders.push(Reminder {
                kind: "review".to_string(),
                entity_type: "review_item".to_string(),
                entity_id: review.id.clone(),
                scheduled_at: utc_text(scheduled),
                dedupe_key: format!("review:{}:{due_date}", review.id),
                title: format!("Repaso: {topic}"),
                body: format!("Repaso pendiente desde {due_date}"),
            });
        }
        Ok(reminders)
    }

    fn topic_name(&self, topic_id: &str) -> Option<String> {
        self.connection()
            .query_row(
                "SELECT name FROM topic WHERE id = ?1 AND deleted_at IS NULL",
                params![topic_id],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }
}

fn notification_from_row(row: &Row<'_>) -> rusqlite::Result<NotificationRecord> {
    Ok(NotificationRecord {
        id: row.get(0)?,
        kind: row.get(1)?,
        entity_type: row.get(2)?,
        entity_id: row.get(3)?,
        scheduled_at: row.get(4)?,
        fired_at: row.get(5)?,
        state: row.get(6)?,
        dedupe_key: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn setting_bool(store: &Store, key: &str, default: bool) -> bool {
    store
        .get_setting(key)
        .ok()
        .flatten()
        .map_or(default, |value| value.trim() == "true")
}

fn setting_i64(store: &Store, key: &str, default: i64) -> i64 {
    store
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(default)
}

fn parse_time(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value.trim(), "%H:%M:%S"))
        .ok()
}

fn local_datetime(date: &str, time: Option<&str>) -> Option<DateTime<Local>> {
    let date = NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok()?;
    let time = time
        .and_then(parse_time)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).expect("hora de fin de día válida"));
    let naive = date.and_time(time);
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Some(value),
        LocalResult::Ambiguous(earliest, _) => Some(earliest),
        LocalResult::None => None,
    }
}

fn utc_text(value: DateTime<Local>) -> String {
    value
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::migrated_store_with_subject;
    use crate::{ClassSlotInput, ExamInput, TaskInput, TopicInput};

    fn fixed_now() -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 5, 12, 9, 0, 0)
            .single()
            .expect("hora local válida")
    }

    fn seed_reminders(store: &Store, subject_id: &str, term_id: &str) {
        store
            .create_class_slot(
                subject_id,
                &ClassSlotInput {
                    weekday: 2,
                    start_time: "09:05".to_string(),
                    end_time: "09:55".to_string(),
                    recurrence: "weekly".to_string(),
                    rotation_key: None,
                    valid_from: None,
                    valid_to: None,
                    room_override: Some("Aula 2".to_string()),
                },
            )
            .expect("crear franja");

        store
            .create_task_full(&TaskInput {
                title: "Entregar práctico".to_string(),
                due_date: Some("2026-05-12".to_string()),
                due_time: Some("09:30".to_string()),
                ..TaskInput::default()
            })
            .expect("crear tarea");

        store
            .create_exam(
                subject_id,
                term_id,
                &ExamInput {
                    title: "Parcial 1".to_string(),
                    date: "2026-05-13".to_string(),
                    start_time: Some("09:00".to_string()),
                    end_time: Some("11:00".to_string()),
                    ..ExamInput::default()
                },
            )
            .expect("crear examen");

        let topic = store
            .create_topic(
                subject_id,
                &TopicInput {
                    name: "Matrices".to_string(),
                    ..TopicInput::default()
                },
            )
            .expect("crear tema");
        store
            .set_review_due(&topic.id, "2026-05-12")
            .expect("programar repaso");
    }

    #[test]
    fn builds_reminders_for_all_four_kinds() {
        let (store, subject_id, term_id) = migrated_store_with_subject();
        seed_reminders(&store, &subject_id, &term_id);

        let reminders = store
            .due_reminders_at(fixed_now())
            .expect("calcular recordatorios");
        let kinds: Vec<&str> = reminders.iter().map(|item| item.kind.as_str()).collect();
        assert!(kinds.contains(&"class"));
        assert!(kinds.contains(&"task"));
        assert!(kinds.contains(&"exam"));
        assert!(kinds.contains(&"review"));
        assert!(
            reminders
                .iter()
                .any(|item| item.title.contains("Parcial 1"))
        );
    }

    #[test]
    fn disabled_notifications_produce_no_reminders() {
        let (store, subject_id, term_id) = migrated_store_with_subject();
        seed_reminders(&store, &subject_id, &term_id);
        store
            .set_setting("notifications.enabled", "false", "portable")
            .expect("desactivar");

        assert!(
            store
                .due_reminders_at(fixed_now())
                .expect("calcular")
                .is_empty()
        );
    }

    #[test]
    fn claims_and_fires_each_reminder_only_once() {
        let (store, subject_id, term_id) = migrated_store_with_subject();
        seed_reminders(&store, &subject_id, &term_id);
        let reminder = store
            .due_reminders_at(fixed_now())
            .expect("calcular")
            .into_iter()
            .find(|item| item.kind == "task")
            .expect("tarea pendiente");

        let claim = store
            .claim_notification(&reminder)
            .expect("reclamar")
            .expect("primer reclamo");
        assert!(
            store
                .claim_notification(&reminder)
                .expect("reclamar de nuevo")
                .is_none()
        );

        store.mark_notification_fired(&claim).expect("marcar");
        let fired = store.list_notifications(10).expect("listar");
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].kind, "task");
        assert_eq!(fired[0].state, "fired");
    }

    #[test]
    fn releases_a_failed_claim_for_retry() {
        let (store, subject_id, term_id) = migrated_store_with_subject();
        seed_reminders(&store, &subject_id, &term_id);
        let reminder = store
            .due_reminders_at(fixed_now())
            .expect("calcular")
            .into_iter()
            .next()
            .expect("recordatorio");

        let claim = store
            .claim_notification(&reminder)
            .expect("reclamar")
            .expect("primer reclamo");
        store.release_notification(&claim).expect("liberar");

        assert!(
            store
                .claim_notification(&reminder)
                .expect("reclamar de nuevo")
                .is_some()
        );
    }
}
