//! Modelos académicos y reglas puras de horario.
//!
//! Este módulo no conoce SQLite, Tauri ni el reloj del sistema: todas las
//! fechas se reciben como `YYYY-MM-DD` y las horas como `HH:MM`, de modo que
//! los cálculos sean deterministas y testeables.

use serde::Serialize;

/// Longitud máxima de un nombre visible (año, término, materia, feriado).
pub const NAME_MAX_CHARS: usize = 120;

/// Longitud máxima de códigos cortos (código de materia, aula, rotación).
pub const SHORT_TEXT_MAX_CHARS: usize = 40;

/// Color por defecto de una materia.
pub const DEFAULT_SUBJECT_COLOR: &str = "#6750A4";

// ---------------------------------------------------------------------------
// Modelos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicYear {
    pub id: String,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TermKind {
    Semester,
    Trimester,
    Quarter,
    Bimonth,
    Annual,
    Other,
}

impl TermKind {
    pub const ALL: [Self; 6] = [
        Self::Semester,
        Self::Trimester,
        Self::Quarter,
        Self::Bimonth,
        Self::Annual,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Semester => "semester",
            Self::Trimester => "trimester",
            Self::Quarter => "quarter",
            Self::Bimonth => "bimonth",
            Self::Annual => "annual",
            Self::Other => "other",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "semester" => Some(Self::Semester),
            "trimester" => Some(Self::Trimester),
            "quarter" => Some(Self::Quarter),
            "bimonth" => Some(Self::Bimonth),
            "annual" => Some(Self::Annual),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub id: String,
    pub year_id: String,
    pub name: String,
    pub kind: TermKind,
    pub start_date: String,
    pub end_date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub id: String,
    pub term_id: String,
    pub name: String,
    pub code: Option<String>,
    pub color: String,
    pub teacher: Option<String>,
    pub room: Option<String>,
    pub credits: Option<i64>,
    pub goal_score_cp: Option<i64>,
    pub attendance_min_bp: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    Weekly,
    Biweekly,
}

impl Recurrence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Biweekly => "biweekly",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "weekly" => Some(Self::Weekly),
            "biweekly" => Some(Self::Biweekly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassSlot {
    pub id: String,
    pub subject_id: String,
    pub weekday: u8,
    pub start_time: String,
    pub end_time: String,
    pub recurrence: Recurrence,
    pub rotation_key: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub room_override: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Holiday {
    pub id: String,
    pub term_id: Option<String>,
    pub date: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionKind {
    Cancelled,
    Moved,
    Extra,
}

impl ExceptionKind {
    pub const ALL: [Self; 3] = [Self::Cancelled, Self::Moved, Self::Extra];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Moved => "moved",
            Self::Extra => "extra",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "cancelled" => Some(Self::Cancelled),
            "moved" => Some(Self::Moved),
            "extra" => Some(Self::Extra),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassException {
    pub id: String,
    pub class_slot_id: String,
    pub date: String,
    pub kind: ExceptionKind,
    pub new_start_time: Option<String>,
    pub new_end_time: Option<String>,
    pub new_room: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceStatus {
    Present,
    Absent,
    Late,
    Justified,
}

impl AttendanceStatus {
    pub const ALL: [Self; 4] = [Self::Present, Self::Absent, Self::Late, Self::Justified];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Late => "late",
            Self::Justified => "justified",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "present" => Some(Self::Present),
            "absent" => Some(Self::Absent),
            "late" => Some(Self::Late),
            "justified" => Some(Self::Justified),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceRecord {
    pub id: String,
    pub class_slot_id: String,
    pub date: String,
    pub status: AttendanceStatus,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RotationMode {
    WeeklyAb,
    Parity,
    DayCycle,
}

impl RotationMode {
    pub const ALL: [Self; 3] = [Self::WeeklyAb, Self::Parity, Self::DayCycle];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::WeeklyAb => "weekly_ab",
            Self::Parity => "parity",
            Self::DayCycle => "day_cycle",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "weekly_ab" => Some(Self::WeeklyAb),
            "parity" => Some(Self::Parity),
            "day_cycle" => Some(Self::DayCycle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationConfig {
    pub id: String,
    pub term_id: String,
    pub mode: RotationMode,
    pub anchor_date: String,
    pub cycle_length: u32,
    pub labels: Vec<String>,
    pub day_cycle: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Validaciones puras
// ---------------------------------------------------------------------------

/// Normaliza un nombre visible: recorta y exige contenido.
pub fn normalize_name(raw: &str) -> Result<String, &'static str> {
    let name = raw.trim();

    if name.is_empty() {
        return Err("El nombre no puede estar vacío.");
    }

    if name.chars().count() > NAME_MAX_CHARS {
        return Err("El nombre supera los 120 caracteres.");
    }

    Ok(name.to_string())
}

/// Normaliza un texto corto opcional. Vacío se convierte en `None`.
pub fn normalize_optional_short(raw: &str) -> Result<Option<String>, &'static str> {
    let value = raw.trim();

    if value.is_empty() {
        return Ok(None);
    }

    if value.chars().count() > SHORT_TEXT_MAX_CHARS {
        return Err("El texto supera los 40 caracteres.");
    }

    Ok(Some(value.to_string()))
}

/// Normaliza texto libre opcional (docente, aula, nota).
pub fn normalize_optional_text(raw: &str) -> Result<Option<String>, &'static str> {
    let value = raw.trim();

    if value.is_empty() {
        return Ok(None);
    }

    if value.chars().count() > NAME_MAX_CHARS {
        return Err("El texto supera los 120 caracteres.");
    }

    Ok(Some(value.to_string()))
}

/// Valida un color `#RRGGBB` (o `#RGB`) y lo devuelve en mayúsculas.
pub fn normalize_color(raw: &str) -> Result<String, &'static str> {
    let value = raw.trim();

    if value.is_empty() {
        return Ok(DEFAULT_SUBJECT_COLOR.to_string());
    }

    let hex = value.strip_prefix('#').unwrap_or(value);
    let valid_length = hex.len() == 3 || hex.len() == 6;
    if !valid_length || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("El color debe tener el formato #RRGGBB.");
    }

    if hex.len() == 3 {
        let expanded: String = hex.chars().flat_map(|c| [c, c]).collect();
        return Ok(format!("#{}", expanded.to_uppercase()));
    }

    Ok(format!("#{}", hex.to_uppercase()))
}

/// Valida una fecha civil `YYYY-MM-DD` y la devuelve normalizada.
pub fn normalize_date(raw: &str) -> Result<String, &'static str> {
    let value = raw.trim();
    if parse_date(value).is_none() {
        return Err("La fecha debe tener el formato AAAA-MM-DD y ser válida.");
    }
    Ok(value.to_string())
}

/// Valida una hora local `HH:MM`.
pub fn normalize_time(raw: &str) -> Result<String, &'static str> {
    let value = raw.trim();
    let Some((hours, minutes)) = value.split_once(':') else {
        return Err("La hora debe tener el formato HH:MM.");
    };
    if hours.len() != 2 || minutes.len() != 2 {
        return Err("La hora debe tener el formato HH:MM.");
    }
    let Ok(hours) = hours.parse::<u8>() else {
        return Err("La hora debe tener el formato HH:MM.");
    };
    let Ok(minutes) = minutes.parse::<u8>() else {
        return Err("La hora debe tener el formato HH:MM.");
    };
    if hours > 23 || minutes > 59 {
        return Err("La hora debe estar entre 00:00 y 23:59.");
    }
    Ok(format!("{hours:02}:{minutes:02}"))
}

/// Valida que el rango de fechas sea coherente.
pub fn validate_date_range(start: &str, end: &str) -> Result<(), &'static str> {
    let start = normalize_date(start)?;
    let end = normalize_date(end)?;
    if start > end {
        return Err("La fecha de inicio no puede ser posterior a la de fin.");
    }
    Ok(())
}

/// Valida que una franja horaria tenga inicio anterior al fin.
pub fn validate_time_range(start: &str, end: &str) -> Result<(), &'static str> {
    let start = normalize_time(start)?;
    let end = normalize_time(end)?;
    if start >= end {
        return Err("La hora de inicio debe ser anterior a la de fin.");
    }
    Ok(())
}

/// Valida un día de la semana ISO (1 = lunes, 7 = domingo).
pub fn validate_weekday(weekday: u8) -> Result<u8, &'static str> {
    if (1..=7).contains(&weekday) {
        Ok(weekday)
    } else {
        Err("El día de la semana debe estar entre 1 (lunes) y 7 (domingo).")
    }
}

/// Valida un porcentaje en basis points (0 a 10000).
pub fn validate_basis_points(value: i64) -> Result<i64, &'static str> {
    if (0..=10_000).contains(&value) {
        Ok(value)
    } else {
        Err("El porcentaje debe estar entre 0 y 10000 basis points.")
    }
}

