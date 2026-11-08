//! Estudio, autoevaluación, repaso espaciado y estadísticas.
//!
//! Todo el cálculo es puro y determinista: no toca SQLite, la red ni el
//! sistema de archivos. Las reglas viven acá para que la UI y el store
//! compartan exactamente la misma semántica.
//!
//! Repaso espaciado: SM-2 con `ease_factor_cp` en centésimas (250 = 2.50) y
//! `algorithm_version = "1"`. Una calidad menor que 3 reinicia las
//! repeticiones y cuenta un lapso; el resto aplica los intervalos 1, 6 y
//! `intervalo * ease` redondeado.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::academic::{add_days, days_between, normalize_date, normalize_optional_text};

/// Calidad mínima de una autoevaluación o repaso.
pub const ASSESSMENT_MIN: u8 = 1;
/// Calidad máxima de una autoevaluación o repaso.
pub const ASSESSMENT_MAX: u8 = 5;

/// Factor de facilidad por defecto (2.50).
pub const EASE_DEFAULT_CP: i64 = 250;
/// Factor de facilidad mínimo (1.30), igual al `CHECK` del esquema.
pub const EASE_MIN_CP: i64 = 130;

/// Identificador del algoritmo de repaso.
pub const REVIEW_ALGORITHM: &str = "sm2";
/// Versión del algoritmo de repaso.
pub const REVIEW_ALGORITHM_VERSION: &str = "1";

/// Duración por defecto de un Pomodoro, en minutos.
pub const DEFAULT_POMODORO_MINUTES: i64 = 25;
/// Duración por defecto de un descanso, en minutos.
pub const DEFAULT_BREAK_MINUTES: i64 = 5;

/// Umbral por defecto para clasificar una materia como débil (40 %).
pub const DEFAULT_WEAK_THRESHOLD_BP: i64 = 4_000;
/// Umbral por defecto para clasificar una materia como fuerte (70 %).
pub const DEFAULT_STRONG_THRESHOLD_BP: i64 = 7_000;
/// Días de decaimiento usados por la prioridad de estudio.
pub const DEFAULT_PRIORITY_DECAY_DAYS: i64 = 7;
/// Horizonte visible de próximos repasos, en días.
pub const REVIEW_HORIZON_DAYS: i64 = 30;

/// Origen de una autoevaluación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentSource {
    Manual,
    Prompt,
    Review,
}

impl AssessmentSource {
    pub const ALL: [Self; 3] = [Self::Manual, Self::Prompt, Self::Review];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Prompt => "prompt",
            Self::Review => "review",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "prompt" => Some(Self::Prompt),
            "review" => Some(Self::Review),
            _ => None,
        }
    }
}

/// Fortaleza percibida de un tema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrengthClassification {
    Weak,
    Neutral,
    Strength,
}

impl StrengthClassification {
    pub const ALL: [Self; 3] = [Self::Weak, Self::Neutral, Self::Strength];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Weak => "weak",
            Self::Neutral => "neutral",
            Self::Strength => "strength",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "weak" => Some(Self::Weak),
            "neutral" => Some(Self::Neutral),
            "strength" => Some(Self::Strength),
            _ => None,
        }
    }
}

/// Modo de una sesión de estudio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StudySessionMode {
    Pomodoro,
    Free,
    Review,
}

impl StudySessionMode {
    pub const ALL: [Self; 3] = [Self::Pomodoro, Self::Free, Self::Review];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pomodoro => "pomodoro",
            Self::Free => "free",
            Self::Review => "review",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "pomodoro" => Some(Self::Pomodoro),
            "free" => Some(Self::Free),
            "review" => Some(Self::Review),
            _ => None,
        }
    }
}

/// Autoevaluación de un tema, con escala 1–5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfAssessment {
    pub id: String,
    pub topic_id: String,
    pub score: u8,
    pub note: Option<String>,
    pub source: AssessmentSource,
    pub assessed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Override manual de la clasificación de un tema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrengthOverride {
    pub id: String,
    pub topic_id: String,
    pub classification: StrengthClassification,
    pub reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Estado SM-2 de un tema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub id: String,
    pub topic_id: String,
    pub algorithm: String,
    pub algorithm_version: String,
    pub ease_factor_cp: i64,
    pub interval_days: i64,
    pub repetitions: i64,
    pub lapses: i64,
    pub due_date: Option<String>,
    pub last_reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Estado SM-2 mínimo para calcular el próximo repaso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sm2State {
    pub ease_factor_cp: i64,
    pub interval_days: i64,
    pub repetitions: i64,
    pub lapses: i64,
}

impl Default for Sm2State {
    fn default() -> Self {
        Self {
            ease_factor_cp: EASE_DEFAULT_CP,
            interval_days: 0,
            repetitions: 0,
            lapses: 0,
        }
    }
}

/// Resultado de aplicar una calidad SM-2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sm2Result {
    pub ease_factor_cp: i64,
    pub interval_days: i64,
    pub repetitions: i64,
    pub lapses: i64,
    pub due_date: String,
}

