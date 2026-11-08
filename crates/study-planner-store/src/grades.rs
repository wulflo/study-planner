//! Repositorio de escalas, categorías y entradas de nota.
//!
//! El cálculo del promedio vive en `study-planner-core`; este módulo solo
//! persiste datos validados y arma la vista compuesta por materia.

use rusqlite::types::Value;
use rusqlite::{OptionalExtension, Row, params, params_from_iter};
use serde::Deserialize;
use study_planner_core::{
    GradeCategory, GradeEntry, GradeScale, SubjectGrade, new_id, normalize_date, normalize_name,
    subject_grade, validate_basis_points, validate_exam_title, validate_grade_entry,
    validate_grade_scale,
};

use crate::{Store, StoreError};

const SCALE_COLUMNS: &str = "id, subject_id, name, min_cp, max_cp, pass_cp, precision, \
     is_default, created_at, updated_at";
const CATEGORY_COLUMNS: &str = "id, subject_id, scale_id, name, weight_bp, drop_lowest, \
     created_at, updated_at";
const ENTRY_COLUMNS: &str = "id, subject_id, category_id, exam_id, title, score_cp, max_cp, \
     weight_bp, date, created_at, updated_at";

/// Datos editables de una escala de nota.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GradeScaleInput {
    pub subject_id: Option<String>,
    pub name: String,
    pub min_cp: i64,
    pub max_cp: i64,
    pub pass_cp: i64,
    pub precision: i64,
    pub is_default: bool,
}

/// Datos editables de una categoría.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GradeCategoryInput {
    pub scale_id: String,
    pub name: String,
    pub weight_bp: i64,
    pub drop_lowest: i64,
}

/// Datos editables de una entrada de nota.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GradeEntryInput {
    pub category_id: String,
    pub exam_id: Option<String>,
    pub title: String,
    pub score_cp: i64,
    pub max_cp: i64,
    pub weight_bp: Option<i64>,
    pub date: Option<String>,
}