/// Convierte `HH:MM` a minutos desde la medianoche.
pub fn time_to_minutes(value: &str) -> Option<u16> {
    let (hours, minutes) = value.split_once(':')?;
    let hours: u16 = hours.parse().ok()?;
    let minutes: u16 = minutes.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

/// Convierte minutos desde la medianoche a `HH:MM`.
pub fn minutes_to_time(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

// ---------------------------------------------------------------------------
// Fechas civiles
// ---------------------------------------------------------------------------

/// Parsea `YYYY-MM-DD` y valida el calendario gregoriano.
pub fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    if value.len() != 10
        || !value.is_ascii()
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
    {
        return None;
    }
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }

    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };

    if day == 0 || day > days_in_month {
        return None;
    }

    Some((year, month, day))
}

/// Días desde 1970-01-01 (algoritmo civil de Howard Hinnant).
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = i64::from(year_of_era) * 365 + i64::from(year_of_era) / 4
        - i64::from(year_of_era) / 100
        + day_of_year;
    i64::from(era) * 146_097 + day_of_era - 719_468
}

/// Fecha civil a partir de días desde 1970-01-01.
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year as i32, month as u32, day as u32)
}

/// Día ISO de la semana: 1 = lunes … 7 = domingo.
pub fn weekday_of(date: &str) -> Option<u8> {
    let (year, month, day) = parse_date(date)?;
    let days = days_from_civil(year, month, day);
    Some((((days + 3).rem_euclid(7)) + 1) as u8)
}