/// Sesión de estudio registrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    pub id: String,
    pub subject_id: Option<String>,
    pub topic_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub planned_minutes: Option<i64>,
    pub actual_minutes: Option<i64>,
    pub mode: StudySessionMode,
    pub interruptions: i64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Motivo puntuado de la prioridad de estudio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriorityReason {
    pub code: String,
    pub label: String,
    pub points: i64,
}

/// Prioridad calculada para un tema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPriority {
    pub topic_id: String,
    pub score: i64,
    pub classification: StrengthClassification,
    pub reasons: Vec<PriorityReason>,
}

/// Datos de entrada para calcular una prioridad.
#[derive(Debug, Clone)]
pub struct StudyPriorityInput<'a> {
    pub topic_id: &'a str,
    pub classification: StrengthClassification,
    pub review_due: Option<&'a str>,
    pub exam_date: Option<&'a str>,
    pub today: &'a str,
    pub decay_days: i64,
}

/// Fila mínima de un tema para agregar fortalezas por materia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicStrengthRow {
    pub topic_id: String,
    pub subject_id: String,
    pub classification: StrengthClassification,
    pub latest_score: Option<u8>,
}

/// Resumen de una materia con temas débiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakSubject {
    pub subject_id: String,
    pub topic_count: usize,
    pub weak_topics: usize,
    pub average_score_bp: Option<i64>,
}

/// Próximo repaso de un tema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingReview {
    pub topic_id: String,
    pub due_date: String,
    pub days_until: i64,
}

/// Minutos acumulados por materia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectMinutes {
    pub subject_id: Option<String>,
    pub minutes: i64,
    pub sessions: usize,
}

/// Totales de estudio de un período.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTotals {
    pub total_minutes: i64,
    pub session_count: usize,
    pub current_streak_days: i64,
    pub longest_streak_days: i64,
}

/// Valida una calidad 1–5 y la devuelve como `u8`.
pub fn validate_assessment_score(score: i64) -> Result<u8, &'static str> {
    if (i64::from(ASSESSMENT_MIN)..=i64::from(ASSESSMENT_MAX)).contains(&score) {
        Ok(score as u8)
    } else {
        Err("La autoevaluación debe estar entre 1 y 5.")
    }
}

/// Normaliza la nota opcional de una autoevaluación.
pub fn normalize_assessment_note(raw: &str) -> Result<Option<String>, &'static str> {
    normalize_optional_text(raw)
}

/// Valida los umbrales de clasificación en basis points.
pub fn validate_strength_thresholds(weak_bp: i64, strong_bp: i64) -> Result<(), &'static str> {
    if !(0..=10_000).contains(&weak_bp) || !(0..=10_000).contains(&strong_bp) {
        return Err("Los umbrales de fortaleza deben estar entre 0 y 10000 basis points.");
    }
    if weak_bp >= strong_bp {
        return Err("El umbral de debilidad debe ser menor que el de fortaleza.");
    }
    Ok(())
}

/// Clasifica un tema a partir de su última autoevaluación.
///
/// Sin autoevaluación el tema es neutral. El puntaje se proyecta a basis
/// points (1 = 2000, 5 = 10000) y se compara con los umbrales.
pub fn classification_from_score(
    score: Option<u8>,
    weak_threshold_bp: i64,
    strong_threshold_bp: i64,
) -> StrengthClassification {
    let Some(score) = score else {
        return StrengthClassification::Neutral;
    };
    let score_bp = i64::from(score.clamp(ASSESSMENT_MIN, ASSESSMENT_MAX)) * 2_000;
    if score_bp <= weak_threshold_bp {
        StrengthClassification::Weak
    } else if score_bp >= strong_threshold_bp {
        StrengthClassification::Strength
    } else {
        StrengthClassification::Neutral
    }
}

/// Aplica un override manual sobre la clasificación derivada.
pub fn effective_classification(
    derived: StrengthClassification,
    override_classification: Option<StrengthClassification>,
) -> StrengthClassification {
    override_classification.unwrap_or(derived)
}