impl Store {
    /// Lista escalas vivas. Con materia incluye las globales y las suyas.
    pub fn list_grade_scales(
        &self,
        subject_id: Option<&str>,
    ) -> Result<Vec<GradeScale>, StoreError> {
        let mut sql = format!("SELECT {SCALE_COLUMNS} FROM grade_scale WHERE deleted_at IS NULL");
        let mut values: Vec<Value> = Vec::new();
        if let Some(subject_id) = subject_id {
            sql.push_str(" AND (subject_id = ? OR subject_id IS NULL)");
            values.push(Value::Text(subject_id.to_string()));
        }
        sql.push_str(" ORDER BY is_default DESC, name");

        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), scale_from_row)?;
        let mut scales = Vec::new();
        for row in rows {
            scales.push(row?);
        }
        Ok(scales)
    }

    /// Crea una escala de nota.
    pub fn create_grade_scale(&self, input: &GradeScaleInput) -> Result<GradeScale, StoreError> {
        let values = validate_scale_input(input)?;
        if let Some(subject_id) = &values.subject_id {
            self.ensure_live_subject(subject_id)?;
        }

        let id = new_id();
        if values.is_default {
            self.clear_default_scales(values.subject_id.as_deref())?;
        }
        self.connection().execute(
            "INSERT INTO grade_scale
               (id, subject_id, name, min_cp, max_cp, pass_cp, precision, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                values.subject_id,
                values.name,
                values.min_cp,
                values.max_cp,
                values.pass_cp,
                values.precision,
                values.is_default,
            ],
        )?;

        self.get_grade_scale(&id)?
            .ok_or(StoreError::GradeScaleNotFound)
    }

    /// Actualiza una escala viva.
    pub fn update_grade_scale(
        &self,
        id: &str,
        input: &GradeScaleInput,
    ) -> Result<GradeScale, StoreError> {
        self.get_grade_scale(id)?
            .ok_or(StoreError::GradeScaleNotFound)?;
        let values = validate_scale_input(input)?;
        if let Some(subject_id) = &values.subject_id {
            self.ensure_live_subject(subject_id)?;
        }

        if values.is_default {
            self.clear_default_scales(values.subject_id.as_deref())?;
        }
        let changed = self.connection().execute(
            "UPDATE grade_scale
             SET subject_id = ?1, name = ?2, min_cp = ?3, max_cp = ?4, pass_cp = ?5,
                 precision = ?6, is_default = ?7,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?8 AND deleted_at IS NULL",
            params![
                values.subject_id,
                values.name,
                values.min_cp,
                values.max_cp,
                values.pass_cp,
                values.precision,
                values.is_default,
                id,
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::GradeScaleNotFound);
        }
        self.get_grade_scale(id)?
            .ok_or(StoreError::GradeScaleNotFound)
    }

    /// Borra lógicamente una escala sin categorías vivas.
    pub fn delete_grade_scale(&self, id: &str) -> Result<(), StoreError> {
        let categories: i64 = self.connection().query_row(
            "SELECT count(*) FROM grade_category WHERE scale_id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        if categories > 0 {
            return Err(StoreError::Conflict(
                "La escala tiene categorías vivas; borralas o movelas primero.",
            ));
        }

        let changed = self.connection().execute(
            "UPDATE grade_scale
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::GradeScaleNotFound);
        }
        Ok(())
    }

    /// Lista las categorías vivas de una materia.
    pub fn list_grade_categories(
        &self,
        subject_id: &str,
    ) -> Result<Vec<GradeCategory>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {CATEGORY_COLUMNS}
             FROM grade_category
             WHERE subject_id = ?1 AND deleted_at IS NULL
             ORDER BY name"
        ))?;
        let rows = statement.query_map(params![subject_id], category_from_row)?;
        let mut categories = Vec::new();
        for row in rows {
            categories.push(row?);
        }
        Ok(categories)
    }

    /// Crea una categoría para una materia.
    pub fn create_grade_category(
        &self,
        subject_id: &str,
        input: &GradeCategoryInput,
    ) -> Result<GradeCategory, StoreError> {
        self.ensure_live_subject(subject_id)?;
        let values = validate_category_input(input)?;
        let scale = self
            .get_grade_scale(&values.scale_id)?
            .ok_or(StoreError::GradeScaleNotFound)?;
        if let Some(scale_subject) = &scale.subject_id
            && scale_subject != subject_id
        {
            return Err(StoreError::InvalidGrade(
                "La escala pertenece a otra materia.",
            ));
        }

        let id = new_id();
        self.connection().execute(
            "INSERT INTO grade_category (id, subject_id, scale_id, name, weight_bp, drop_lowest)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                subject_id,
                values.scale_id,
                values.name,
                values.weight_bp,
                values.drop_lowest,
            ],
        )?;
        self.get_grade_category(&id)?
            .ok_or(StoreError::GradeCategoryNotFound)
    }

    /// Actualiza una categoría viva.
    pub fn update_grade_category(
        &self,
        id: &str,
        input: &GradeCategoryInput,
    ) -> Result<GradeCategory, StoreError> {
        let current = self
            .get_grade_category(id)?
            .ok_or(StoreError::GradeCategoryNotFound)?;
        let values = validate_category_input(input)?;
        let scale = self
            .get_grade_scale(&values.scale_id)?
            .ok_or(StoreError::GradeScaleNotFound)?;
        if let Some(scale_subject) = &scale.subject_id
            && scale_subject != &current.subject_id
        {
            return Err(StoreError::InvalidGrade(
                "La escala pertenece a otra materia.",
            ));
        }

        let changed = self.connection().execute(
            "UPDATE grade_category
             SET scale_id = ?1, name = ?2, weight_bp = ?3, drop_lowest = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?5 AND deleted_at IS NULL",
            params![
                values.scale_id,
                values.name,
                values.weight_bp,
                values.drop_lowest,
                id,
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::GradeCategoryNotFound);
        }
        self.get_grade_category(id)?
            .ok_or(StoreError::GradeCategoryNotFound)
    }

    /// Borra lógicamente una categoría sin entradas vivas.
    pub fn delete_grade_category(&self, id: &str) -> Result<(), StoreError> {
        let entries: i64 = self.connection().query_row(
            "SELECT count(*) FROM grade_entry WHERE category_id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        if entries > 0 {
            return Err(StoreError::Conflict(
                "La categoría tiene notas vivas; borralas o movelas primero.",
            ));
        }

        let changed = self.connection().execute(
            "UPDATE grade_category
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::GradeCategoryNotFound);
        }
        Ok(())
    }

    /// Lista las entradas vivas de una materia.
    pub fn list_grade_entries(&self, subject_id: &str) -> Result<Vec<GradeEntry>, StoreError> {
        let mut statement = self.connection().prepare(&format!(
            "SELECT {ENTRY_COLUMNS}
             FROM grade_entry
             WHERE subject_id = ?1 AND deleted_at IS NULL
             ORDER BY date, created_at"
        ))?;
        let rows = statement.query_map(params![subject_id], entry_from_row)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Crea una entrada de nota.
    pub fn create_grade_entry(
        &self,
        subject_id: &str,
        input: &GradeEntryInput,
    ) -> Result<GradeEntry, StoreError> {
        self.ensure_live_subject(subject_id)?;
        let values = validate_entry_input(input)?;
        self.ensure_entry_category(subject_id, &values.category_id)?;
        self.ensure_entry_exam(subject_id, values.exam_id.as_deref())?;

        let id = new_id();
        self.connection().execute(
            "INSERT INTO grade_entry
               (id, subject_id, category_id, exam_id, title, score_cp, max_cp, weight_bp, date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                subject_id,
                values.category_id,
                values.exam_id,
                values.title,
                values.score_cp,
                values.max_cp,
                values.weight_bp,
                values.date,
            ],
        )?;
        self.get_grade_entry(&id)?
            .ok_or(StoreError::GradeEntryNotFound)
    }

    /// Actualiza una entrada viva.
    pub fn update_grade_entry(
        &self,
        id: &str,
        input: &GradeEntryInput,
    ) -> Result<GradeEntry, StoreError> {
        let current = self
            .get_grade_entry(id)?
            .ok_or(StoreError::GradeEntryNotFound)?;
        let values = validate_entry_input(input)?;
        self.ensure_entry_category(&current.subject_id, &values.category_id)?;
        self.ensure_entry_exam(&current.subject_id, values.exam_id.as_deref())?;

        let changed = self.connection().execute(
            "UPDATE grade_entry
             SET category_id = ?1, exam_id = ?2, title = ?3, score_cp = ?4, max_cp = ?5,
                 weight_bp = ?6, date = ?7,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?8 AND deleted_at IS NULL",
            params![
                values.category_id,
                values.exam_id,
                values.title,
                values.score_cp,
                values.max_cp,
                values.weight_bp,
                values.date,
                id,
            ],
        )?;
        if changed == 0 {
            return Err(StoreError::GradeEntryNotFound);
        }
        self.get_grade_entry(id)?
            .ok_or(StoreError::GradeEntryNotFound)
    }

    /// Borra lógicamente una entrada de nota.
    pub fn delete_grade_entry(&self, id: &str) -> Result<(), StoreError> {
        let changed = self.connection().execute(
            "UPDATE grade_entry
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if changed == 0 {
            return Err(StoreError::GradeEntryNotFound);
        }
        Ok(())
    }

    /// Calcula el promedio final de una materia con su escala preferida.
    pub fn get_subject_grade(&self, subject_id: &str) -> Result<SubjectGrade, StoreError> {
        self.ensure_live_subject(subject_id)?;
        let scale = self.resolve_grade_scale(subject_id)?;
        let categories = self.list_grade_categories(subject_id)?;
        let entries = self.list_grade_entries(subject_id)?;
        Ok(subject_grade(
            subject_id,
            scale.as_ref(),
            &categories,
            &entries,
        ))
    }

    fn resolve_grade_scale(&self, subject_id: &str) -> Result<Option<GradeScale>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!(
                    "SELECT {SCALE_COLUMNS}
                     FROM grade_scale
                     WHERE deleted_at IS NULL
                       AND (subject_id = ?1 OR subject_id IS NULL)
                     ORDER BY CASE WHEN subject_id = ?1 THEN 0 ELSE 1 END,
                              is_default DESC,
                              created_at
                     LIMIT 1"
                ),
                params![subject_id],
                scale_from_row,
            )
            .optional()?)
    }

    fn get_grade_scale(&self, id: &str) -> Result<Option<GradeScale>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!(
                    "SELECT {SCALE_COLUMNS} FROM grade_scale
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                scale_from_row,
            )
            .optional()?)
    }

    fn get_grade_category(&self, id: &str) -> Result<Option<GradeCategory>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!(
                    "SELECT {CATEGORY_COLUMNS} FROM grade_category
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                category_from_row,
            )
            .optional()?)
    }

    fn get_grade_entry(&self, id: &str) -> Result<Option<GradeEntry>, StoreError> {
        Ok(self
            .connection()
            .query_row(
                &format!(
                    "SELECT {ENTRY_COLUMNS} FROM grade_entry
                     WHERE id = ?1 AND deleted_at IS NULL"
                ),
                params![id],
                entry_from_row,
            )
            .optional()?)
    }

    fn clear_default_scales(&self, subject_id: Option<&str>) -> Result<(), StoreError> {
        match subject_id {
            Some(subject_id) => {
                self.connection().execute(
                    "UPDATE grade_scale
                     SET is_default = 0, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE subject_id = ?1 AND deleted_at IS NULL AND is_default = 1",
                    params![subject_id],
                )?;
            }
            None => {
                self.connection().execute(
                    "UPDATE grade_scale
                     SET is_default = 0, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE subject_id IS NULL AND deleted_at IS NULL AND is_default = 1",
                    [],
                )?;
            }
        }
        Ok(())
    }

    fn ensure_entry_category(&self, subject_id: &str, category_id: &str) -> Result<(), StoreError> {
        let category = self
            .get_grade_category(category_id)?
            .ok_or(StoreError::GradeCategoryNotFound)?;
        if category.subject_id != subject_id {
            return Err(StoreError::InvalidGrade(
                "La categoría pertenece a otra materia.",
            ));
        }
        Ok(())
    }

    fn ensure_entry_exam(&self, subject_id: &str, exam_id: Option<&str>) -> Result<(), StoreError> {
        let Some(exam_id) = exam_id else {
            return Ok(());
        };
        let subject: Option<String> = self
            .connection()
            .query_row(
                "SELECT subject_id FROM exam WHERE id = ?1 AND deleted_at IS NULL",
                params![exam_id],
                |row| row.get(0),
            )
            .optional()?;
        let exam_subject = subject.ok_or(StoreError::ExamNotFound)?;
        if exam_subject != subject_id {
            return Err(StoreError::InvalidGrade(
                "El examen pertenece a otra materia.",
            ));
        }
        Ok(())
    }
}