/// Suma (o resta) días a una fecha civil.
pub fn add_days(date: &str, delta: i64) -> Option<String> {
    let (year, month, day) = parse_date(date)?;
    let days = days_from_civil(year, month, day) + delta;
    let (year, month, day) = civil_from_days(days);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// Diferencia en días entre dos fechas (`b - a`).
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    let (from_year, from_month, from_day) = parse_date(from)?;
    let (to_year, to_month, to_day) = parse_date(to)?;
    Some(
        days_from_civil(to_year, to_month, to_day)
            - days_from_civil(from_year, from_month, from_day),
    )
}

/// Lunes de la semana que contiene la fecha.
pub fn week_start_of(date: &str) -> Option<String> {
    let weekday = i64::from(weekday_of(date)?);
    add_days(date, -(weekday - 1))
}

/// Indica si la fecha cae entre dos límites inclusivos, con `None` como infinito.
pub fn date_within(date: &str, from: Option<&str>, to: Option<&str>) -> bool {
    if let Some(from) = from
        && date < from
    {
        return false;
    }
    if let Some(to) = to
        && date > to
    {
        return false;
    }
    true
}

/// Cantidad de días hábiles (lunes a viernes) entre dos fechas, inclusiva.
pub fn weekdays_between(from: &str, to: &str) -> Option<i64> {
    let total = days_between(from, to)?;
    if total < 0 {
        return Some(0);
    }

    let full_weeks = total / 7;
    let mut count = full_weeks * 5;
    let remainder = total % 7;
    let start_weekday = i64::from(weekday_of(from)?);
    for offset in 0..=remainder {
        let weekday = (start_weekday - 1 + offset) % 7 + 1;
        if weekday <= 5 {
            count += 1;
        }
    }
    Some(count)
}

