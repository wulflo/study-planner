//! Escalas de nota, categorías, entradas y cálculo del promedio ponderado.
//!
//! Todo se calcula con enteros de punto fijo:
//! - `*_cp`: centipuntos de la nota (`8500` = 85.00).
//! - `*_bp`: basis points de un porcentaje o peso (`5000` = 50 %).
//!
//! El promedio final se expresa en basis points (0–10000) y, si hay escala, se
//! proyecta a la escala configurada. El descarte del menor es determinista:
//! primero se ordena por porcentaje ascendente, luego por fecha ascendente
//! (las entradas sin fecha van al final) y por último por identificador.

use serde::Serialize;

use crate::academic::validate_basis_points;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeScale {
    pub id: String,
    pub subject_id: Option<String>,
    pub name: String,
    pub min_cp: i64,
    pub max_cp: i64,
    pub pass_cp: i64,
    pub precision: u8,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeCategory {
    pub id: String,
    pub subject_id: String,
    pub scale_id: String,
    pub name: String,
    pub weight_bp: i64,
    pub drop_lowest: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeEntry {
    pub id: String,
    pub subject_id: String,
    pub category_id: String,
    pub exam_id: Option<String>,
    pub title: String,
    pub score_cp: i64,
    pub max_cp: i64,
    pub weight_bp: Option<i64>,
    pub date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Resultado de una categoría.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryGrade {
    pub category_id: String,
    pub category_name: String,
    pub weight_bp: i64,
    pub entries: usize,
    pub dropped: usize,
    pub percent_bp: Option<i64>,
    pub average_cp: Option<i64>,
}

/// Resultado completo de una materia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectGrade {
    pub subject_id: String,
    pub scale_id: Option<String>,
    pub final_bp: Option<i64>,
    pub final_cp: Option<i64>,
    pub passed: Option<bool>,
    pub categories: Vec<CategoryGrade>,
}

/// Porcentaje de una entrada, redondeado a basis points.
pub fn entry_percent_bp(score_cp: i64, max_cp: i64) -> Option<i64> {
    if max_cp <= 0 {
        return None;
    }
    let score = score_cp.clamp(0, max_cp);
    Some(round_div(i128::from(score) * 10_000, i128::from(max_cp)))
}

/// Proyecta un porcentaje en basis points a centipuntos de una escala.
pub fn scale_cp_from_percent(percent_bp: i64, scale: &GradeScale) -> i64 {
    let percent = percent_bp.clamp(0, 10_000);
    let span = i128::from(scale.max_cp) - i128::from(scale.min_cp);
    let value =
        i128::from(scale.min_cp) + i128::from(round_div(i128::from(percent) * span, 10_000));
    value
        .clamp(i128::from(scale.min_cp), i128::from(scale.max_cp))
        .try_into()
        .unwrap_or(scale.min_cp)
}

/// Calcula una categoría. `entries` puede traer entradas de otras categorías:
/// se filtran por `category_id`.
pub fn grade_category_result(
    category: &GradeCategory,
    entries: &[GradeEntry],
    scale: Option<&GradeScale>,
) -> CategoryGrade {
    let mut items: Vec<(&GradeEntry, i64)> = entries
        .iter()
        .filter(|entry| entry.category_id == category.id)
        .filter_map(|entry| {
            entry_percent_bp(entry.score_cp, entry.max_cp).map(|percent| (entry, percent))
        })
        .collect();

    items.sort_by(|(left_entry, left_percent), (right_entry, right_percent)| {
        left_percent
            .cmp(right_percent)
            .then_with(|| match (&left_entry.date, &right_entry.date) {
                (Some(left), Some(right)) => left.cmp(right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| left_entry.id.cmp(&right_entry.id))
    });

    // Siempre se conserva al menos una entrada: descartar todo no tiene sentido.
    let drop = (category.drop_lowest as usize).min(items.len().saturating_sub(1));
    let kept = &items[drop..];

    let mut weighted_sum: i128 = 0;
    let mut weight_sum: i128 = 0;
    for (entry, percent) in kept {
        let weight = i128::from(entry.weight_bp.unwrap_or(entry.max_cp).max(0));
        weighted_sum += i128::from(*percent) * weight;
        weight_sum += weight;
    }

    let percent_bp = if kept.is_empty() {
        None
    } else if weight_sum == 0 {
        // Todas las entradas tienen peso explícito cero: promedio simple.
        Some(round_div(
            kept.iter().map(|(_, percent)| i128::from(*percent)).sum(),
            kept.len() as i128,
        ))
    } else {
        Some(round_div(weighted_sum, weight_sum))
    };

    CategoryGrade {
        category_id: category.id.clone(),
        category_name: category.name.clone(),
        weight_bp: category.weight_bp,
        entries: items.len(),
        dropped: drop,
        percent_bp,
        average_cp: match (percent_bp, scale) {
            (Some(percent), Some(scale)) => Some(scale_cp_from_percent(percent, scale)),
            _ => None,
        },
    }
}

/// Calcula el promedio final de una materia.
pub fn subject_grade(
    subject_id: &str,
    scale: Option<&GradeScale>,
    categories: &[GradeCategory],
    entries: &[GradeEntry],
) -> SubjectGrade {
    let category_results: Vec<CategoryGrade> = categories
        .iter()
        .map(|category| grade_category_result(category, entries, scale))
        .collect();

    let mut weighted_sum: i128 = 0;
    let mut weight_sum: i128 = 0;
    for category in &category_results {
        if let Some(percent) = category.percent_bp
            && category.weight_bp > 0
        {
            weighted_sum += i128::from(percent) * i128::from(category.weight_bp);
            weight_sum += i128::from(category.weight_bp);
        }
    }

    let final_bp = if weight_sum == 0 {
        None
    } else {
        Some(round_div(weighted_sum, weight_sum))
    };

    let final_cp = match (final_bp, scale) {
        (Some(percent), Some(scale)) => Some(scale_cp_from_percent(percent, scale)),
        _ => None,
    };
    let passed = match (final_cp, scale) {
        (Some(score), Some(scale)) => Some(score >= scale.pass_cp),
        _ => None,
    };

    SubjectGrade {
        subject_id: subject_id.to_string(),
        scale_id: scale.map(|scale| scale.id.clone()),
        final_bp,
        final_cp,
        passed,
        categories: category_results,
    }
}

/// Valida una escala: mínimo menor que máximo y aprobación dentro del rango.
pub fn validate_grade_scale(
    min_cp: i64,
    max_cp: i64,
    pass_cp: i64,
    precision: i64,
) -> Result<(), &'static str> {
    if min_cp >= max_cp {
        return Err("El mínimo de la escala debe ser menor que el máximo.");
    }
    if pass_cp < min_cp || pass_cp > max_cp {
        return Err("La nota de aprobación debe estar dentro de la escala.");
    }
    if !(0..=4).contains(&precision) {
        return Err("La precisión debe estar entre 0 y 4.");
    }
    Ok(())
}

/// Valida el peso de una categoría o entrada.
pub fn validate_grade_weight(value: i64) -> Result<i64, &'static str> {
    validate_basis_points(value)
}

/// Valida una entrada de nota.
pub fn validate_grade_entry(score_cp: i64, max_cp: i64) -> Result<(), &'static str> {
    if max_cp <= 0 {
        return Err("El máximo de la nota debe ser mayor que cero.");
    }
    if score_cp < 0 || score_cp > max_cp {
        return Err("La nota debe estar entre 0 y su máximo.");
    }
    Ok(())
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

    fn scale(min: i64, max: i64, pass: i64) -> GradeScale {
        GradeScale {
            id: "scale".to_string(),
            subject_id: Some("subject".to_string()),
            name: "Escala".to_string(),
            min_cp: min,
            max_cp: max,
            pass_cp: pass,
            precision: 2,
            is_default: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn category(id: &str, weight: i64, drop: u32) -> GradeCategory {
        GradeCategory {
            id: id.to_string(),
            subject_id: "subject".to_string(),
            scale_id: "scale".to_string(),
            name: id.to_string(),
            weight_bp: weight,
            drop_lowest: drop,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn entry(
        id: &str,
        category: &str,
        score: i64,
        max: i64,
        weight: Option<i64>,
        date: Option<&str>,
    ) -> GradeEntry {
        GradeEntry {
            id: id.to_string(),
            subject_id: "subject".to_string(),
            category_id: category.to_string(),
            exam_id: None,
            title: id.to_string(),
            score_cp: score,
            max_cp: max,
            weight_bp: weight,
            date: date.map(str::to_string),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn entry_percent_rounds_to_basis_points() {
        assert_eq!(entry_percent_bp(8_000, 10_000), Some(8_000));
        assert_eq!(entry_percent_bp(1, 3), Some(3_333));
        assert_eq!(entry_percent_bp(10, 3), Some(10_000));
        assert_eq!(entry_percent_bp(1, 0), None);
    }

    #[test]
    fn category_weighted_average_uses_entry_weights() {
        let entries = vec![
            entry("a", "cat", 10_000, 10_000, Some(3_000), None),
            entry("b", "cat", 5_000, 10_000, Some(1_000), None),
        ];
        let result = grade_category_result(&category("cat", 10_000, 0), &entries, None);
        assert_eq!(result.percent_bp, Some(8_750));
    }

    #[test]
    fn drops_the_lowest_entry_deterministically() {
        let entries = vec![
            entry("a", "cat", 9_000, 10_000, None, Some("2026-03-01")),
            entry("b", "cat", 1_000, 10_000, None, Some("2026-03-02")),
            entry("c", "cat", 8_000, 10_000, None, Some("2026-03-03")),
        ];
        let result = grade_category_result(&category("cat", 10_000, 1), &entries, None);
        assert_eq!(result.entries, 3);
        assert_eq!(result.dropped, 1);
        assert_eq!(result.percent_bp, Some(8_500));
    }

    #[test]
    fn dropping_more_than_available_keeps_the_best_entry() {
        let entries = vec![
            entry("a", "cat", 2_000, 10_000, None, None),
            entry("b", "cat", 7_000, 10_000, None, None),
        ];
        let result = grade_category_result(&category("cat", 10_000, 9), &entries, None);
        assert_eq!(result.dropped, 1);
        assert_eq!(result.percent_bp, Some(7_000));
    }

    #[test]
    fn final_average_combines_category_weights() {
        let entries = vec![
            entry("a", "cat-a", 9_000, 10_000, None, None),
            entry("b", "cat-b", 5_000, 10_000, None, None),
        ];
        let categories = vec![category("cat-a", 5_000, 0), category("cat-b", 5_000, 0)];
        let result = subject_grade(
            "subject",
            Some(&scale(0, 10_000, 6_000)),
            &categories,
            &entries,
        );
        assert_eq!(result.final_bp, Some(7_000));
        assert_eq!(result.final_cp, Some(7_000));
        assert_eq!(result.passed, Some(true));
    }

    #[test]
    fn maps_percent_to_a_custom_scale() {
        let entries = vec![entry("a", "cat", 7_000, 10_000, None, None)];
        let result = subject_grade(
            "subject",
            Some(&scale(0, 100, 60)),
            &[category("cat", 10_000, 0)],
            &entries,
        );
        assert_eq!(result.final_cp, Some(70));
        assert_eq!(result.passed, Some(true));
    }

    #[test]
    fn empty_or_zero_weight_categories_produce_no_final_grade() {
        let result = subject_grade("subject", None, &[category("cat", 10_000, 0)], &[]);
        assert_eq!(result.final_bp, None);
        assert_eq!(result.final_cp, None);
        assert_eq!(result.passed, None);

        let entries = vec![entry("a", "cat", 8_000, 10_000, None, None)];
        let result = subject_grade("subject", None, &[category("cat", 0, 0)], &entries);
        assert_eq!(result.final_bp, None);
    }

    #[test]
    fn validates_scales_entries_and_weights() {
        assert!(validate_grade_scale(0, 100, 60, 2).is_ok());
        assert!(validate_grade_scale(100, 0, 60, 2).is_err());
        assert!(validate_grade_scale(0, 100, 120, 2).is_err());
        assert!(validate_grade_scale(0, 100, 60, 9).is_err());
        assert!(validate_grade_entry(80, 100).is_ok());
        assert!(validate_grade_entry(-1, 100).is_err());
        assert!(validate_grade_entry(101, 100).is_err());
        assert!(validate_grade_entry(10, 0).is_err());
        assert_eq!(validate_grade_weight(5_000).unwrap(), 5_000);
        assert!(validate_grade_weight(10_001).is_err());
    }
}
