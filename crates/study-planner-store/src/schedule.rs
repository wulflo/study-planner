//! Repositorio de horario: franjas, rotaciones, feriados, excepciones y
//! asistencia, más la composición de la vista semanal.
//!
//! La matemática de ocurrencias vive en `study-planner-core`; acá solo se
//! cargan las filas y se arma el resultado que consume la UI.

use serde::Serialize;
use study_planner_core::{
    AttendanceRecord, AttendanceStatus, AttendanceSummary, ClassException, ClassSlot,
    ExceptionKind, Holiday, Recurrence, RotationConfig, RotationMode, ScheduleOccurrence,
    ScheduleSlot, SlotOverlap, add_days, attendance_summary, find_overlaps, next_class,
    normalize_date, normalize_name, normalize_optional_short, normalize_optional_text,
    normalize_time, validate_date_range, validate_time_range, validate_weekday, week_occurrences,
    week_start_of,
};

use rusqlite::OptionalExtension;

use crate::{Store, StoreError};

const SLOT_COLUMNS: &str = "id, subject_id, weekday, start_time, end_time, recurrence, \
     rotation_key, valid_from, valid_to, room_override, created_at, updated_at";
const HOLIDAY_COLUMNS: &str = "id, term_id, date, name, created_at, updated_at";
const EXCEPTION_COLUMNS: &str = "id, class_slot_id, date, kind, new_start_time, \
     new_end_time, new_room, created_at, updated_at";
const ATTENDANCE_COLUMNS: &str = "id, class_slot_id, date, status, note, created_at, updated_at";

/// Datos editables de una franja horaria.
#[derive(Debug, Clone, Default)]
pub struct ClassSlotInput {
    pub weekday: u8,
    pub start_time: String,
    pub end_time: String,
    pub recurrence: String,
    pub rotation_key: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub room_override: Option<String>,
}

/// Datos editables de una rotación.
#[derive(Debug, Clone, Default)]
pub struct RotationInput {
    pub mode: String,
    pub anchor_date: String,
    pub cycle_length: u32,
    pub labels: Vec<String>,
    pub day_cycle: Vec<String>,
}

/// Feriado visible en la semana.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekHoliday {
    pub date: String,
    pub name: String,
}

/// Resultado completo de una semana de un término.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekSchedule {
    pub term_id: String,
    pub week_start: String,
    pub week_end: String,
    pub rotation_label: Option<String>,
    pub holidays: Vec<WeekHoliday>,
    pub occurrences: Vec<ScheduleOccurrence>,
    pub overlaps: Vec<SlotOverlap>,
    pub attendance: Vec<AttendanceSummary>,
}