// ---------------------------------------------------------------------------
// Rotaciones
// ---------------------------------------------------------------------------

impl RotationConfig {
    /// Índice de rotación para una fecha, según el modo configurado.
    pub fn index_for(&self, date: &str) -> Option<u32> {
        let anchor_week = week_start_of(&self.anchor_date)?;
        let target_week = week_start_of(date)?;
        let cycle = self.cycle_length.max(1);

        match self.mode {
            RotationMode::WeeklyAb | RotationMode::Parity => {
                let weeks = days_between(&anchor_week, &target_week)? / 7;
                Some((weeks.rem_euclid(i64::from(cycle))) as u32)
            }
            RotationMode::DayCycle => {
                let weekdays = weekdays_between(&self.anchor_date, date)?;
                let index = if weekdays == 0 {
                    self.cycle_length.max(1) - 1
                } else {
                    (weekdays - 1) as u32
                };
                Some(index % cycle)
            }
        }
    }

    /// Etiqueta visible de la rotación para una fecha.
    pub fn label_for(&self, date: &str) -> Option<String> {
        let index = self.index_for(date)? as usize;
        let fallback: [&str; 2] = match self.mode {
            RotationMode::WeeklyAb | RotationMode::DayCycle => ["A", "B"],
            RotationMode::Parity => ["Par", "Impar"],
        };
        let labels = match self.mode {
            RotationMode::DayCycle if !self.day_cycle.is_empty() => &self.day_cycle,
            _ if !self.labels.is_empty() => &self.labels,
            _ => return Some(fallback[index % fallback.len()].to_string()),
        };
        labels.get(index % labels.len()).cloned()
    }
}

// ---------------------------------------------------------------------------
// Ocurrencias y cálculos
// ---------------------------------------------------------------------------