/// Calcula el próximo repaso SM-2 para una calidad 1–5.
pub fn sm2_review(state: &Sm2State, quality: u8, today: &str) -> Result<Sm2Result, &'static str> {
    let today = normalize_date(today)?;
    if !(ASSESSMENT_MIN..=ASSESSMENT_MAX).contains(&quality) {
        return Err("La calidad del repaso debe estar entre 1 y 5.");
    }

    let quality = i64::from(quality);
    let ease = state.ease_factor_cp.max(EASE_MIN_CP);
    let delta = 10 - (5 - quality) * (8 + (5 - quality) * 2);
    let ease_factor_cp = (ease + delta).max(EASE_MIN_CP);

    let (interval_days, repetitions, lapses) = if quality < 3 {
        (1, 0, state.lapses + 1)
    } else {
        let interval_days = match state.repetitions {
            0 => 1,
            1 => 6,
            _ => {
                let scaled = round_div(
                    i128::from(state.interval_days.max(1)) * i128::from(ease_factor_cp),
                    100,
                );
                scaled.max(1)
            }
        };
        (interval_days, state.repetitions + 1, state.lapses)
    };

    let due_date = add_days(&today, interval_days)
        .ok_or("No se pudo calcular la fecha del próximo repaso.")?;

    Ok(Sm2Result {
        ease_factor_cp,
        interval_days,
        repetitions,
        lapses,
        due_date,
    })
}

/// Calcula la prioridad explicable de un tema.
///
/// La suma es determinista y cada componente queda registrado en `reasons`
/// para que la UI pueda justificar el orden. No usa azar ni el reloj salvo la
/// fecha `today` recibida.
pub fn study_priority(input: &StudyPriorityInput<'_>) -> StudyPriority {
    let mut score = 0;
    let mut reasons = Vec::new();

    let (code, label, points) = match input.classification {
        StrengthClassification::Weak => ("weak", "Tema débil", 4_000),
        StrengthClassification::Neutral => ("neutral", "Tema neutral", 2_000),
        StrengthClassification::Strength => ("strength", "Tema fuerte", 500),
    };
    score += points;
    reasons.push(PriorityReason {
        code: code.to_string(),
        label: label.to_string(),
        points,
    });

    if let Some(due) = input.review_due
        && let Some(days) = days_between(due, input.today)
    {
        if days >= 0 {
            let points = 3_000 + days.min(input.decay_days.max(1)) * 200;
            score += points;
            reasons.push(PriorityReason {
                code: "review_overdue".to_string(),
                label: format!("Repaso vencido hace {days} día(s)"),
                points,
            });
        } else {
            let days_until = -days;
            if days_until <= input.decay_days {
                let points = (input.decay_days - days_until + 1) * 300;
                score += points;
                reasons.push(PriorityReason {
                    code: "review_due".to_string(),
                    label: format!("Repaso en {days_until} día(s)"),
                    points,
                });
            }
        }
    }

    if let Some(exam_date) = input.exam_date
        && let Some(days_until) = days_between(input.today, exam_date)
        && (0..=input.decay_days).contains(&days_until)
    {
        let points = (input.decay_days - days_until + 1) * 400;
        score += points;
        reasons.push(PriorityReason {
            code: "exam_soon".to_string(),
            label: format!("Examen en {days_until} día(s)"),
            points,
        });
    }

    StudyPriority {
        topic_id: input.topic_id.to_string(),
        score,
        classification: input.classification,
        reasons,
    }
}

/// Ordena prioridades: mayor puntaje primero y, a igualdad, por tema.
pub fn sort_study_priorities(items: &mut [StudyPriority]) {
    items.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.topic_id.cmp(&right.topic_id))
    });
}

/// Minutos efectivos de una sesión. Sin `actual_minutes` cuenta cero.
pub fn effective_session_minutes(session: &StudySession) -> i64 {
    session.actual_minutes.unwrap_or(0).max(0)
}