impl Store {
    /// Lista las franjas vivas de un término.
    pub fn list_class_slots(&self, term_id: &str) -> Result<Vec<ClassSlot>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {SLOT_COLUMNS}
             FROM class_slot
             WHERE subject_id IN (
               SELECT id FROM subject WHERE term_id = ?1 AND deleted_at IS NULL
             )
             AND deleted_at IS NULL
             ORDER BY weekday, start_time"
        ))?;
        let rows = statement.query_map(rusqlite::params![term_id], class_slot_from_row)?;
        let mut slots = Vec::new();
        for row in rows {
            slots.push(row?);
        }
        Ok(slots)
    }

    /// Lista las franjas con el nombre y color de su materia resueltos.
    pub fn list_schedule_slots(&self, term_id: &str) -> Result<Vec<ScheduleSlot>, StoreError> {
        let mut statement = self.connection().prepare(
            "SELECT cs.id, cs.subject_id, cs.weekday, cs.start_time, cs.end_time,
                    cs.recurrence, cs.rotation_key, cs.valid_from, cs.valid_to,
                    cs.room_override, cs.created_at, cs.updated_at,
                    s.name, s.color
             FROM class_slot cs
             JOIN subject s ON s.id = cs.subject_id
             WHERE s.term_id = ?1
               AND s.deleted_at IS NULL
               AND cs.deleted_at IS NULL
             ORDER BY cs.weekday, cs.start_time",
        )?;

        let rows = statement.query_map(rusqlite::params![term_id], |row| {
            Ok(ScheduleSlot {
                slot: class_slot_from_row(row)?,
                subject_name: row.get(12)?,
                subject_color: row.get(13)?,
            })
        })?;

        let mut slots = Vec::new();
        for row in rows {
            slots.push(row?);
        }
        Ok(slots)
    }

    /// Crea una franja dentro de una materia viva.
    pub fn create_class_slot(
        &self,
        subject_id: &str,
        input: &ClassSlotInput,
    ) -> Result<ClassSlot, StoreError> {
        self.get_subject(subject_id)?
            .ok_or(StoreError::SubjectNotFound)?;
        let values = validate_slot_input(input)?;
        let id = study_planner_core::new_id();

        self.connection().execute(
            "INSERT INTO class_slot
               (id, subject_id, weekday, start_time, end_time, recurrence,
                rotation_key, valid_from, valid_to, room_override)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id,
                subject_id,
                values.weekday,
                values.start_time,
                values.end_time,
                values.recurrence,
                values.rotation_key,
                values.valid_from,
                values.valid_to,
                values.room_override,
            ],
        )?;

        self.get_class_slot(&id)?
            .ok_or(StoreError::ClassSlotNotFound)
    }

    /// Actualiza una franja viva.
    pub fn update_class_slot(
        &self,
        id: &str,
        input: &ClassSlotInput,
    ) -> Result<ClassSlot, StoreError> {
        let values = validate_slot_input(input)?;

        let changed = self.connection().execute(
            "UPDATE class_slot
             SET weekday = ?1, start_time = ?2, end_time = ?3, recurrence = ?4,
                 rotation_key = ?5, valid_from = ?6, valid_to = ?7, room_override = ?8,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?9 AND deleted_at IS NULL",
            rusqlite::params![
                values.weekday,
                values.start_time,
                values.end_time,
                values.recurrence,
                values.rotation_key,
                values.valid_from,
                values.valid_to,
                values.room_override,
                id,
            ],
        )?;

        if changed == 0 {
            return Err(StoreError::ClassSlotNotFound);
        }

        self.get_class_slot(id)?
            .ok_or(StoreError::ClassSlotNotFound)
    }

    /// Borra lógicamente una franja.
    pub fn delete_class_slot(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE class_slot
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![id],
        )?;

        if changed == 0 {
            return Err(StoreError::ClassSlotNotFound);
        }
        Ok(())
    }

    /// Rotación viva de un término, si existe.
    pub fn get_rotation(&self, term_id: &str) -> Result<Option<RotationConfig>, StoreError> {
        let rotation = self
            .connection()
            .query_row(
                "SELECT id, term_id, mode, anchor_date, cycle_length, labels_json,
                        day_cycle_json, created_at, updated_at
                 FROM rotation_config
                 WHERE term_id = ?1 AND deleted_at IS NULL
                 ORDER BY created_at DESC
                 LIMIT 1",
                rusqlite::params![term_id],
                rotation_from_row,
            )
            .optional()?;
        Ok(rotation)
    }

    /// Crea o actualiza la rotación de un término.
    pub fn save_rotation(
        &self,
        term_id: &str,
        input: &RotationInput,
    ) -> Result<RotationConfig, StoreError> {
        self.get_term(term_id)?.ok_or(StoreError::TermNotFound)?;
        let mode = RotationMode::from_db(&input.mode)
            .ok_or(StoreError::InvalidSchedule("Modo de rotación inválido."))?;
        let anchor_date =
            normalize_date(&input.anchor_date).map_err(StoreError::InvalidSchedule)?;

        if !(1..=12).contains(&input.cycle_length) {
            return Err(StoreError::InvalidSchedule(
                "El ciclo debe tener entre 1 y 12 posiciones.",
            ));
        }

        let labels = normalize_labels(&input.labels)?;
        let day_cycle = normalize_labels(&input.day_cycle)?;

        if mode == RotationMode::DayCycle {
            if day_cycle.is_empty() {
                return Err(StoreError::InvalidSchedule(
                    "El ciclo de días necesita al menos una etiqueta.",
                ));
            }
            if day_cycle.len() as u32 != input.cycle_length {
                return Err(StoreError::InvalidSchedule(
                    "La cantidad de etiquetas del ciclo no coincide con cycle_length.",
                ));
            }
        } else if !labels.is_empty() && labels.len() as u32 != input.cycle_length {
            return Err(StoreError::InvalidSchedule(
                "La cantidad de etiquetas no coincide con cycle_length.",
            ));
        }

        let labels_json = serde_json::to_string(&labels).unwrap_or_else(|_| "[]".to_string());
        let day_cycle_json = serde_json::to_string(&day_cycle).unwrap_or_else(|_| "[]".to_string());

        if let Some(existing) = self.get_rotation(term_id)? {
            self.connection().execute(
                "UPDATE rotation_config
                 SET mode = ?1, anchor_date = ?2, cycle_length = ?3,
                     labels_json = ?4, day_cycle_json = ?5,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?6",
                rusqlite::params![
                    mode.as_str(),
                    anchor_date,
                    input.cycle_length,
                    labels_json,
                    day_cycle_json,
                    existing.id,
                ],
            )?;
            return self
                .get_rotation(term_id)?
                .ok_or(StoreError::RotationNotFound);
        }

        let id = study_planner_core::new_id();
        self.connection().execute(
            "INSERT INTO rotation_config
               (id, term_id, mode, anchor_date, cycle_length, labels_json, day_cycle_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                term_id,
                mode.as_str(),
                anchor_date,
                input.cycle_length,
                labels_json,
                day_cycle_json,
            ],
        )?;

        self.get_rotation(term_id)?
            .ok_or(StoreError::RotationNotFound)
    }

    /// Borra lógicamente la rotación de un término.
    pub fn delete_rotation(&self, term_id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE rotation_config
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE term_id = ?1 AND deleted_at IS NULL",
            rusqlite::params![term_id],
        )?;

        if changed == 0 {
            return Err(StoreError::RotationNotFound);
        }
        Ok(())
    }

    /// Lista los feriados del término y los globales.
    pub fn list_holidays(&self, term_id: &str) -> Result<Vec<Holiday>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {HOLIDAY_COLUMNS}
             FROM holiday
             WHERE deleted_at IS NULL AND (term_id = ?1 OR term_id IS NULL)
             ORDER BY date"
        ))?;
        let rows = statement.query_map(rusqlite::params![term_id], holiday_from_row)?;
        let mut holidays = Vec::new();
        for row in rows {
            holidays.push(row?);
        }
        Ok(holidays)
    }

    /// Crea un feriado global (`None`) o de un término.
    pub fn create_holiday(
        &self,
        term_id: Option<&str>,
        date: &str,
        name: &str,
    ) -> Result<Holiday, StoreError> {
        if let Some(term_id) = term_id {
            self.get_term(term_id)?.ok_or(StoreError::TermNotFound)?;
        }
        let date = normalize_date(date).map_err(StoreError::InvalidSchedule)?;
        let name = normalize_name(name).map_err(StoreError::InvalidSchedule)?;
        let id = study_planner_core::new_id();

        self.connection().execute(
            "INSERT INTO holiday (id, term_id, date, name) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, term_id, date, name],
        )?;

        self.get_holiday(&id)?.ok_or(StoreError::HolidayNotFound)
    }

    /// Borra lógicamente un feriado.
    pub fn delete_holiday(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE holiday
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![id],
        )?;

        if changed == 0 {
            return Err(StoreError::HolidayNotFound);
        }
        Ok(())
    }

    /// Lista las excepciones de las franjas de un término.
    pub fn list_class_exceptions(&self, term_id: &str) -> Result<Vec<ClassException>, StoreError> {
        let mut statement = self.connection().prepare(
            "SELECT ce.id, ce.class_slot_id, ce.date, ce.kind, ce.new_start_time,
                    ce.new_end_time, ce.new_room, ce.created_at, ce.updated_at
             FROM class_exception ce
             JOIN class_slot cs ON cs.id = ce.class_slot_id
             JOIN subject s ON s.id = cs.subject_id
             WHERE s.term_id = ?1
               AND s.deleted_at IS NULL
               AND cs.deleted_at IS NULL
               AND ce.deleted_at IS NULL
             ORDER BY ce.date, ce.class_slot_id",
        )?;
        let rows = statement.query_map(rusqlite::params![term_id], exception_from_row)?;
        let mut exceptions = Vec::new();
        for row in rows {
            exceptions.push(row?);
        }
        Ok(exceptions)
    }

    /// Crea una excepción de clase. `moved` exige hora o aula nueva.
    #[allow(clippy::too_many_arguments)]
    pub fn create_class_exception(
        &self,
        slot_id: &str,
        date: &str,
        kind: &str,
        new_start_time: Option<&str>,
        new_end_time: Option<&str>,
        new_room: Option<&str>,
    ) -> Result<ClassException, StoreError> {
        self.get_class_slot(slot_id)?
            .ok_or(StoreError::ClassSlotNotFound)?;
        let kind = ExceptionKind::from_db(kind)
            .ok_or(StoreError::InvalidSchedule("Excepción inválida."))?;
        let date = normalize_date(date).map_err(StoreError::InvalidSchedule)?;

        let new_start_time = match new_start_time.filter(|value| !value.trim().is_empty()) {
            Some(value) => Some(normalize_time(value).map_err(StoreError::InvalidSchedule)?),
            None => None,
        };
        let new_end_time = match new_end_time.filter(|value| !value.trim().is_empty()) {
            Some(value) => Some(normalize_time(value).map_err(StoreError::InvalidSchedule)?),
            None => None,
        };
        if new_start_time.is_some() != new_end_time.is_some() {
            return Err(StoreError::InvalidSchedule(
                "La excepción necesita hora de inicio y de fin, o ninguna.",
            ));
        }
        if let (Some(start), Some(end)) = (&new_start_time, &new_end_time) {
            validate_time_range(start, end).map_err(StoreError::InvalidSchedule)?;
        }
        let new_room = normalize_optional_text(new_room.unwrap_or_default())
            .map_err(StoreError::InvalidSchedule)?;

        if kind == ExceptionKind::Moved && new_start_time.is_none() && new_room.is_none() {
            return Err(StoreError::InvalidSchedule(
                "Una clase movida necesita una hora o un aula nueva.",
            ));
        }

        let existing: i64 = self.connection().query_row(
            "SELECT count(*) FROM class_exception
             WHERE class_slot_id = ?1 AND date = ?2 AND kind = ?3 AND deleted_at IS NULL",
            rusqlite::params![slot_id, date, kind.as_str()],
            |row| row.get(0),
        )?;
        if existing > 0 {
            return Err(StoreError::Conflict(
                "Ya existe una excepción de ese tipo para esa fecha.",
            ));
        }

        let id = study_planner_core::new_id();
        self.connection().execute(
            "INSERT INTO class_exception
               (id, class_slot_id, date, kind, new_start_time, new_end_time, new_room)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                slot_id,
                date,
                kind.as_str(),
                new_start_time,
                new_end_time,
                new_room,
            ],
        )?;

        self.get_class_exception(&id)?
            .ok_or(StoreError::ExceptionNotFound)
    }

    /// Borra lógicamente una excepción.
    pub fn delete_class_exception(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE class_exception
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![id],
        )?;

        if changed == 0 {
            return Err(StoreError::ExceptionNotFound);
        }
        Ok(())
    }

    /// Lista los registros de asistencia de un término.
    pub fn list_attendance(&self, term_id: &str) -> Result<Vec<AttendanceRecord>, StoreError> {
        let mut statement = self.connection().prepare(
            "SELECT ar.id, ar.class_slot_id, ar.date, ar.status, ar.note,
                    ar.created_at, ar.updated_at
             FROM attendance_record ar
             JOIN class_slot cs ON cs.id = ar.class_slot_id
             JOIN subject s ON s.id = cs.subject_id
             WHERE s.term_id = ?1
               AND s.deleted_at IS NULL
               AND cs.deleted_at IS NULL
               AND ar.deleted_at IS NULL
             ORDER BY ar.date, ar.class_slot_id",
        )?;
        let rows = statement.query_map(rusqlite::params![term_id], attendance_from_row)?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// Registra o actualiza la asistencia de una franja en una fecha.
    pub fn set_attendance(
        &self,
        slot_id: &str,
        date: &str,
        status: &str,
        note: Option<&str>,
    ) -> Result<AttendanceRecord, StoreError> {
        self.get_class_slot(slot_id)?
            .ok_or(StoreError::ClassSlotNotFound)?;
        let status = AttendanceStatus::from_db(status).ok_or(StoreError::InvalidSchedule(
            "Estado de asistencia inválido.",
        ))?;
        let date = normalize_date(date).map_err(StoreError::InvalidSchedule)?;
        let note = normalize_optional_text(note.unwrap_or_default())
            .map_err(StoreError::InvalidSchedule)?;

        let existing = self
            .connection()
            .query_row(
                "SELECT id FROM attendance_record
                 WHERE class_slot_id = ?1 AND date = ?2 AND deleted_at IS NULL",
                rusqlite::params![slot_id, date],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        if let Some(id) = existing {
            self.connection().execute(
                "UPDATE attendance_record
                 SET status = ?1, note = ?2,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?3",
                rusqlite::params![status.as_str(), note, id],
            )?;
            return self
                .get_attendance(&id)?
                .ok_or(StoreError::AttendanceNotFound);
        }

        let id = study_planner_core::new_id();
        self.connection().execute(
            "INSERT INTO attendance_record (id, class_slot_id, date, status, note)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, slot_id, date, status.as_str(), note],
        )?;

        self.get_attendance(&id)?
            .ok_or(StoreError::AttendanceNotFound)
    }

    /// Borra lógicamente la asistencia de una franja en una fecha.
    pub fn clear_attendance(&self, slot_id: &str, date: &str) -> Result<(), StoreError> {
        let date = normalize_date(date).map_err(StoreError::InvalidSchedule)?;
        let changed = self.connection().execute(
            "UPDATE attendance_record
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE class_slot_id = ?1 AND date = ?2 AND deleted_at IS NULL",
            rusqlite::params![slot_id, date],
        )?;

        if changed == 0 {
            return Err(StoreError::AttendanceNotFound);
        }
        Ok(())
    }

    /// Compone la semana de un término con ocurrencias, feriados, solapamientos
    /// y el resumen de asistencia por materia.
    pub fn week_schedule(
        &self,
        term_id: &str,
        week_start: &str,
    ) -> Result<WeekSchedule, StoreError> {
        let week_start = week_start_of(week_start)
            .ok_or(StoreError::InvalidSchedule("Fecha de semana inválida."))?;
        let week_end = add_days(&week_start, 6)
            .ok_or(StoreError::InvalidSchedule("Fecha de semana inválida."))?;

        self.get_term(term_id)?.ok_or(StoreError::TermNotFound)?;
        let subjects = self.list_subjects(term_id)?;
        let slots = self.list_schedule_slots(term_id)?;
        let rotation = self.get_rotation(term_id)?;
        let holidays = self.list_holidays(term_id)?;
        let exceptions = self.list_class_exceptions(term_id)?;
        let attendance = self.list_attendance(term_id)?;

        let occurrences = week_occurrences(
            &week_start,
            &slots,
            rotation.as_ref(),
            &holidays,
            &exceptions,
            &attendance,
        );

        let week_holidays = holidays
            .iter()
            .filter(|holiday| holiday.date >= week_start && holiday.date <= week_end)
            .map(|holiday| WeekHoliday {
                date: holiday.date.clone(),
                name: holiday.name.clone(),
            })
            .collect();

        Ok(WeekSchedule {
            term_id: term_id.to_string(),
            week_start: week_start.clone(),
            week_end,
            rotation_label: rotation
                .as_ref()
                .and_then(|config| config.label_for(&week_start)),
            holidays: week_holidays,
            occurrences,
            overlaps: find_overlaps(&slots),
            attendance: attendance_summary(&subjects, &slots, &attendance),
        })
    }

    /// Primera clase no cancelada desde una fecha y hora dadas.
    pub fn next_class(
        &self,
        term_id: &str,
        from_date: &str,
        from_time: &str,
    ) -> Result<Option<ScheduleOccurrence>, StoreError> {
        let from_date = normalize_date(from_date).map_err(StoreError::InvalidSchedule)?;
        let from_time = normalize_time(from_time).map_err(StoreError::InvalidSchedule)?;
        let start =
            week_start_of(&from_date).ok_or(StoreError::InvalidSchedule("Fecha inválida."))?;

        let mut occurrences = Vec::new();
        for offset in 0..4 {
            let week = add_days(&start, offset * 7)
                .ok_or(StoreError::InvalidSchedule("Fecha inválida."))?;
            occurrences.extend(self.week_schedule(term_id, &week)?.occurrences);
        }
        occurrences.sort_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then(left.start_time.cmp(&right.start_time))
        });

        Ok(next_class(&occurrences, &from_date, &from_time).cloned())
    }

    fn get_class_slot(&self, id: &str) -> Result<Option<ClassSlot>, StoreError> {
        let slot = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {SLOT_COLUMNS}
                     FROM class_slot
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                rusqlite::params![id],
                class_slot_from_row,
            )
            .optional()?;
        Ok(slot)
    }

    fn get_holiday(&self, id: &str) -> Result<Option<Holiday>, StoreError> {
        let holiday = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {HOLIDAY_COLUMNS}
                     FROM holiday
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                rusqlite::params![id],
                holiday_from_row,
            )
            .optional()?;
        Ok(holiday)
    }

    fn get_class_exception(&self, id: &str) -> Result<Option<ClassException>, StoreError> {
        let exception = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {EXCEPTION_COLUMNS}
                     FROM class_exception
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                rusqlite::params![id],
                exception_from_row,
            )
            .optional()?;
        Ok(exception)
    }

    fn get_attendance(&self, id: &str) -> Result<Option<AttendanceRecord>, StoreError> {
        let record = self
            .connection()
            .query_row(
                &format!(
                    "SELECT {ATTENDANCE_COLUMNS}
                     FROM attendance_record
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                rusqlite::params![id],
                attendance_from_row,
            )
            .optional()?;
        Ok(record)
    }
}