/// Franja con los datos de materia ya resueltos para la vista.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSlot {
    pub slot: ClassSlot,
    pub subject_name: String,
    pub subject_color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OccurrenceKind {
    Scheduled,
    Cancelled,
    Moved,
    Extra,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleOccurrence {
    pub date: String,
    pub weekday: u8,
    pub slot_id: String,
    pub subject_id: String,
    pub subject_name: String,
    pub subject_color: String,
    pub start_time: String,
    pub end_time: String,
    pub room: Option<String>,
    pub kind: OccurrenceKind,
    pub attendance_status: Option<AttendanceStatus>,
}

/// Indica si una franja se dicta normalmente en la fecha dada.
pub fn slot_occurs_on(slot: &ClassSlot, date: &str, rotation: Option<&RotationConfig>) -> bool {
    if weekday_of(date) != Some(slot.weekday) {
        return false;
    }

    if !date_within(date, slot.valid_from.as_deref(), slot.valid_to.as_deref()) {
        return false;
    }

    if slot.recurrence == Recurrence::Biweekly {
        let reference = rotation
            .map(|config| config.anchor_date.as_str())
            .or(slot.valid_from.as_deref())
            .unwrap_or(date);
        let reference_week = match week_start_of(reference) {
            Some(week) => week,
            None => return false,
        };
        let target_week = match week_start_of(date) {
            Some(week) => week,
            None => return false,
        };
        let weeks = match days_between(&reference_week, &target_week) {
            Some(weeks) => weeks / 7,
            None => return false,
        };
        if weeks.rem_euclid(2) != 0 {
            return false;
        }
    }

    if let (Some(key), Some(config)) = (slot.rotation_key.as_deref(), rotation)
        && let Some(label) = config.label_for(date)
        && !key.eq_ignore_ascii_case(&label)
    {
        return false;
    }

    true
}

/// Construye las ocurrencias de una semana (lunes a domingo) con feriados,
/// excepciones y asistencia aplicados.
pub fn week_occurrences(
    week_start: &str,
    slots: &[ScheduleSlot],
    rotation: Option<&RotationConfig>,
    holidays: &[Holiday],
    exceptions: &[ClassException],
    attendance: &[AttendanceRecord],
) -> Vec<ScheduleOccurrence> {
    let Some(monday) = week_start_of(week_start) else {
        return Vec::new();
    };

    let mut occurrences = Vec::new();
    let days: Vec<String> = (0..7)
        .filter_map(|offset| add_days(&monday, offset))
        .collect();

    for slot in slots {
        for date in &days {
            if !slot_occurs_on(&slot.slot, date, rotation) {
                continue;
            }
            if holidays.iter().any(|holiday| &holiday.date == date) {
                continue;
            }

            occurrences.push(occurrence_from_slot(slot, date, OccurrenceKind::Scheduled));
        }

        for exception in exceptions
            .iter()
            .filter(|exception| exception.class_slot_id == slot.slot.id)
            .filter(|exception| days.contains(&exception.date))
        {
            match exception.kind {
                ExceptionKind::Cancelled => {
                    if let Some(existing) = occurrences.iter_mut().find(|occurrence| {
                        occurrence.slot_id == slot.slot.id && occurrence.date == exception.date
                    }) {
                        existing.kind = OccurrenceKind::Cancelled;
                    } else if slot_occurs_on(&slot.slot, &exception.date, rotation) {
                        let mut occurrence =
                            occurrence_from_slot(slot, &exception.date, OccurrenceKind::Cancelled);
                        occurrence.kind = OccurrenceKind::Cancelled;
                        occurrences.push(occurrence);
                    }
                }
                ExceptionKind::Moved => {
                    let mut occurrence =
                        occurrence_from_slot(slot, &exception.date, OccurrenceKind::Moved);
                    if let Some(start) = exception.new_start_time.as_deref() {
                        occurrence.start_time = start.to_string();
                    }
                    if let Some(end) = exception.new_end_time.as_deref() {
                        occurrence.end_time = end.to_string();
                    }
                    if let Some(room) = exception.new_room.as_deref() {
                        occurrence.room = Some(room.to_string());
                    }
                    occurrences.retain(|existing| {
                        !(existing.slot_id == slot.slot.id
                            && existing.date == exception.date
                            && existing.kind == OccurrenceKind::Scheduled)
                    });
                    occurrences.push(occurrence);
                }
                ExceptionKind::Extra => {
                    let mut occurrence =
                        occurrence_from_slot(slot, &exception.date, OccurrenceKind::Extra);
                    if let Some(start) = exception.new_start_time.as_deref() {
                        occurrence.start_time = start.to_string();
                    }
                    if let Some(end) = exception.new_end_time.as_deref() {
                        occurrence.end_time = end.to_string();
                    }
                    if let Some(room) = exception.new_room.as_deref() {
                        occurrence.room = Some(room.to_string());
                    }
                    occurrences.push(occurrence);
                }
            }
        }
    }

    for occurrence in &mut occurrences {
        occurrence.attendance_status = attendance
            .iter()
            .find(|record| {
                record.class_slot_id == occurrence.slot_id && record.date == occurrence.date
            })
            .map(|record| record.status);
    }

    occurrences.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then(left.start_time.cmp(&right.start_time))
            .then(left.subject_name.cmp(&right.subject_name))
    });

    occurrences
}

fn occurrence_from_slot(
    slot: &ScheduleSlot,
    date: &str,
    kind: OccurrenceKind,
) -> ScheduleOccurrence {
    ScheduleOccurrence {
        date: date.to_string(),
        weekday: slot.slot.weekday,
        slot_id: slot.slot.id.clone(),
        subject_id: slot.slot.subject_id.clone(),
        subject_name: slot.subject_name.clone(),
        subject_color: slot.subject_color.clone(),
        start_time: slot.slot.start_time.clone(),
        end_time: slot.slot.end_time.clone(),
        room: slot.slot.room_override.clone(),
        kind,
        attendance_status: None,
    }
}