/// Calcula rachas de constancia a partir de fechas `AAAA-MM-DD`.
///
/// La racha actual cuenta desde hoy; si hoy no hubo sesión, desde ayer. La
/// racha más larga se calcula sobre el histórico completo.
pub fn streak_days(dates: &[String], today: &str) -> (i64, i64) {
    let Some(today) = normalize_date(today).ok() else {
        return (0, 0);
    };

    let mut days: Vec<i64> = dates
        .iter()
        .filter_map(|value| {
            let date = value.get(..10)?;
            days_between("1970-01-01", &normalize_date(date).ok()?)
        })
        .collect();
    days.sort_unstable();
    days.dedup();

    if days.is_empty() {
        return (0, 0);
    }

    let mut longest = 1;
    let mut run = 1;
    for pair in days.windows(2) {
        if pair[1] == pair[0] + 1 {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }

    let today_number = days_between("1970-01-01", &today).unwrap_or_default();
    let yesterday = today_number - 1;
    let mut cursor = if days.contains(&today_number) {
        today_number
    } else if days.contains(&yesterday) {
        yesterday
    } else {
        return (0, longest);
    };

    let mut current = 0;
    while days.contains(&cursor) {
        current += 1;
        cursor -= 1;
    }

    (current, longest)
}

/// Totales de un conjunto de sesiones.
pub fn session_totals(sessions: &[StudySession], today: &str) -> StudyTotals {
    let dates: Vec<String> = sessions
        .iter()
        .map(|session| session.started_at.clone())
        .collect();
    let (current_streak_days, longest_streak_days) = streak_days(&dates, today);
    StudyTotals {
        total_minutes: sessions.iter().map(effective_session_minutes).sum(),
        session_count: sessions.len(),
        current_streak_days,
        longest_streak_days,
    }
}

/// Agrupa minutos por materia, de mayor a menor.
pub fn minutes_by_subject(sessions: &[StudySession]) -> Vec<SubjectMinutes> {
    let mut totals: Vec<SubjectMinutes> = Vec::new();
    for session in sessions {
        let minutes = effective_session_minutes(session);
        match totals
            .iter_mut()
            .find(|item| item.subject_id == session.subject_id)
        {
            Some(item) => {
                item.minutes += minutes;
                item.sessions += 1;
            }
            None => totals.push(SubjectMinutes {
                subject_id: session.subject_id.clone(),
                minutes,
                sessions: 1,
            }),
        }
    }
    totals.sort_by(|left, right| {
        right
            .minutes
            .cmp(&left.minutes)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
    });
    totals
}

/// Agrupa temas por materia y resume cuántos están débiles.
pub fn summarize_weak_subjects(rows: &[TopicStrengthRow]) -> Vec<WeakSubject> {
    // subject_id -> (temas, débiles, suma de puntajes en bp, puntajes contados)
    let mut map: BTreeMap<String, (usize, usize, i128, i64)> = BTreeMap::new();
    for row in rows {
        let entry = map.entry(row.subject_id.clone()).or_default();
        entry.0 += 1;
        if row.classification == StrengthClassification::Weak {
            entry.1 += 1;
        }
        if let Some(score) = row.latest_score {
            entry.2 += i128::from(score.clamp(ASSESSMENT_MIN, ASSESSMENT_MAX)) * 2_000;
            entry.3 += 1;
        }
    }

    let mut subjects: Vec<WeakSubject> = map
        .into_iter()
        .map(
            |(subject_id, (topic_count, weak_topics, sum, count))| WeakSubject {
                subject_id,
                topic_count,
                weak_topics,
                average_score_bp: (count > 0).then(|| round_div(sum, i128::from(count))),
            },
        )
        .collect();

    subjects.retain(|subject| subject.weak_topics > 0);
    subjects.sort_by(|left, right| {
        right
            .weak_topics
            .cmp(&left.weak_topics)
            .then_with(|| left.average_score_bp.cmp(&right.average_score_bp))
            .then_with(|| left.subject_id.cmp(&right.subject_id))
    });
    subjects
}

/// Lista repasos desde vencidos hasta el horizonte, ordenados por fecha.
pub fn upcoming_reviews(
    items: &[(String, String)],
    today: &str,
    horizon_days: i64,
) -> Vec<UpcomingReview> {
    let mut reviews: Vec<UpcomingReview> = items
        .iter()
        .filter_map(|(topic_id, due_date)| {
            let days_until = days_between(today, due_date)?;
            (days_until <= horizon_days).then(|| UpcomingReview {
                topic_id: topic_id.clone(),
                due_date: due_date.clone(),
                days_until,
            })
        })
        .collect();
    reviews.sort_by(|left, right| {
        left.due_date
            .cmp(&right.due_date)
            .then_with(|| left.topic_id.cmp(&right.topic_id))
    });
    reviews
}

fn round_div(numerator: i128, denominator: i128) -> i64 {
    if denominator == 0 {
        return 0;
    }
    let value = if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        (numerator - denominator / 2) / denominator
    };
    value as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, subject: Option<&str>, started_at: &str, minutes: i64) -> StudySession {
        StudySession {
            id: id.to_string(),
            subject_id: subject.map(str::to_string),
            topic_id: None,
            started_at: started_at.to_string(),
            ended_at: None,
            planned_minutes: Some(minutes),
            actual_minutes: Some(minutes),
            mode: StudySessionMode::Pomodoro,
            interruptions: 0,
            notes: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn validates_scores_and_classifies_thresholds() {
        assert!(validate_assessment_score(0).is_err());
        assert!(validate_assessment_score(6).is_err());
        assert_eq!(validate_assessment_score(4).unwrap(), 4);

        assert_eq!(
            classification_from_score(None, 4_000, 7_000),
            StrengthClassification::Neutral
        );
        assert_eq!(
            classification_from_score(Some(1), 4_000, 7_000),
            StrengthClassification::Weak
        );
        assert_eq!(
            classification_from_score(Some(3), 4_000, 7_000),
            StrengthClassification::Neutral
        );
        assert_eq!(
            classification_from_score(Some(4), 4_000, 7_000),
            StrengthClassification::Strength
        );

        assert_eq!(
            effective_classification(
                StrengthClassification::Weak,
                Some(StrengthClassification::Strength)
            ),
            StrengthClassification::Strength
        );
    }

    #[test]
    fn validates_strength_thresholds() {
        assert!(validate_strength_thresholds(4_000, 7_000).is_ok());
        assert!(validate_strength_thresholds(7_000, 4_000).is_err());
        assert!(validate_strength_thresholds(-1, 7_000).is_err());
    }

    #[test]
    fn sm2_success_progression_is_deterministic() {
        let first = sm2_review(&Sm2State::default(), 4, "2026-03-01").unwrap();
        assert_eq!(first.ease_factor_cp, 250);
        assert_eq!(first.interval_days, 1);
        assert_eq!(first.repetitions, 1);
        assert_eq!(first.due_date, "2026-03-02");

        let second = sm2_review(
            &Sm2State {
                ease_factor_cp: first.ease_factor_cp,
                interval_days: first.interval_days,
                repetitions: first.repetitions,
                lapses: first.lapses,
            },
            4,
            "2026-03-02",
        )
        .unwrap();
        assert_eq!(second.interval_days, 6);
        assert_eq!(second.repetitions, 2);
        assert_eq!(second.due_date, "2026-03-08");

        let third = sm2_review(
            &Sm2State {
                ease_factor_cp: second.ease_factor_cp,
                interval_days: second.interval_days,
                repetitions: second.repetitions,
                lapses: second.lapses,
            },
            4,
            "2026-03-08",
        )
        .unwrap();
        assert_eq!(third.interval_days, 15);
        assert_eq!(third.repetitions, 3);
    }

    #[test]
    fn sm2_failure_resets_and_counts_lapse() {
        let result = sm2_review(
            &Sm2State {
                ease_factor_cp: 250,
                interval_days: 30,
                repetitions: 5,
                lapses: 1,
            },
            1,
            "2026-03-01",
        )
        .unwrap();
        assert_eq!(result.interval_days, 1);
        assert_eq!(result.repetitions, 0);
        assert_eq!(result.lapses, 2);
        assert_eq!(result.ease_factor_cp, 196);
        assert_eq!(result.due_date, "2026-03-02");
    }

    #[test]
    fn sm2_ease_never_goes_below_minimum() {
        let mut state = Sm2State::default();
        for _ in 0..20 {
            let result = sm2_review(&state, 1, "2026-03-01").unwrap();
            state = Sm2State {
                ease_factor_cp: result.ease_factor_cp,
                interval_days: result.interval_days,
                repetitions: result.repetitions,
                lapses: result.lapses,
            };
        }
        assert_eq!(state.ease_factor_cp, EASE_MIN_CP);
    }

    #[test]
    fn priority_is_explainable_and_sorted() {
        let weak = study_priority(&StudyPriorityInput {
            topic_id: "topic-a",
            classification: StrengthClassification::Weak,
            review_due: Some("2026-03-01"),
            exam_date: Some("2026-03-03"),
            today: "2026-03-02",
            decay_days: 7,
        });
        let strong = study_priority(&StudyPriorityInput {
            topic_id: "topic-b",
            classification: StrengthClassification::Strength,
            review_due: None,
            exam_date: None,
            today: "2026-03-02",
            decay_days: 7,
        });

        assert_eq!(weak.score, 4_000 + 3_200 + 2_800);
        assert_eq!(strong.score, 500);
        assert_eq!(weak.reasons.len(), 3);
        assert_eq!(weak.reasons[1].code, "review_overdue");
        assert_eq!(weak.reasons[2].code, "exam_soon");

        let mut items = vec![strong, weak];
        sort_study_priorities(&mut items);
        assert_eq!(items[0].topic_id, "topic-a");
    }

    #[test]
    fn priority_only_counts_reviews_inside_the_window() {
        let far = study_priority(&StudyPriorityInput {
            topic_id: "topic",
            classification: StrengthClassification::Neutral,
            review_due: Some("2026-04-30"),
            exam_date: None,
            today: "2026-03-02",
            decay_days: 7,
        });
        assert_eq!(far.score, 2_000);
        assert_eq!(far.reasons.len(), 1);
    }

    #[test]
    fn streaks_handle_today_yesterday_and_gaps() {
        let dates = vec![
            "2026-03-01".to_string(),
            "2026-03-02".to_string(),
            "2026-03-04".to_string(),
            "2026-03-05".to_string(),
            "2026-03-06T10:00:00.000Z".to_string(),
        ];
        assert_eq!(streak_days(&dates, "2026-03-06"), (3, 3));
        assert_eq!(streak_days(&dates, "2026-03-07"), (3, 3));
        assert_eq!(streak_days(&dates, "2026-03-08"), (0, 3));
        assert_eq!(streak_days(&[], "2026-03-08"), (0, 0));
    }

    #[test]
    fn session_totals_and_subject_minutes() {
        let sessions = vec![
            session("s1", Some("subject-a"), "2026-03-05T10:00:00.000Z", 25),
            session("s2", Some("subject-a"), "2026-03-06T10:00:00.000Z", 50),
            session("s3", Some("subject-b"), "2026-03-06T12:00:00.000Z", 10),
            session("s4", None, "2026-03-06T13:00:00.000Z", 5),
        ];
        let totals = session_totals(&sessions, "2026-03-06");
        assert_eq!(totals.total_minutes, 90);
        assert_eq!(totals.session_count, 4);
        assert_eq!(totals.current_streak_days, 2);
        assert_eq!(totals.longest_streak_days, 2);

        let by_subject = minutes_by_subject(&sessions);
        assert_eq!(by_subject[0].subject_id.as_deref(), Some("subject-a"));
        assert_eq!(by_subject[0].minutes, 75);
        assert_eq!(by_subject[1].subject_id.as_deref(), Some("subject-b"));
        assert_eq!(by_subject[2].subject_id, None);
    }

    #[test]
    fn weak_subjects_average_available_scores() {
        let rows = vec![
            TopicStrengthRow {
                topic_id: "t1".to_string(),
                subject_id: "a".to_string(),
                classification: StrengthClassification::Weak,
                latest_score: Some(2),
            },
            TopicStrengthRow {
                topic_id: "t2".to_string(),
                subject_id: "a".to_string(),
                classification: StrengthClassification::Weak,
                latest_score: Some(1),
            },
            TopicStrengthRow {
                topic_id: "t3".to_string(),
                subject_id: "a".to_string(),
                classification: StrengthClassification::Strength,
                latest_score: Some(5),
            },
            TopicStrengthRow {
                topic_id: "t4".to_string(),
                subject_id: "b".to_string(),
                classification: StrengthClassification::Strength,
                latest_score: None,
            },
        ];
        let subjects = summarize_weak_subjects(&rows);
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].subject_id, "a");
        assert_eq!(subjects[0].weak_topics, 2);
        assert_eq!(subjects[0].topic_count, 3);
        assert!(subjects[0].average_score_bp.unwrap() > 0);
    }

    #[test]
    fn upcoming_reviews_are_sorted_and_filtered() {
        let items = vec![
            ("topic-b".to_string(), "2026-03-05".to_string()),
            ("topic-a".to_string(), "2026-03-01".to_string()),
            ("topic-c".to_string(), "2026-06-01".to_string()),
        ];
        let reviews = upcoming_reviews(&items, "2026-03-02", 30);
        assert_eq!(reviews.len(), 2);
        assert_eq!(reviews[0].topic_id, "topic-a");
        assert_eq!(reviews[0].days_until, -1);
        assert_eq!(reviews[1].topic_id, "topic-b");
    }
}