struct ScaleValues {
    subject_id: Option<String>,
    name: String,
    min_cp: i64,
    max_cp: i64,
    pass_cp: i64,
    precision: i64,
    is_default: bool,
}

fn validate_scale_input(input: &GradeScaleInput) -> Result<ScaleValues, StoreError> {
    validate_grade_scale(input.min_cp, input.max_cp, input.pass_cp, input.precision)
        .map_err(StoreError::InvalidGrade)?;
    let name = normalize_name(&input.name).map_err(StoreError::InvalidGrade)?;
    let subject_id = input
        .subject_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Ok(ScaleValues {
        subject_id,
        name,
        min_cp: input.min_cp,
        max_cp: input.max_cp,
        pass_cp: input.pass_cp,
        precision: input.precision,
        is_default: input.is_default,
    })
}

struct CategoryValues {
    scale_id: String,
    name: String,
    weight_bp: i64,
    drop_lowest: i64,
}

fn validate_category_input(input: &GradeCategoryInput) -> Result<CategoryValues, StoreError> {
    let name = normalize_name(&input.name).map_err(StoreError::InvalidGrade)?;
    let weight_bp = validate_basis_points(input.weight_bp).map_err(StoreError::InvalidGrade)?;
    if input.drop_lowest < 0 {
        return Err(StoreError::InvalidGrade(
            "El descarte no puede ser negativo.",
        ));
    }

    Ok(CategoryValues {
        scale_id: input.scale_id.trim().to_string(),
        name,
        weight_bp,
        drop_lowest: input.drop_lowest,
    })
}