/// Primera clase no cancelada a partir de una fecha y hora locales.
pub fn next_class<'a>(
    occurrences: &'a [ScheduleOccurrence],
    from_date: &str,
    from_time: &str,
) -> Option<&'a ScheduleOccurrence> {
    occurrences.iter().find(|occurrence| {
        occurrence.kind != OccurrenceKind::Cancelled
            && (occurrence.date.as_str() > from_date
                || (occurrence.date == from_date && occurrence.start_time.as_str() >= from_time))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotOverlap {
    pub weekday: u8,
    pub first_slot_id: String,
    pub second_slot_id: String,
    pub first_subject_name: String,
    pub second_subject_name: String,
    pub start_time: String,
    pub end_time: String,
}

/// Detecta solapamientos de horario dentro de un término.
pub fn find_overlaps(slots: &[ScheduleSlot]) -> Vec<SlotOverlap> {
    let mut overlaps = Vec::new();

    for (index, first) in slots.iter().enumerate() {
        for second in slots.iter().skip(index + 1) {
            if first.slot.weekday != second.slot.weekday {
                continue;
            }

            if let (Some(first_key), Some(second_key)) = (
                first.slot.rotation_key.as_deref(),
                second.slot.rotation_key.as_deref(),
            ) && !first_key.eq_ignore_ascii_case(second_key)
            {
                continue;
            }

            if !ranges_overlap(&first.slot, &second.slot) {
                continue;
            }

            let Some(first_start) = time_to_minutes(&first.slot.start_time) else {
                continue;
            };
            let Some(first_end) = time_to_minutes(&first.slot.end_time) else {
                continue;
            };
            let Some(second_start) = time_to_minutes(&second.slot.start_time) else {
                continue;
            };
            let Some(second_end) = time_to_minutes(&second.slot.end_time) else {
                continue;
            };

            let start = first_start.max(second_start);
            let end = first_end.min(second_end);

            overlaps.push(SlotOverlap {
                weekday: first.slot.weekday,
                first_slot_id: first.slot.id.clone(),
                second_slot_id: second.slot.id.clone(),
                first_subject_name: first.subject_name.clone(),
                second_subject_name: second.subject_name.clone(),
                start_time: minutes_to_time(start),
                end_time: minutes_to_time(end),
            });
        }
    }

    overlaps
}

fn ranges_overlap(first: &ClassSlot, second: &ClassSlot) -> bool {
    let first_start = first.valid_from.as_deref().unwrap_or("0000-01-01");
    let first_end = first.valid_to.as_deref().unwrap_or("9999-12-31");
    let second_start = second.valid_from.as_deref().unwrap_or("0000-01-01");
    let second_end = second.valid_to.as_deref().unwrap_or("9999-12-31");

    first_start <= second_end && second_start <= first_end
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceSummary {
    pub subject_id: String,
    pub subject_name: String,
    pub subject_color: String,
    pub present: usize,
    pub absent: usize,
    pub late: usize,
    pub justified: usize,
    pub counted: usize,
    pub attended: usize,
    pub percent_bp: Option<i64>,
    pub min_bp: Option<i64>,
    pub below_min: bool,
}

/// Resume la asistencia por materia. Una falta justificada no penaliza.
pub fn attendance_summary(
    subjects: &[Subject],
    slots: &[ScheduleSlot],
    records: &[AttendanceRecord],
) -> Vec<AttendanceSummary> {
    let mut summaries = Vec::new();

    for subject in subjects {
        let subject_slots: Vec<&str> = slots
            .iter()
            .filter(|slot| slot.slot.subject_id == subject.id)
            .map(|slot| slot.slot.id.as_str())
            .collect();

        let mut present = 0;
        let mut absent = 0;
        let mut late = 0;
        let mut justified = 0;

        for record in records
            .iter()
            .filter(|record| subject_slots.contains(&record.class_slot_id.as_str()))
        {
            match record.status {
                AttendanceStatus::Present => present += 1,
                AttendanceStatus::Absent => absent += 1,
                AttendanceStatus::Late => late += 1,
                AttendanceStatus::Justified => justified += 1,
            }
        }

        let counted = present + absent + late + justified;
        let attended = present + late + justified;
        let percent_bp = if counted == 0 {
            None
        } else {
            Some(((attended as i64) * 10_000) / (counted as i64))
        };
        let below_min = match (percent_bp, subject.attendance_min_bp) {
            (Some(percent), Some(min)) => percent < min,
            _ => false,
        };

        summaries.push(AttendanceSummary {
            subject_id: subject.id.clone(),
            subject_name: subject.name.clone(),
            subject_color: subject.color.clone(),
            present,
            absent,
            late,
            justified,
            counted,
            attended,
            percent_bp,
            min_bp: subject.attendance_min_bp,
            below_min,
        });
    }

    summaries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(id: &str, subject_id: &str, weekday: u8, start: &str, end: &str) -> ScheduleSlot {
        ScheduleSlot {
            slot: ClassSlot {
                id: id.to_string(),
                subject_id: subject_id.to_string(),
                weekday,
                start_time: start.to_string(),
                end_time: end.to_string(),
                recurrence: Recurrence::Weekly,
                rotation_key: None,
                valid_from: None,
                valid_to: None,
                room_override: None,
                created_at: String::new(),
                updated_at: String::new(),
            },
            subject_name: format!("Materia {subject_id}"),
            subject_color: DEFAULT_SUBJECT_COLOR.to_string(),
        }
    }

    fn rotation(mode: RotationMode, anchor: &str, labels: &[&str]) -> RotationConfig {
        RotationConfig {
            id: "rotation".to_string(),
            term_id: "term".to_string(),
            mode,
            anchor_date: anchor.to_string(),
            cycle_length: 2,
            labels: labels.iter().map(|label| (*label).to_string()).collect(),
            day_cycle: Vec::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn date_math_matches_known_weekdays() {
        assert_eq!(weekday_of("2026-11-08"), Some(7));
        assert_eq!(weekday_of("2026-11-09"), Some(1));
        assert_eq!(week_start_of("2026-11-08").as_deref(), Some("2026-11-02"));
        assert_eq!(add_days("2026-02-28", 1).as_deref(), Some("2026-03-01"));
        assert_eq!(add_days("2024-02-28", 1).as_deref(), Some("2024-02-29"));
        assert_eq!(days_between("2026-01-01", "2026-01-31"), Some(30));
        assert_eq!(weekdays_between("2026-11-02", "2026-11-08"), Some(5));
    }

    #[test]
    fn rejects_invalid_dates_and_times() {
        assert!(normalize_date("2026-02-30").is_err());
        assert!(normalize_date("2026-2-3").is_err());
        assert!(normalize_time("24:00").is_err());
        assert!(normalize_time("7:30").is_err());
        assert!(validate_time_range("10:00", "10:00").is_err());
        assert!(validate_date_range("2026-05-01", "2026-04-30").is_err());
    }

    #[test]
    fn weekly_rotation_changes_each_week() {
        let config = rotation(RotationMode::WeeklyAb, "2026-03-02", &["A", "B"]);
        assert_eq!(config.label_for("2026-03-02").as_deref(), Some("A"));
        assert_eq!(config.label_for("2026-03-09").as_deref(), Some("B"));
        assert_eq!(config.label_for("2026-03-16").as_deref(), Some("A"));
    }

    #[test]
    fn day_cycle_advances_on_weekdays() {
        let mut config = rotation(RotationMode::DayCycle, "2026-03-02", &[]);
        config.cycle_length = 3;
        config.day_cycle = vec!["1".into(), "2".into(), "3".into()];
        assert_eq!(config.label_for("2026-03-02").as_deref(), Some("1"));
        assert_eq!(config.label_for("2026-03-03").as_deref(), Some("2"));
        assert_eq!(config.label_for("2026-03-06").as_deref(), Some("2"));
        assert_eq!(config.label_for("2026-03-09").as_deref(), Some("3"));
    }

    #[test]
    fn rotation_key_filters_slots() {
        let config = rotation(RotationMode::WeeklyAb, "2026-03-02", &["A", "B"]);
        let mut ab_slot = slot("slot", "subject", 1, "08:00", "10:00");
        ab_slot.slot.rotation_key = Some("B".to_string());

        assert!(!slot_occurs_on(&ab_slot.slot, "2026-03-02", Some(&config)));
        assert!(slot_occurs_on(&ab_slot.slot, "2026-03-09", Some(&config)));
    }

    #[test]
    fn holidays_and_exceptions_transform_the_week() {
        let subject = "subject";
        let slot = slot("slot", subject, 1, "08:00", "10:00");
        let week = "2026-03-02";

        let holiday = Holiday {
            id: "holiday".to_string(),
            term_id: None,
            date: "2026-03-02".to_string(),
            name: "Feriado".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert!(
            week_occurrences(
                week,
                std::slice::from_ref(&slot),
                None,
                &[holiday],
                &[],
                &[]
            )
            .is_empty()
        );

        let moved = ClassException {
            id: "exception".to_string(),
            class_slot_id: "slot".to_string(),
            date: "2026-03-02".to_string(),
            kind: ExceptionKind::Moved,
            new_start_time: Some("10:00".to_string()),
            new_end_time: Some("12:00".to_string()),
            new_room: Some("Aula 2".to_string()),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let occurrences =
            week_occurrences(week, std::slice::from_ref(&slot), None, &[], &[moved], &[]);
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].kind, OccurrenceKind::Moved);
        assert_eq!(occurrences[0].start_time, "10:00");
        assert_eq!(occurrences[0].room.as_deref(), Some("Aula 2"));

        let extra = ClassException {
            id: "extra".to_string(),
            class_slot_id: "slot".to_string(),
            date: "2026-03-04".to_string(),
            kind: ExceptionKind::Extra,
            new_start_time: None,
            new_end_time: None,
            new_room: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let occurrences = week_occurrences(week, &[slot], None, &[], &[extra], &[]);
        assert_eq!(occurrences.len(), 2);
        assert!(
            occurrences
                .iter()
                .any(|occurrence| occurrence.kind == OccurrenceKind::Extra)
        );
    }

    #[test]
    fn next_class_skips_cancelled_occurrences() {
        let occurrences = week_occurrences(
            "2026-03-02",
            &[slot("a", "s1", 1, "08:00", "10:00")],
            None,
            &[],
            &[],
            &[],
        );
        let next = next_class(&occurrences, "2026-03-02", "07:00").expect("hay clase");
        assert_eq!(next.date, "2026-03-02");
        assert!(next_class(&occurrences, "2026-03-04", "07:00").is_none());
    }

    #[test]
    fn overlapping_slots_are_reported() {
        let slots = vec![
            slot("a", "s1", 1, "08:00", "10:00"),
            slot("b", "s2", 1, "09:00", "11:00"),
            slot("c", "s3", 2, "08:00", "10:00"),
        ];
        let overlaps = find_overlaps(&slots);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].start_time, "09:00");
        assert_eq!(overlaps[0].end_time, "10:00");
    }

    #[test]
    fn attendance_respects_the_subject_minimum() {
        let subject = Subject {
            id: "s1".to_string(),
            term_id: "term".to_string(),
            name: "Álgebra".to_string(),
            code: None,
            color: DEFAULT_SUBJECT_COLOR.to_string(),
            teacher: None,
            room: None,
            credits: None,
            goal_score_cp: None,
            attendance_min_bp: Some(8_000),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let record = |id: &str, status: AttendanceStatus| AttendanceRecord {
            id: id.to_string(),
            class_slot_id: "slot".to_string(),
            date: "2026-03-02".to_string(),
            status,
            note: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let records = vec![
            record("1", AttendanceStatus::Present),
            record("2", AttendanceStatus::Late),
            record("3", AttendanceStatus::Absent),
            record("4", AttendanceStatus::Justified),
        ];

        let summary = attendance_summary(
            &[subject],
            &[slot("slot", "s1", 1, "08:00", "10:00")],
            &records,
        );
        assert_eq!(summary[0].counted, 4);
        assert_eq!(summary[0].attended, 3);
        assert_eq!(summary[0].percent_bp, Some(7_500));
        assert!(summary[0].below_min);
    }
}