struct ValidSlot {
    weekday: u8,
    start_time: String,
    end_time: String,
    recurrence: &'static str,
    rotation_key: Option<String>,
    valid_from: Option<String>,
    valid_to: Option<String>,
    room_override: Option<String>,
}

fn validate_slot_input(input: &ClassSlotInput) -> Result<ValidSlot, StoreError> {
    let weekday = validate_weekday(input.weekday).map_err(StoreError::InvalidSchedule)?;
    let start_time = normalize_time(&input.start_time).map_err(StoreError::InvalidSchedule)?;
    let end_time = normalize_time(&input.end_time).map_err(StoreError::InvalidSchedule)?;
    validate_time_range(&start_time, &end_time).map_err(StoreError::InvalidSchedule)?;
    let recurrence = Recurrence::from_db(&input.recurrence)
        .ok_or(StoreError::InvalidSchedule("Recurrencia inválida."))?;
    let rotation_key = normalize_optional_short(input.rotation_key.as_deref().unwrap_or_default())
        .map_err(StoreError::InvalidSchedule)?;

    let valid_from = match input.valid_from.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => {
            Some(normalize_date(value).map_err(StoreError::InvalidSchedule)?)
        }
        _ => None,
    };
    let valid_to = match input.valid_to.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => {
            Some(normalize_date(value).map_err(StoreError::InvalidSchedule)?)
        }
        _ => None,
    };
    if let (Some(from), Some(to)) = (&valid_from, &valid_to) {
        validate_date_range(from, to).map_err(StoreError::InvalidSchedule)?;
    }
    let room_override = normalize_optional_text(input.room_override.as_deref().unwrap_or_default())
        .map_err(StoreError::InvalidSchedule)?;

    Ok(ValidSlot {
        weekday,
        start_time,
        end_time,
        recurrence: recurrence.as_str(),
        rotation_key,
        valid_from,
        valid_to,
        room_override,
    })
}