struct EntryValues {
    category_id: String,
    exam_id: Option<String>,
    title: String,
    score_cp: i64,
    max_cp: i64,
    weight_bp: Option<i64>,
    date: Option<String>,
}

fn validate_entry_input(input: &GradeEntryInput) -> Result<EntryValues, StoreError> {
    validate_grade_entry(input.score_cp, input.max_cp).map_err(StoreError::InvalidGrade)?;
    let title = validate_exam_title(&input.title).map_err(StoreError::InvalidGrade)?;
    let weight_bp = match input.weight_bp {
        Some(value) => Some(validate_basis_points(value).map_err(StoreError::InvalidGrade)?),
        None => None,
    };
    let date = match input
        .date
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(value) => Some(normalize_date(value).map_err(StoreError::InvalidGrade)?),
        None => None,
    };
    let exam_id = input
        .exam_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    Ok(EntryValues {
        category_id: input.category_id.trim().to_string(),
        exam_id,
        title,
        score_cp: input.score_cp,
        max_cp: input.max_cp,
        weight_bp,
        date,
    })
}

fn scale_from_row(row: &Row<'_>) -> rusqlite::Result<GradeScale> {
    Ok(GradeScale {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        name: row.get(2)?,
        min_cp: row.get(3)?,
        max_cp: row.get(4)?,
        pass_cp: row.get(5)?,
        precision: row.get(6)?,
        is_default: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn category_from_row(row: &Row<'_>) -> rusqlite::Result<GradeCategory> {
    Ok(GradeCategory {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        scale_id: row.get(2)?,
        name: row.get(3)?,
        weight_bp: row.get(4)?,
        drop_lowest: row.get::<_, i64>(5)?.max(0) as u32,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn entry_from_row(row: &Row<'_>) -> rusqlite::Result<GradeEntry> {
    Ok(GradeEntry {
        id: row.get(0)?,
        subject_id: row.get(1)?,
        category_id: row.get(2)?,
        exam_id: row.get(3)?,
        title: row.get(4)?,
        score_cp: row.get(5)?,
        max_cp: row.get(6)?,
        weight_bp: row.get(7)?,
        date: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::migrated_store_with_subject;

    fn scale_input(subject_id: &str, is_default: bool) -> GradeScaleInput {
        GradeScaleInput {
            subject_id: Some(subject_id.to_string()),
            name: "Escala 0–100".to_string(),
            min_cp: 0,
            max_cp: 10_000,
            pass_cp: 6_000,
            precision: 2,
            is_default,
        }
    }

    #[test]
    fn builds_a_weighted_subject_grade() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let scale = store
            .create_grade_scale(&scale_input(&subject_id, true))
            .unwrap();
        let category = store
            .create_grade_category(
                &subject_id,
                &GradeCategoryInput {
                    scale_id: scale.id.clone(),
                    name: "Parciales".to_string(),
                    weight_bp: 10_000,
                    drop_lowest: 1,
                },
            )
            .unwrap();

        for (title, score) in [
            ("Parcial 1", 4_000),
            ("Parcial 2", 8_000),
            ("Parcial 3", 9_000),
        ] {
            store
                .create_grade_entry(
                    &subject_id,
                    &GradeEntryInput {
                        category_id: category.id.clone(),
                        title: title.to_string(),
                        score_cp: score,
                        max_cp: 10_000,
                        ..GradeEntryInput::default()
                    },
                )
                .unwrap();
        }

        let grade = store.get_subject_grade(&subject_id).unwrap();
        assert_eq!(grade.final_bp, Some(8_500));
        assert_eq!(grade.final_cp, Some(8_500));
        assert_eq!(grade.passed, Some(true));
        assert_eq!(grade.categories[0].dropped, 1);
    }

    #[test]
    fn keeps_only_one_default_scale_per_scope() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let first = store
            .create_grade_scale(&scale_input(&subject_id, true))
            .unwrap();
        let second = store
            .create_grade_scale(&GradeScaleInput {
                name: "Escala 0–20".to_string(),
                min_cp: 0,
                max_cp: 2_000,
                pass_cp: 1_200,
                ..scale_input(&subject_id, true)
            })
            .unwrap();

        let scales = store.list_grade_scales(Some(&subject_id)).unwrap();
        let default = scales.iter().find(|scale| scale.is_default).unwrap();
        assert_eq!(default.id, second.id);
        assert!(
            !scales
                .iter()
                .find(|scale| scale.id == first.id)
                .unwrap()
                .is_default
        );
    }

    #[test]
    fn rejects_categories_from_another_subject() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let scale = store
            .create_grade_scale(&scale_input(&subject_id, true))
            .unwrap();
        let category = store
            .create_grade_category(
                &subject_id,
                &GradeCategoryInput {
                    scale_id: scale.id.clone(),
                    name: "Parciales".to_string(),
                    weight_bp: 5_000,
                    drop_lowest: 0,
                },
            )
            .unwrap();

        let error = store
            .create_grade_entry(
                "otra-materia",
                &GradeEntryInput {
                    category_id: category.id.clone(),
                    title: "Parcial".to_string(),
                    score_cp: 5_000,
                    max_cp: 10_000,
                    ..GradeEntryInput::default()
                },
            )
            .unwrap_err();
        assert!(matches!(error, StoreError::SubjectNotFound));
    }

    #[test]
    fn deletes_entries_and_categories_logically() {
        let (store, subject_id, _) = migrated_store_with_subject();
        let scale = store
            .create_grade_scale(&scale_input(&subject_id, true))
            .unwrap();
        let category = store
            .create_grade_category(
                &subject_id,
                &GradeCategoryInput {
                    scale_id: scale.id.clone(),
                    name: "Parciales".to_string(),
                    weight_bp: 5_000,
                    drop_lowest: 0,
                },
            )
            .unwrap();
        let entry = store
            .create_grade_entry(
                &subject_id,
                &GradeEntryInput {
                    category_id: category.id.clone(),
                    title: "Parcial".to_string(),
                    score_cp: 5_000,
                    max_cp: 10_000,
                    ..GradeEntryInput::default()
                },
            )
            .unwrap();

        let error = store.delete_grade_category(&category.id).unwrap_err();
        assert!(matches!(error, StoreError::Conflict(_)));

        store.delete_grade_entry(&entry.id).unwrap();
        store.delete_grade_category(&category.id).unwrap();
        store.delete_grade_scale(&scale.id).unwrap();
        assert!(store.list_grade_categories(&subject_id).unwrap().is_empty());
    }
}