fn normalize_labels(labels: &[String]) -> Result<Vec<String>, StoreError> {
    if labels.len() > 12 {
        return Err(StoreError::InvalidSchedule(
            "La rotación admite hasta 12 etiquetas.",
        ));
    }

    let mut normalized = Vec::new();
    for label in labels {
        let value = label.trim();
        if value.is_empty() {
            return Err(StoreError::InvalidSchedule(
                "Las etiquetas de rotación no pueden estar vacías.",
            ));
        }
        if value.chars().count() > 40 {
            return Err(StoreError::InvalidSchedule(
                "Cada etiqueta admite hasta 40 caracteres.",
            ));
        }
        normalized.push(value.to_string());
    }
    Ok(normalized)
}

fn class_slot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClassSlot> {
    let recurrence: String = row.get(5)?;
    Ok(ClassSlot {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        weekday: row.get(2)?,
        start_time: row.get(3)?,
        end_time: row.get(4)?,
        recurrence: Recurrence::from_db(&recurrence).unwrap_or(Recurrence::Weekly),
        rotation_key: row.get(6)?,
        valid_from: row.get(7)?,
        valid_to: row.get(8)?,
        room_override: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn holiday_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Holiday> {
    Ok(Holiday {
        id: row.get(0)?,
        term_id: row.get(1)?,
        date: row.get(2)?,
        name: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn exception_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClassException> {
    let kind: String = row.get(3)?;
    Ok(ClassException {
        id: row.get(0)?,
        class_slot_id: row.get(1)?,
        date: row.get(2)?,
        kind: ExceptionKind::from_db(&kind).unwrap_or(ExceptionKind::Cancelled),
        new_start_time: row.get(4)?,
        new_end_time: row.get(5)?,
        new_room: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn attendance_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AttendanceRecord> {
    let status: String = row.get(3)?;
    Ok(AttendanceRecord {
        id: row.get(0)?,
        class_slot_id: row.get(1)?,
        date: row.get(2)?,
        status: AttendanceStatus::from_db(&status).unwrap_or(AttendanceStatus::Present),
        note: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn rotation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RotationConfig> {
    let mode: String = row.get(2)?;
    let labels_json: String = row.get(5)?;
    let day_cycle_json: String = row.get(6)?;
    Ok(RotationConfig {
        id: row.get(0)?,
        term_id: row.get(1)?,
        mode: RotationMode::from_db(&mode).unwrap_or(RotationMode::WeeklyAb),
        anchor_date: row.get(3)?,
        cycle_length: row.get(4)?,
        labels: serde_json::from_str(&labels_json).unwrap_or_default(),
        day_cycle: serde_json::from_str(&day_cycle_json).unwrap_or_default(),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

// Las pruebas usan el tipo de ocurrencia para verificar excepciones.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SubjectInput, TermInput};
    use study_planner_core::{DEFAULT_SUBJECT_COLOR, OccurrenceKind, TermKind};

    fn migrated_store() -> Store {
        let mut store = Store::open_in_memory().expect("abrir memoria");
        store.migrate().expect("migrar");
        store
    }

    fn seed(store: &Store) -> (String, String, String) {
        let year = store
            .create_academic_year("2026", "2026-03-01", "2026-12-20")
            .unwrap();
        let term = store
            .create_term(
                &year.id,
                &TermInput {
                    name: "Primer semestre".to_string(),
                    kind: TermKind::Semester.as_str().to_string(),
                    start_date: "2026-03-02".to_string(),
                    end_date: "2026-07-10".to_string(),
                },
            )
            .unwrap();
        let subject = store
            .create_subject(
                &term.id,
                &SubjectInput {
                    name: "Álgebra".to_string(),
                    color: Some(DEFAULT_SUBJECT_COLOR.to_string()),
                    attendance_min_bp: Some(7_500),
                    ..SubjectInput::default()
                },
            )
            .unwrap();
        (year.id, term.id, subject.id)
    }

    fn weekly_slot(weekday: u8, start: &str, end: &str) -> ClassSlotInput {
        ClassSlotInput {
            weekday,
            start_time: start.to_string(),
            end_time: end.to_string(),
            recurrence: "weekly".to_string(),
            ..ClassSlotInput::default()
        }
    }

    #[test]
    fn creates_slots_and_builds_a_week() {
        let store = migrated_store();
        let (_, term_id, subject_id) = seed(&store);
        let slot = store
            .create_class_slot(&subject_id, &weekly_slot(1, "08:00", "10:00"))
            .unwrap();

        let week = store.week_schedule(&term_id, "2026-03-02").unwrap();
        assert_eq!(week.week_start, "2026-03-02");
        assert_eq!(week.week_end, "2026-03-08");
        assert_eq!(week.occurrences.len(), 1);
        assert_eq!(week.occurrences[0].subject_name, "Álgebra");
        assert_eq!(week.occurrences[0].slot_id, slot.id);

        let holiday = store
            .create_holiday(Some(&term_id), "2026-03-02", "Feriado")
            .unwrap();
        let week = store.week_schedule(&term_id, "2026-03-02").unwrap();
        assert!(week.occurrences.is_empty());
        assert_eq!(week.holidays.len(), 1);
        store.delete_holiday(&holiday.id).unwrap();
    }

    #[test]
    fn rotation_filters_and_labels_the_week() {
        let store = migrated_store();
        let (_, term_id, subject_id) = seed(&store);
        store
            .save_rotation(
                &term_id,
                &RotationInput {
                    mode: "weekly_ab".to_string(),
                    anchor_date: "2026-03-02".to_string(),
                    cycle_length: 2,
                    labels: vec!["A".to_string(), "B".to_string()],
                    day_cycle: Vec::new(),
                },
            )
            .unwrap();

        let mut input = weekly_slot(1, "08:00", "10:00");
        input.rotation_key = Some("B".to_string());
        store.create_class_slot(&subject_id, &input).unwrap();

        let week_a = store.week_schedule(&term_id, "2026-03-02").unwrap();
        assert_eq!(week_a.rotation_label.as_deref(), Some("A"));
        assert!(week_a.occurrences.is_empty());

        let week_b = store.week_schedule(&term_id, "2026-03-09").unwrap();
        assert_eq!(week_b.rotation_label.as_deref(), Some("B"));
        assert_eq!(week_b.occurrences.len(), 1);
    }

    #[test]
    fn attendance_is_upserted_and_summarized() {
        let store = migrated_store();
        let (_, term_id, subject_id) = seed(&store);
        let slot = store
            .create_class_slot(&subject_id, &weekly_slot(1, "08:00", "10:00"))
            .unwrap();

        store
            .set_attendance(&slot.id, "2026-03-02", "present", None)
            .unwrap();
        store
            .set_attendance(&slot.id, "2026-03-09", "absent", Some("Aviso"))
            .unwrap();
        store
            .set_attendance(&slot.id, "2026-03-02", "late", None)
            .unwrap();

        let records = store.list_attendance(&term_id).unwrap();
        assert_eq!(records.len(), 2);

        let week = store.week_schedule(&term_id, "2026-03-02").unwrap();
        assert_eq!(week.attendance.len(), 1);
        assert_eq!(week.attendance[0].counted, 2);
        assert_eq!(week.attendance[0].attended, 1);
        assert_eq!(week.attendance[0].percent_bp, Some(5_000));
        assert!(week.attendance[0].below_min);

        store.clear_attendance(&slot.id, "2026-03-02").unwrap();
        assert_eq!(store.list_attendance(&term_id).unwrap().len(), 1);
    }

    #[test]
    fn exceptions_change_the_week_and_reject_duplicates() {
        let store = migrated_store();
        let (_, term_id, subject_id) = seed(&store);
        let slot = store
            .create_class_slot(&subject_id, &weekly_slot(1, "08:00", "10:00"))
            .unwrap();

        let exception = store
            .create_class_exception(
                &slot.id,
                "2026-03-02",
                "moved",
                Some("10:00"),
                Some("12:00"),
                Some("Aula 4"),
            )
            .unwrap();
        let week = store.week_schedule(&term_id, "2026-03-02").unwrap();
        assert_eq!(week.occurrences.len(), 1);
        assert_eq!(week.occurrences[0].kind, OccurrenceKind::Moved);
        assert_eq!(week.occurrences[0].end_time, "12:00");

        let duplicate = store.create_class_exception(
            &slot.id,
            "2026-03-02",
            "moved",
            Some("11:00"),
            Some("12:00"),
            None,
        );
        assert!(matches!(duplicate, Err(StoreError::Conflict(_))));

        store.delete_class_exception(&exception.id).unwrap();
        assert_eq!(
            store
                .week_schedule(&term_id, "2026-03-02")
                .unwrap()
                .occurrences
                .len(),
            1
        );
    }

    #[test]
    fn next_class_and_overlaps_are_available() {
        let store = migrated_store();
        let (_, term_id, subject_id) = seed(&store);
        store
            .create_class_slot(&subject_id, &weekly_slot(1, "08:00", "10:00"))
            .unwrap();
        let other = store
            .create_subject(
                &term_id,
                &SubjectInput {
                    name: "Física".to_string(),
                    ..SubjectInput::default()
                },
            )
            .unwrap();
        store
            .create_class_slot(&other.id, &weekly_slot(1, "09:00", "11:00"))
            .unwrap();

        let next = store
            .next_class(&term_id, "2026-03-02", "07:00")
            .unwrap()
            .expect("hay clase");
        assert_eq!(next.date, "2026-03-02");
        assert_eq!(next.start_time, "08:00");

        let week = store.week_schedule(&term_id, "2026-03-02").unwrap();
        assert_eq!(week.overlaps.len(), 1);
    }

    #[test]
    fn validates_slots_and_rotation_inputs() {
        let store = migrated_store();
        let (_, term_id, subject_id) = seed(&store);

        let error = store
            .create_class_slot(&subject_id, &weekly_slot(8, "08:00", "09:00"))
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidSchedule(_)));

        let error = store
            .create_class_slot(&subject_id, &weekly_slot(1, "10:00", "09:00"))
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidSchedule(_)));

        let error = store
            .save_rotation(
                &term_id,
                &RotationInput {
                    mode: "day_cycle".to_string(),
                    anchor_date: "2026-03-02".to_string(),
                    cycle_length: 2,
                    labels: Vec::new(),
                    day_cycle: vec!["1".to_string(), "2".to_string(), "3".to_string()],
                },
            )
            .unwrap_err();
        assert!(matches!(error, StoreError::InvalidSchedule(_)));
    }
}
