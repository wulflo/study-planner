//! Formato de exportación total de Study Planner.
//!
//! El archivo es texto UTF-8 canónico: cabecera, cuerpo JSON compacto con una
//! fila por línea y terminador. El cuerpo se serializa siempre en el mismo
//! orden y con las mismas claves para que `exportar → importar → exportar`
//! produzca bytes idénticos dentro de la misma versión de formato y esquema.

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, params, params_from_iter};
use serde_json::Value;
use sha2::{Digest, Sha256};
use study_planner_core::{APP_VERSION, SCHEMA_VERSION};
use study_planner_store::{Store, StoreError};
use thiserror::Error;

/// Primera línea obligatoria del archivo de exportación.
pub const EXPORT_MAGIC: &str = "STUDY-PLANNER-EXPORT";

/// Última línea obligatoria del archivo de exportación.
pub const EXPORT_END: &str = "STUDY-PLANNER-END";

/// Versión del formato de archivo. Es independiente de `schema_version`.
pub const EXPORT_FORMAT_VERSION_VALUE: u32 = study_planner_core::EXPORT_FORMAT_VERSION;

/// Tamaño máximo aceptado para un archivo de importación.
pub const MAX_EXPORT_BYTES: u64 = 512 * 1024 * 1024;

/// Orden fijo de tablas del cuerpo. `attachment_blob` es una tabla lógica.
pub const TABLE_ORDER: &[&str] = &[
    "academic_year",
    "term",
    "holiday",
    "rotation_config",
    "subject",
    "class_slot",
    "class_exception",
    "attendance_record",
    "exam",
    "topic",
    "exam_topic",
    "grade_scale",
    "grade_category",
    "grade_entry",
    "task",
    "tag",
    "task_tag",
    "saved_filter",
    "self_assessment",
    "strength_override",
    "review_item",
    "study_session",
    "note",
    "attachment",
    "attachment_blob",
    "setting",
];

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("formato de exportación inválido: {0}")]
    InvalidFormat(String),
    #[error("checksum inválido: esperado {expected}, obtenido {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("json inválido: {0}")]
    Json(#[from] serde_json::Error),
    #[error("error de archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("error de la bóveda: {0}")]
    Store(#[from] StoreError),
    #[error("error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("base64 inválido: {0}")]
    Base64(String),
    #[error("la bóveda no está vacía; elegí una bóveda vacía para importar")]
    VaultNotEmpty,
}

/// Cabecera validada de un archivo de exportación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportHeader {
    pub format_version: u32,
    pub schema_version: u32,
    pub app_version: String,
    pub vault_id: String,
    pub snapshot_at: String,
    pub attachments: String,
    pub body_sha256: String,
}

/// Una fila exportada. `line` es la línea del archivo al importar; al exportar
/// se deja en cero.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportRow {
    pub table: String,
    pub row: BTreeMap<String, Value>,
    pub line: usize,
}

/// Documento listo para serializar o recién parseado.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDocument {
    pub header: ExportHeader,
    pub rows: Vec<ExportRow>,
}

/// Resultado de escribir un archivo de exportación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub rows: usize,
    pub snapshot_at: String,
}

/// Resultado de importar un archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub rows: usize,
    pub attachments: usize,
    pub vault_id: String,
    pub snapshot_at: String,
}

/// Modo de importación. La Fase 1 acepta bóvedas vacías; el reemplazo total
/// exige que la capa de bóveda haga un backup previo y pida confirmación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    EmptyOnly,
    Replace,
}

/// Exporta la bóveda a un `String` canónico.
pub fn export_to_text(store: &Store, attachments_dir: &Path) -> Result<String, TransferError> {
    let document = collect_export(store, attachments_dir)?;
    export_text(&document)
}

/// Exporta la bóveda a `destination` con escritura atómica y permisos `600`.
pub fn export_to_path(
    store: &Store,
    attachments_dir: &Path,
    destination: &Path,
) -> Result<ExportReport, TransferError> {
    let document = collect_export(store, attachments_dir)?;
    let text = export_text(&document)?;

    write_atomic(destination, text.as_bytes())?;

    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| TransferError::InvalidFormat("nombre de archivo inválido".to_string()))?
        .to_string();

    Ok(ExportReport {
        file_name,
        sha256: sha256_hex(text.as_bytes()),
        size_bytes: text.len() as u64,
        rows: document.rows.len(),
        snapshot_at: document.header.snapshot_at,
    })
}

/// Serializa un documento al formato canónico. Recalcula el checksum del
/// cuerpo, así que también sirve para tests que modifican filas.
pub fn export_text(document: &ExportDocument) -> Result<String, TransferError> {
    let body = serialize_body(&document.rows)?;
    let body_sha256 = sha256_hex(body.as_bytes());
    let header = &document.header;

    let mut text = String::with_capacity(body.len() + 512);
    text.push_str(EXPORT_MAGIC);
    text.push('\n');
    text.push_str(&format!("format-version: {}\n", header.format_version));
    text.push_str(&format!("schema-version: {}\n", header.schema_version));
    text.push_str(&format!("app-version: {}\n", header.app_version));
    text.push_str(&format!("vault-id: {}\n", header.vault_id));
    text.push_str(&format!("snapshot-at: {}\n", header.snapshot_at));
    text.push_str(&format!("attachments: {}\n", header.attachments));
    text.push_str(&format!("body-sha256: {body_sha256}\n"));
    text.push_str("---\n");
    text.push_str(&body);
    text.push_str(EXPORT_END);
    text.push('\n');

    Ok(text)
}

/// Lee una bóveda completa dentro de una transacción de lectura y arma el
/// documento canónico. Los adjuntos se incorporan como `attachment_blob`.
pub fn collect_export(
    store: &Store,
    attachments_dir: &Path,
) -> Result<ExportDocument, TransferError> {
    let tx = store.connection().unchecked_transaction()?;
    let mut rows = Vec::new();
    let mut snapshot_at: Option<String> = None;

    for table in TABLE_ORDER {
        if *table == "attachment_blob" {
            collect_attachment_blobs(&tx, attachments_dir, &mut rows)?;
            continue;
        }

        let order = if *table == "setting" {
            " ORDER BY key"
        } else {
            " ORDER BY id"
        };
        let filter = if *table == "setting" {
            " WHERE scope = 'portable'"
        } else {
            ""
        };
        let sql = format!("SELECT * FROM \"{table}\"{filter}{order}");

        let mut statement = tx.prepare(&sql)?;
        let original_columns: Vec<String> = statement.column_names().map(str::to_string).collect();
        let mut columns: Vec<(String, usize)> = original_columns
            .into_iter()
            .enumerate()
            .map(|(index, name)| (name, index))
            .collect();
        columns.sort_by(|left, right| left.0.cmp(&right.0));

        let mut query = statement.query([])?;
        while let Some(row) = query.next()? {
            let mut values = BTreeMap::new();
            for (column, index) in &columns {
                let value = value_ref_to_json(row.get_ref(*index)?).map_err(|error| {
                    TransferError::InvalidFormat(format!("columna {table}.{column}: {error}"))
                })?;
                if column == "updated_at"
                    && let Value::String(updated_at) = &value
                    && snapshot_at
                        .as_ref()
                        .is_none_or(|current| updated_at > current)
                {
                    snapshot_at = Some(updated_at.clone());
                }
                values.insert(column.clone(), value);
            }
            rows.push(ExportRow {
                table: (*table).to_string(),
                row: values,
                line: 0,
            });
        }
    }

    tx.commit()?;

    Ok(ExportDocument {
        header: ExportHeader {
            format_version: EXPORT_FORMAT_VERSION_VALUE,
            schema_version: SCHEMA_VERSION,
            app_version: APP_VERSION.to_string(),
            vault_id: store.vault_id()?,
            snapshot_at: snapshot_at.unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string()),
            attachments: "embedded".to_string(),
            body_sha256: String::new(),
        },
        rows,
    })
}

fn collect_attachment_blobs(
    conn: &Connection,
    attachments_dir: &Path,
    rows: &mut Vec<ExportRow>,
) -> Result<(), TransferError> {
    let mut statement = conn.prepare("SELECT id, sha256 FROM attachment ORDER BY id")?;
    let mut query = statement.query([])?;

    while let Some(row) = query.next()? {
        let attachment_id: String = row.get(0)?;
        let sha256: String = row.get(1)?;
        if !is_hex_sha256(&sha256) {
            return Err(TransferError::InvalidFormat(format!(
                "el adjunto {attachment_id} tiene un SHA-256 inválido"
            )));
        }

        let path = attachments_dir.join(&sha256);
        let bytes = fs::read(&path).map_err(|error| {
            TransferError::InvalidFormat(format!(
                "no se pudo leer el adjunto {sha256} de la bóveda: {error}"
            ))
        })?;
        if sha256_hex(&bytes) != sha256 {
            return Err(TransferError::InvalidFormat(format!(
                "el adjunto {sha256} no coincide con su checksum"
            )));
        }

        let mut values = BTreeMap::new();
        values.insert("attachment_id".to_string(), Value::String(attachment_id));
        values.insert(
            "data_base64".to_string(),
            Value::String(BASE64.encode(bytes)),
        );
        values.insert("sha256".to_string(), Value::String(sha256));
        rows.push(ExportRow {
            table: "attachment_blob".to_string(),
            row: values,
            line: 0,
        });
    }

    Ok(())
}

fn serialize_body(rows: &[ExportRow]) -> Result<String, TransferError> {
    let mut body = String::new();

    for row in rows {
        let table = serde_json::to_string(&row.table)?;
        let values = serde_json::to_string(&row.row)?;
        body.push_str(&format!("{{\"table\":{table},\"row\":{values}}}\n"));
    }

    Ok(body)
}

/// Parsea y valida un archivo de exportación completo.
pub fn parse_export(text: &str) -> Result<ExportDocument, TransferError> {
    if text.starts_with('\u{feff}') {
        return Err(TransferError::InvalidFormat(
            "el archivo no puede empezar con BOM".to_string(),
        ));
    }
    if text.contains('\r') {
        return Err(TransferError::InvalidFormat(
            "el archivo debe usar saltos de línea LF, no CRLF".to_string(),
        ));
    }
    if !text.ends_with('\n') {
        return Err(TransferError::InvalidFormat(
            "el archivo debe terminar con un salto de línea".to_string(),
        ));
    }

    let content = &text[..text.len() - 1];
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() < 10 {
        return Err(TransferError::InvalidFormat(
            "el archivo es demasiado corto para ser una exportación válida".to_string(),
        ));
    }
    if lines[0] != EXPORT_MAGIC {
        return Err(TransferError::InvalidFormat(format!(
            "se esperaba la primera línea {EXPORT_MAGIC}"
        )));
    }
    if lines[8] != "---" {
        return Err(TransferError::InvalidFormat(
            "falta el separador --- entre cabecera y cuerpo".to_string(),
        ));
    }
    if lines[lines.len() - 1] != EXPORT_END {
        return Err(TransferError::InvalidFormat(format!(
            "se esperaba la última línea {EXPORT_END}"
        )));
    }

    let format_version = parse_header_u32(&lines, 1, "format-version")?;
    if format_version != EXPORT_FORMAT_VERSION_VALUE {
        return Err(TransferError::InvalidFormat(format!(
            "versión de formato no soportada: {format_version}"
        )));
    }

    let schema_version = parse_header_u32(&lines, 2, "schema-version")?;
    if schema_version != SCHEMA_VERSION {
        return Err(TransferError::InvalidFormat(format!(
            "versión de esquema no soportada: {schema_version}; se esperaba {SCHEMA_VERSION}"
        )));
    }

    let app_version = parse_header_value(&lines, 3, "app-version")?.to_string();
    let vault_id = parse_header_value(&lines, 4, "vault-id")?.to_string();
    let snapshot_at = parse_header_value(&lines, 5, "snapshot-at")?.to_string();
    let attachments = parse_header_value(&lines, 6, "attachments")?.to_string();
    if attachments != "embedded" {
        return Err(TransferError::InvalidFormat(
            "solo se admite attachments: embedded en v1".to_string(),
        ));
    }
    let body_sha256 = parse_header_value(&lines, 7, "body-sha256")?.to_string();
    if !is_hex_sha256(&body_sha256) {
        return Err(TransferError::InvalidFormat(
            "body-sha256 debe ser un hash SHA-256 en hex minúscula".to_string(),
        ));
    }
    if vault_id.is_empty() || snapshot_at.is_empty() || app_version.is_empty() {
        return Err(TransferError::InvalidFormat(
            "la cabecera tiene campos obligatorios vacíos".to_string(),
        ));
    }

    let body_start: usize = lines[..9].iter().map(|line| line.len() + 1).sum();
    let body_end = text.len() - (EXPORT_END.len() + 1);
    let body = &text[body_start..body_end];
    let actual_sha256 = sha256_hex(body.as_bytes());
    if !constant_time_eq(actual_sha256.as_bytes(), body_sha256.as_bytes()) {
        return Err(TransferError::ChecksumMismatch {
            expected: body_sha256,
            actual: actual_sha256,
        });
    }

    let mut rows = Vec::new();
    let mut previous_table: Option<usize> = None;
    let mut previous_key: Option<String> = None;

    if !body.is_empty() {
        for (index, line) in body.split('\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let line_number = 10 + index;
            let value: Value = serde_json::from_str(line).map_err(|error| {
                TransferError::InvalidFormat(format!("línea {line_number}: JSON inválido: {error}"))
            })?;
            let Value::Object(mut object) = value else {
                return Err(TransferError::InvalidFormat(format!(
                    "línea {line_number}: se esperaba un objeto JSON"
                )));
            };
            let keys: Vec<&String> = object.keys().collect();
            if keys.len() != 2 || keys[0] != "table" || keys[1] != "row" {
                return Err(TransferError::InvalidFormat(format!(
                    "línea {line_number}: se esperaban exactamente las claves table y row"
                )));
            }
            let table = object
                .get("table")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {line_number}: table debe ser un string"
                    ))
                })?
                .to_string();
            let table_index = TABLE_ORDER
                .iter()
                .position(|candidate| *candidate == table)
                .ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {line_number}: tabla desconocida {table}"
                    ))
                })?;

            let row_value = object.remove("row").ok_or_else(|| {
                TransferError::InvalidFormat(format!("línea {line_number}: falta row"))
            })?;
            let Value::Object(row_object) = row_value else {
                return Err(TransferError::InvalidFormat(format!(
                    "línea {line_number}: row debe ser un objeto JSON"
                )));
            };

            let mut previous_column: Option<&String> = None;
            for column in row_object.keys() {
                if let Some(previous) = previous_column
                    && previous >= column
                {
                    return Err(TransferError::InvalidFormat(format!(
                        "línea {line_number}: las columnas de row deben estar ordenadas"
                    )));
                }
                previous_column = Some(column);
            }

            if let Some(last_table) = previous_table {
                if table_index < last_table {
                    return Err(TransferError::InvalidFormat(format!(
                        "línea {line_number}: la tabla {table} aparece fuera del orden canónico"
                    )));
                }
                if table_index > last_table {
                    previous_key = None;
                }
            }
            previous_table = Some(table_index);

            let key_name = key_column(&table);
            let key = row_object
                .get(key_name)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {line_number}: falta la clave {key_name} en {table}"
                    ))
                })?
                .to_string();
            if let Some(last_key) = &previous_key
                && key <= *last_key
            {
                return Err(TransferError::InvalidFormat(format!(
                    "línea {line_number}: las filas de {table} deben estar ordenadas por {key_name}"
                )));
            }
            previous_key = Some(key);

            let row: BTreeMap<String, Value> = row_object.into_iter().collect();
            rows.push(ExportRow {
                table,
                row,
                line: line_number,
            });
        }
    }

    Ok(ExportDocument {
        header: ExportHeader {
            format_version,
            schema_version,
            app_version,
            vault_id,
            snapshot_at,
            attachments,
            body_sha256: actual_sha256,
        },
        rows,
    })
}

/// Lee un archivo, valida el formato y lo importa en la bóveda.
pub fn import_from_path(
    store: &Store,
    attachments_dir: &Path,
    path: &Path,
    mode: ImportMode,
) -> Result<ImportReport, TransferError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_EXPORT_BYTES {
        return Err(TransferError::InvalidFormat(format!(
            "el archivo supera el máximo permitido de {MAX_EXPORT_BYTES} bytes"
        )));
    }

    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes)
        .map_err(|_| TransferError::InvalidFormat("el archivo no es UTF-8 válido".to_string()))?;
    import_from_text(store, attachments_dir, &text, mode)
}

/// Importa un archivo ya leído. Todo ocurre en una transacción: si falla algo,
/// la bóveda queda como estaba.
pub fn import_from_text(
    store: &Store,
    attachments_dir: &Path,
    text: &str,
    mode: ImportMode,
) -> Result<ImportReport, TransferError> {
    let document = parse_export(text)?;
    import_document(store, attachments_dir, &document, mode)
}

/// Aplica un documento validado a la bóveda.
pub fn import_document(
    store: &Store,
    attachments_dir: &Path,
    document: &ExportDocument,
    mode: ImportMode,
) -> Result<ImportReport, TransferError> {
    let blob_rows = validate_attachment_consistency(&document.rows)?;
    let tx = store.connection().unchecked_transaction()?;
    // Aplaza las comprobaciones de FK al final de la transacción para poder
    // informar todas las violaciones juntas antes del commit.
    tx.execute("PRAGMA defer_foreign_keys = ON", [])?;

    if mode == ImportMode::EmptyOnly && !is_vault_empty(&tx)? {
        return Err(TransferError::VaultNotEmpty);
    }
    if mode == ImportMode::Replace {
        clear_vault(&tx)?;
    }

    // Los settings portables seed se reemplazan por los del archivo. Los
    // settings machine de esta máquina no se tocan.
    tx.execute("DELETE FROM setting WHERE scope = 'portable'", [])?;
    tx.execute("DELETE FROM task_fts", [])?;
    tx.execute("DELETE FROM note_fts", [])?;

    let mut columns_by_table: HashMap<String, Vec<String>> = HashMap::new();
    let mut db_rows = 0usize;

    for row in &document.rows {
        if row.table == "attachment_blob" {
            continue;
        }

        let columns = match columns_by_table.get(&row.table) {
            Some(columns) => columns.clone(),
            None => {
                let columns = table_columns(&tx, &row.table)?;
                columns_by_table.insert(row.table.clone(), columns.clone());
                columns
            }
        };

        let actual: Vec<&String> = row.row.keys().collect();
        let expected: Vec<&String> = columns.iter().collect();
        if actual != expected {
            return Err(TransferError::InvalidFormat(format!(
                "línea {}: las columnas de {} no coinciden con el esquema",
                row.line, row.table
            )));
        }

        if row.table == "setting" {
            let scope = row.row.get("scope").and_then(Value::as_str);
            if scope != Some("portable") {
                return Err(TransferError::InvalidFormat(format!(
                    "línea {}: solo se importan settings portables",
                    row.line
                )));
            }
        }

        let placeholders = (1..=columns.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let quoted_columns = columns
            .iter()
            .map(|column| format!("\"{column}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO \"{}\" ({quoted_columns}) VALUES ({placeholders})",
            row.table
        );

        let values = columns
            .iter()
            .map(|column| {
                row.row
                    .get(column)
                    .ok_or_else(|| {
                        TransferError::InvalidFormat(format!(
                            "línea {}: falta la columna {column} en {}",
                            row.line, row.table
                        ))
                    })
                    .and_then(|value| json_to_sql(value, row.line))
            })
            .collect::<Result<Vec<_>, _>>()?;

        tx.execute(&sql, params_from_iter(values.iter()))
            .map_err(|error| {
                TransferError::InvalidFormat(format!(
                    "línea {}: no se pudo insertar en {}: {error}",
                    row.line, row.table
                ))
            })?;
        db_rows += 1;
    }

    // Los blobs se escriben después de validar las filas. Si algo falla antes
    // del commit, se eliminan los archivos nuevos y la transacción se revierte.
    let mut created_blobs = Vec::new();
    let result = (|| -> Result<(), TransferError> {
        fs::create_dir_all(attachments_dir)?;
        restrict_directory(attachments_dir);

        for blob in &blob_rows {
            let bytes = BASE64
                .decode(&blob.data_base64)
                .map_err(|error| TransferError::Base64(format!("línea {}: {error}", blob.line)))?;
            let actual = sha256_hex(&bytes);
            if actual != blob.sha256 {
                return Err(TransferError::InvalidFormat(format!(
                    "línea {}: el adjunto no coincide con su SHA-256",
                    blob.line
                )));
            }

            let destination = attachments_dir.join(&blob.sha256);
            if destination.exists() {
                let existing = fs::read(&destination)?;
                if sha256_hex(&existing) != blob.sha256 {
                    return Err(TransferError::InvalidFormat(format!(
                        "el adjunto existente {} está corrupto",
                        blob.sha256
                    )));
                }
                continue;
            }

            write_atomic(&destination, &bytes)?;
            created_blobs.push(destination);
        }

        rebuild_fts(&tx)?;
        set_imported_meta(&tx, &document.header)?;
        assert_database_healthy(&tx)?;
        Ok(())
    })();

    if let Err(error) = result {
        for path in created_blobs {
            let _ = fs::remove_file(path);
        }
        return Err(error);
    }

    tx.commit()?;

    Ok(ImportReport {
        rows: db_rows,
        attachments: blob_rows.len(),
        vault_id: document.header.vault_id.clone(),
        snapshot_at: document.header.snapshot_at.clone(),
    })
}

/// Indica si la bóveda no tiene datos de dominio (solo settings técnicos).
pub fn is_empty_vault(store: &Store) -> Result<bool, TransferError> {
    is_vault_empty(store.connection())
}

fn is_vault_empty(conn: &Connection) -> Result<bool, TransferError> {
    for table in TABLE_ORDER {
        if *table == "attachment_blob" || *table == "setting" {
            continue;
        }
        let count: i64 =
            conn.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| {
                row.get(0)
            })?;
        if count != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

fn clear_vault(conn: &Connection) -> Result<(), TransferError> {
    for table in TABLE_ORDER.iter().rev() {
        if *table == "attachment_blob" || *table == "setting" {
            continue;
        }
        conn.execute(&format!("DELETE FROM \"{table}\""), [])?;
    }
    conn.execute("DELETE FROM setting WHERE scope = 'portable'", [])?;
    conn.execute("DELETE FROM task_fts", [])?;
    conn.execute("DELETE FROM note_fts", [])?;
    Ok(())
}

fn rebuild_fts(conn: &Connection) -> Result<(), TransferError> {
    conn.execute(
        "INSERT INTO task_fts(task_id, title, description, tags)
         SELECT t.id, t.title, COALESCE(t.description, ''),
                COALESCE((
                  SELECT group_concat(g.name, ' ')
                  FROM task_tag tt
                  JOIN tag g ON g.id = tt.tag_id
                  WHERE tt.task_id = t.id
                    AND tt.deleted_at IS NULL
                    AND g.deleted_at IS NULL
                ), '')
         FROM task t
         WHERE t.deleted_at IS NULL",
        [],
    )?;
    conn.execute(
        "INSERT INTO note_fts(note_id, title, body)
         SELECT id, title, body FROM note WHERE deleted_at IS NULL",
        [],
    )?;
    Ok(())
}

fn set_imported_meta(conn: &Connection, header: &ExportHeader) -> Result<(), TransferError> {
    conn.execute(
        "INSERT INTO app_meta(key, value) VALUES ('vault_id', ?1)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![header.vault_id],
    )?;
    conn.execute(
        "INSERT INTO app_meta(key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        params![header.schema_version.to_string()],
    )?;
    Ok(())
}

fn assert_database_healthy(conn: &Connection) -> Result<(), TransferError> {
    let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
    let mut rows = statement.query([])?;
    let mut violations = 0usize;
    while rows.next()?.is_some() {
        violations += 1;
    }
    drop(rows);
    drop(statement);

    if violations != 0 {
        return Err(TransferError::InvalidFormat(format!(
            "la importación deja {violations} violaciones de claves foráneas"
        )));
    }

    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(TransferError::InvalidFormat(format!(
            "integrity_check devolvió «{integrity}»"
        )));
    }
    Ok(())
}

#[derive(Debug)]
struct BlobRow {
    line: usize,
    sha256: String,
    data_base64: String,
}

fn validate_attachment_consistency(rows: &[ExportRow]) -> Result<Vec<BlobRow>, TransferError> {
    let mut attachments: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut blobs: BTreeMap<String, BlobRow> = BTreeMap::new();

    for row in rows {
        match row.table.as_str() {
            "attachment" => {
                let id = row.row.get("id").and_then(Value::as_str).ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {}: falta id de attachment",
                        row.line
                    ))
                })?;
                let sha256 = row
                    .row
                    .get("sha256")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        TransferError::InvalidFormat(format!(
                            "línea {}: falta sha256 de attachment",
                            row.line
                        ))
                    })?;
                if !is_hex_sha256(sha256) {
                    return Err(TransferError::InvalidFormat(format!(
                        "línea {}: sha256 inválido en attachment",
                        row.line
                    )));
                }
                attachments.insert(id.to_string(), (sha256.to_string(), row.line));
            }
            "attachment_blob" => {
                let columns: Vec<&String> = row.row.keys().collect();
                let expected: Vec<&str> = vec!["attachment_id", "data_base64", "sha256"];
                if columns != expected {
                    return Err(TransferError::InvalidFormat(format!(
                        "línea {}: columnas inválidas en attachment_blob",
                        row.line
                    )));
                }
                let attachment_id = row.row["attachment_id"].as_str().ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {}: attachment_id debe ser string",
                        row.line
                    ))
                })?;
                let sha256 = row.row["sha256"].as_str().ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {}: sha256 debe ser string",
                        row.line
                    ))
                })?;
                let data_base64 = row.row["data_base64"].as_str().ok_or_else(|| {
                    TransferError::InvalidFormat(format!(
                        "línea {}: data_base64 debe ser string",
                        row.line
                    ))
                })?;
                if !is_hex_sha256(sha256) {
                    return Err(TransferError::InvalidFormat(format!(
                        "línea {}: sha256 inválido en attachment_blob",
                        row.line
                    )));
                }
                blobs.insert(
                    attachment_id.to_string(),
                    BlobRow {
                        line: row.line,
                        sha256: sha256.to_string(),
                        data_base64: data_base64.to_string(),
                    },
                );
            }
            _ => {}
        }
    }

    for (id, (sha256, line)) in &attachments {
        let blob = blobs.get(id).ok_or_else(|| {
            TransferError::InvalidFormat(format!("línea {line}: falta el blob del adjunto {id}"))
        })?;
        if &blob.sha256 != sha256 {
            return Err(TransferError::InvalidFormat(format!(
                "línea {}: el blob del adjunto {id} no coincide con attachment.sha256",
                blob.line
            )));
        }
    }
    for (id, blob) in &blobs {
        if !attachments.contains_key(id) {
            return Err(TransferError::InvalidFormat(format!(
                "línea {}: blob sin fila attachment para {id}",
                blob.line
            )));
        }
    }

    Ok(blobs.into_values().collect())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, TransferError> {
    if !TABLE_ORDER.contains(&table) {
        return Err(TransferError::InvalidFormat(format!(
            "tabla desconocida: {table}"
        )));
    }
    let sql = format!("SELECT * FROM \"{table}\" LIMIT 0");
    let statement = conn.prepare(&sql)?;
    let mut columns: Vec<String> = statement.column_names().map(str::to_string).collect();
    columns.sort();
    Ok(columns)
}

fn json_to_sql(value: &Value, line: usize) -> Result<SqlValue, TransferError> {
    match value {
        Value::Null => Ok(SqlValue::Null),
        Value::String(value) => Ok(SqlValue::Text(value.clone())),
        Value::Number(number) => number.as_i64().map(SqlValue::Integer).ok_or_else(|| {
            TransferError::InvalidFormat(format!(
                "línea {line}: solo se admiten enteros de 64 bits"
            ))
        }),
        _ => Err(TransferError::InvalidFormat(format!(
            "línea {line}: solo se admiten strings, enteros o null"
        ))),
    }
}

fn key_column(table: &str) -> &'static str {
    match table {
        "setting" => "key",
        "attachment_blob" => "attachment_id",
        _ => "id",
    }
}

fn parse_header_value<'a>(
    lines: &'a [&'a str],
    index: usize,
    key: &str,
) -> Result<&'a str, TransferError> {
    let line = lines.get(index).ok_or_else(|| {
        TransferError::InvalidFormat(format!("falta el campo {key} en la cabecera"))
    })?;
    let prefix = format!("{key}: ");
    let value = line.strip_prefix(&prefix).ok_or_else(|| {
        TransferError::InvalidFormat(format!("línea {}: se esperaba el campo {key}", index + 1))
    })?;
    if value.is_empty() || value.ends_with(' ') {
        return Err(TransferError::InvalidFormat(format!(
            "línea {}: valor inválido para {key}",
            index + 1
        )));
    }
    Ok(value)
}

fn parse_header_u32(lines: &[&str], index: usize, key: &str) -> Result<u32, TransferError> {
    let value = parse_header_value(lines, index, key)?;
    value.parse::<u32>().map_err(|_| {
        TransferError::InvalidFormat(format!("línea {}: {key} debe ser un entero", index + 1))
    })
}

fn value_ref_to_json(value: ValueRef<'_>) -> Result<Value, &'static str> {
    match value {
        ValueRef::Null => Ok(Value::Null),
        ValueRef::Integer(number) => Ok(Value::from(number)),
        ValueRef::Real(_) => Err("no se admiten números reales"),
        ValueRef::Text(bytes) => std::str::from_utf8(bytes)
            .map(|text| Value::String(text.to_string()))
            .map_err(|_| "el texto no es UTF-8 válido"),
        ValueRef::Blob(_) => Err("no se admiten blobs en las tablas de dominio"),
    }
}

fn write_atomic(destination: &Path, bytes: &[u8]) -> Result<(), TransferError> {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| TransferError::InvalidFormat("nombre de archivo inválido".to_string()))?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp = destination.with_file_name(format!(".{file_name}.partial"));
    let _ = fs::remove_file(&temp);

    let result = (|| -> Result<(), TransferError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        restrict_file(&temp);
        fs::rename(&temp, destination)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result?;

    if let Some(parent) = destination.parent() {
        let _ = File::open(parent).and_then(|directory| directory.sync_all());
    }
    Ok(())
}

fn restrict_directory(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn restrict_file(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn migrated_store() -> Store {
        let mut store = Store::open_in_memory().expect("abrir memoria");
        store.migrate().expect("migrar");
        store
    }

    fn seeded_store() -> Store {
        let store = migrated_store();
        store.create_task("Tarea de prueba").expect("crear tarea");
        store
            .connection()
            .execute(
                "INSERT INTO setting(key, value_json, scope)
                 VALUES ('test.portable', 'true', 'portable')",
                [],
            )
            .expect("insertar setting portable");
        store
    }

    fn temp_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis());
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "study-planner-transfer-{tag}-{}-{millis}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("crear directorio temporal");
        path
    }

    fn fixture_with_all_tables() -> (Store, PathBuf, Vec<u8>) {
        let store = migrated_store();
        let attachments = temp_dir("attachments");
        let blob = b"contenido del adjunto".to_vec();
        let blob_sha = sha256_hex(&blob);
        fs::write(attachments.join(&blob_sha), &blob).expect("escribir adjunto");

        let conn = store.connection();
        conn.execute_batch(&format!(
            r#"
            INSERT INTO academic_year(id, name, start_date, end_date)
              VALUES ('year-1', '2026', '2026-03-01', '2026-12-20');
            INSERT INTO term(id, year_id, name, start_date, end_date)
              VALUES ('term-1', 'year-1', 'Primer semestre', '2026-03-01', '2026-07-01');
            INSERT INTO holiday(id, term_id, date, name)
              VALUES ('holiday-1', 'term-1', '2026-05-01', 'Feriado');
            INSERT INTO rotation_config(id, term_id, mode, anchor_date, labels_json)
              VALUES ('rotation-1', 'term-1', 'weekly_ab', '2026-03-02', '["A","B"]');
            INSERT INTO subject(id, term_id, name, code)
              VALUES ('subject-1', 'term-1', 'Álgebra', 'MAT-101');
            INSERT INTO class_slot(id, subject_id, weekday, start_time, end_time)
              VALUES ('slot-1', 'subject-1', 1, '08:00', '09:30');
            INSERT INTO class_exception(id, class_slot_id, date, kind)
              VALUES ('exception-1', 'slot-1', '2026-05-04', 'moved');
            INSERT INTO attendance_record(id, class_slot_id, date, status)
              VALUES ('attendance-1', 'slot-1', '2026-05-11', 'present');
            INSERT INTO exam(id, subject_id, term_id, title, date, start_time, end_time, room, weight_bp, result_score_cp, result_max_cp, result_status, notes)
              VALUES ('exam-1', 'subject-1', 'term-1', 'Parcial 1', '2026-05-20', '08:00', '10:00', 'Aula 3', 5000, 8000, 10000, 'passed', 'Repasar matrices');
            INSERT INTO topic(id, subject_id, name, description, sort_order)
              VALUES ('topic-1', 'subject-1', 'Matrices', 'Definiciones y operaciones', 1);
            INSERT INTO topic(id, subject_id, parent_topic_id, name, sort_order)
              VALUES ('topic-2', 'subject-1', 'topic-1', 'Operaciones', 2);
            INSERT INTO exam_topic(id, exam_id, topic_id)
              VALUES ('exam-topic-1', 'exam-1', 'topic-1');
            INSERT INTO grade_scale(id, subject_id, name, min_cp, max_cp, pass_cp, precision, is_default)
              VALUES ('scale-1', 'subject-1', 'Escala', 0, 10000, 6000, 2, 1);
            INSERT INTO grade_category(id, subject_id, scale_id, name, weight_bp, drop_lowest)
              VALUES ('category-1', 'subject-1', 'scale-1', 'Parciales', 6000, 1);
            INSERT INTO grade_category(id, subject_id, scale_id, name, weight_bp, drop_lowest)
              VALUES ('category-2', 'subject-1', 'scale-1', 'Trabajos', 4000, 0);
            INSERT INTO grade_entry(id, subject_id, category_id, exam_id, title, score_cp, max_cp, weight_bp, date)
              VALUES ('entry-1', 'subject-1', 'category-1', 'exam-1', 'Parcial 1', 8000, 10000, NULL, '2026-05-20');
            INSERT INTO grade_entry(id, subject_id, category_id, title, score_cp, max_cp, weight_bp, date)
              VALUES ('entry-2', 'subject-1', 'category-1', 'Parcial 2', 10000, 10000, NULL, '2026-06-20');
            INSERT INTO grade_entry(id, subject_id, category_id, title, score_cp, max_cp, weight_bp, date)
              VALUES ('entry-3', 'subject-1', 'category-2', 'Trabajo 1', 9000, 10000, NULL, '2026-06-01');
            INSERT INTO task(id, subject_id, title, description, start_date, due_date, due_time, priority, important, estimate_minutes, status, recurrence_json)
              VALUES ('task-1', 'subject-1', 'Repasar matrices', 'Repaso de álgebra', '2026-05-01', '2026-05-20', '18:00', 2, 1, 90, 'pending', '{{"kind":"weekly","interval":1,"until":"2026-06-30"}}');
            INSERT INTO task(id, subject_id, parent_task_id, title, priority, status, completed_at)
              VALUES ('task-2', 'subject-1', 'task-1', 'Hacer ejercicios', 1, 'done', '2026-05-19T10:00:00.000Z');
            INSERT INTO tag(id, name, color) VALUES ('tag-1', 'matemática', '#FF0000');
            INSERT INTO task_tag(id, task_id, tag_id)
              VALUES ('task-tag-1', 'task-1', 'tag-1');
            INSERT INTO saved_filter(id, screen, name, query_json)
              VALUES ('filter-1', 'tasks', 'Pendientes', '{{"text":"matrices","status":"open"}}');
            INSERT INTO self_assessment(id, topic_id, score, note, source, assessed_at)
              VALUES ('assessment-1', 'topic-1', 4, 'Repaso sólido', 'review', '2026-05-09T10:00:00.000Z');
            INSERT INTO strength_override(id, topic_id, classification, reason)
              VALUES ('strength-1', 'topic-1', 'strength', 'Práctica sostenida');
            INSERT INTO review_item(id, topic_id, algorithm, algorithm_version, ease_factor_cp, interval_days, repetitions, lapses, due_date, last_reviewed_at)
              VALUES ('review-1', 'topic-2', 'sm2', '1', 240, 6, 2, 1, '2026-05-25', '2026-05-19T10:00:00.000Z');
            INSERT INTO study_session(id, subject_id, topic_id, started_at, ended_at, planned_minutes, actual_minutes, mode, interruptions, notes)
              VALUES ('session-1', 'subject-1', 'topic-1', '2026-05-10T10:00:00.000Z', '2026-05-10T10:25:00.000Z', 25, 22, 'pomodoro', 1, 'Sesión de prueba');
            INSERT INTO note(id, subject_id, topic_id, title, body)
              VALUES ('note-1', 'subject-1', 'topic-1', 'Apunte', 'Contenido');
            INSERT INTO attachment(id, owner_type, owner_id, filename, mime, size_bytes, sha256)
              VALUES ('attachment-1', 'task', 'task-1', 'apunte.txt', 'text/plain', {size}, '{blob_sha}');
            "#,
            size = blob.len(),
        ))
        .expect("insertar fixture completo");

        (store, attachments, blob)
    }

    #[test]
    fn magic_and_version_are_stable() {
        assert_eq!(EXPORT_MAGIC, "STUDY-PLANNER-EXPORT");
        assert_eq!(EXPORT_END, "STUDY-PLANNER-END");
        assert_eq!(EXPORT_FORMAT_VERSION_VALUE, 1);
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn empty_vault_round_trips_byte_for_byte() {
        let attachments = temp_dir("empty-attachments");
        let source = migrated_store();
        let text = export_to_text(&source, &attachments).expect("exportar origen");

        let target = migrated_store();
        let report = import_from_text(&target, &attachments, &text, ImportMode::EmptyOnly)
            .expect("importar");
        assert!(report.rows > 0);
        assert_eq!(report.attachments, 0);

        let round_trip = export_to_text(&target, &attachments).expect("exportar destino");
        assert_eq!(text, round_trip);

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn tasks_and_soft_deletes_round_trip() {
        let attachments = temp_dir("tasks-attachments");
        let source = migrated_store();
        let first = source.create_task("Primera").unwrap();
        let second = source.create_task("Segunda").unwrap();
        source.set_task_done(&first.id, true).unwrap();
        source.delete_task(&second.id).unwrap();

        let text = export_to_text(&source, &attachments).expect("exportar");
        let target = migrated_store();
        import_from_text(&target, &attachments, &text, ImportMode::EmptyOnly).expect("importar");

        let tasks = target.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, first.id);

        let round_trip = export_to_text(&target, &attachments).expect("re-exportar");
        assert_eq!(text, round_trip);

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn all_tables_and_attachments_round_trip() {
        let (source, source_attachments, blob) = fixture_with_all_tables();
        let text = export_to_text(&source, &source_attachments).expect("exportar fixture");
        assert!(text.contains("\"table\":\"attachment_blob\""));
        assert!(text.contains("\"table\":\"setting\""));

        let target = migrated_store();
        let target_attachments = temp_dir("target-attachments");
        let report = import_from_text(&target, &target_attachments, &text, ImportMode::EmptyOnly)
            .expect("importar fixture");
        assert_eq!(report.attachments, 1);

        let written = target_attachments.join(sha256_hex(&blob));
        assert_eq!(fs::read(&written).expect("leer adjunto importado"), blob);

        let round_trip = export_to_text(&target, &target_attachments).expect("re-exportar");
        assert_eq!(text, round_trip);

        fs::remove_dir_all(source_attachments).unwrap();
        fs::remove_dir_all(target_attachments).unwrap();
    }

    #[test]
    fn phase_two_entities_round_trip_with_semantics() {
        let (source, source_attachments, _) = fixture_with_all_tables();
        let text = export_to_text(&source, &source_attachments).expect("exportar fixture");

        for table in [
            "academic_year",
            "term",
            "holiday",
            "rotation_config",
            "subject",
            "class_slot",
            "class_exception",
            "attendance_record",
        ] {
            assert!(
                text.contains(&format!("\"table\":\"{table}\"")),
                "falta la tabla {table} en la exportación"
            );
        }

        let target = migrated_store();
        let target_attachments = temp_dir("phase-two-attachments");
        import_from_text(&target, &target_attachments, &text, ImportMode::EmptyOnly)
            .expect("importar fixture");

        let conn = target.connection();
        for (table, expected) in [
            ("academic_year", 1),
            ("term", 1),
            ("holiday", 1),
            ("rotation_config", 1),
            ("subject", 1),
            ("class_slot", 1),
            ("class_exception", 1),
            ("attendance_record", 1),
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("contar filas");
            assert_eq!(count, expected, "filas de {table}");
        }

        let slot: (i64, String, String) = conn
            .query_row(
                "SELECT weekday, start_time, end_time FROM class_slot WHERE id = 'slot-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("leer franja");
        assert_eq!(slot, (1, "08:00".to_string(), "09:30".to_string()));

        let exception_kind: String = conn
            .query_row(
                "SELECT kind FROM class_exception WHERE id = 'exception-1'",
                [],
                |row| row.get(0),
            )
            .expect("leer excepción");
        assert_eq!(exception_kind, "moved");

        let attendance_status: String = conn
            .query_row(
                "SELECT status FROM attendance_record WHERE id = 'attendance-1'",
                [],
                |row| row.get(0),
            )
            .expect("leer asistencia");
        assert_eq!(attendance_status, "present");

        let rotation: (String, String, String) = conn
            .query_row(
                "SELECT mode, anchor_date, labels_json FROM rotation_config WHERE id = 'rotation-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("leer rotación");
        assert_eq!(
            rotation,
            (
                "weekly_ab".to_string(),
                "2026-03-02".to_string(),
                "[\"A\",\"B\"]".to_string()
            )
        );

        let round_trip = export_to_text(&target, &target_attachments).expect("re-exportar");
        assert_eq!(text, round_trip);

        fs::remove_dir_all(source_attachments).unwrap();
        fs::remove_dir_all(target_attachments).unwrap();
    }

    #[test]
    fn phase_three_entities_round_trip_with_semantics() {
        let (source, source_attachments, _) = fixture_with_all_tables();
        let text = export_to_text(&source, &source_attachments).expect("exportar fixture");

        for table in [
            "exam",
            "topic",
            "exam_topic",
            "grade_scale",
            "grade_category",
            "grade_entry",
            "task",
            "tag",
            "task_tag",
            "saved_filter",
        ] {
            assert!(
                text.contains(&format!("\"table\":\"{table}\"")),
                "falta la tabla {table} en la exportación"
            );
        }

        let target = migrated_store();
        let target_attachments = temp_dir("phase-three-attachments");
        import_from_text(&target, &target_attachments, &text, ImportMode::EmptyOnly)
            .expect("importar fixture");

        let conn = target.connection();
        for (table, expected) in [
            ("exam", 1),
            ("topic", 2),
            ("exam_topic", 1),
            ("grade_scale", 1),
            ("grade_category", 2),
            ("grade_entry", 3),
            ("task", 2),
            ("tag", 1),
            ("task_tag", 1),
            ("saved_filter", 1),
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("contar filas");
            assert_eq!(count, expected, "filas de {table}");
        }

        let exam: (i64, String, String) = conn
            .query_row(
                "SELECT weight_bp, result_status, notes FROM exam WHERE id = 'exam-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("leer examen");
        assert_eq!(
            exam,
            (5000, "passed".to_string(), "Repasar matrices".to_string())
        );

        let task: (i64, i64, i64, String) = conn
            .query_row(
                "SELECT priority, important, estimate_minutes, recurrence_json
                 FROM task WHERE id = 'task-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("leer tarea");
        assert_eq!(
            task,
            (
                2,
                1,
                90,
                "{\"kind\":\"weekly\",\"interval\":1,\"until\":\"2026-06-30\"}".to_string()
            )
        );

        let filter_query: String = conn
            .query_row(
                "SELECT query_json FROM saved_filter WHERE id = 'filter-1'",
                [],
                |row| row.get(0),
            )
            .expect("leer filtro guardado");
        assert_eq!(filter_query, "{\"text\":\"matrices\",\"status\":\"open\"}");

        let grade = target
            .get_subject_grade("subject-1")
            .expect("calcular promedio importado");
        assert_eq!(grade.final_bp, Some(9600));
        assert_eq!(grade.final_cp, Some(9600));
        assert_eq!(grade.passed, Some(true));
        assert_eq!(grade.categories.len(), 2);
        let parciales = grade
            .categories
            .iter()
            .find(|category| category.category_id == "category-1")
            .expect("categoría de parciales");
        assert_eq!(parciales.dropped, 1);
        assert_eq!(parciales.percent_bp, Some(10000));

        let round_trip = export_to_text(&target, &target_attachments).expect("re-exportar");
        assert_eq!(text, round_trip);

        fs::remove_dir_all(source_attachments).unwrap();
        fs::remove_dir_all(target_attachments).unwrap();
    }

    #[test]
    fn phase_four_entities_round_trip_with_semantics() {
        let (source, source_attachments, _) = fixture_with_all_tables();
        let text = export_to_text(&source, &source_attachments).expect("exportar fixture");

        for table in [
            "self_assessment",
            "strength_override",
            "review_item",
            "study_session",
        ] {
            assert!(
                text.contains(&format!("\"table\":\"{table}\"")),
                "falta la tabla {table} en la exportación"
            );
        }

        let target = migrated_store();
        let target_attachments = temp_dir("phase-four-attachments");
        import_from_text(&target, &target_attachments, &text, ImportMode::EmptyOnly)
            .expect("importar fixture");

        let conn = target.connection();
        for (table, expected) in [
            ("self_assessment", 1),
            ("strength_override", 1),
            ("review_item", 1),
            ("study_session", 1),
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("contar filas");
            assert_eq!(count, expected, "filas de {table}");
        }

        let assessment: (i64, String, String) = conn
            .query_row(
                "SELECT score, source, note FROM self_assessment WHERE id = 'assessment-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("leer autoevaluación");
        assert_eq!(
            assessment,
            (4, "review".to_string(), "Repaso sólido".to_string())
        );

        let override_row: (String, String) = conn
            .query_row(
                "SELECT classification, reason FROM strength_override WHERE id = 'strength-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("leer override");
        assert_eq!(
            override_row,
            ("strength".to_string(), "Práctica sostenida".to_string())
        );

        let review: (String, String, i64, i64, i64, i64, String) = conn
            .query_row(
                "SELECT algorithm, algorithm_version, ease_factor_cp, interval_days,
                        repetitions, lapses, due_date
                 FROM review_item WHERE id = 'review-1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .expect("leer repaso");
        assert_eq!(
            review,
            (
                "sm2".to_string(),
                "1".to_string(),
                240,
                6,
                2,
                1,
                "2026-05-25".to_string()
            )
        );

        let session: (i64, i64, String, i64, String) = conn
            .query_row(
                "SELECT planned_minutes, actual_minutes, mode, interruptions, notes
                 FROM study_session WHERE id = 'session-1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("leer sesión");
        assert_eq!(
            session,
            (
                25,
                22,
                "pomodoro".to_string(),
                1,
                "Sesión de prueba".to_string()
            )
        );

        let studies = target
            .list_topic_studies(None, "2026-05-10")
            .expect("listar estudios importados");
        let matrices = studies
            .iter()
            .find(|study| study.topic_id == "topic-1")
            .expect("tema importado");
        assert_eq!(
            matrices.classification,
            study_planner_core::StrengthClassification::Strength
        );
        assert_eq!(
            matrices.latest_assessment.as_ref().map(|item| item.score),
            Some(4)
        );

        let round_trip = export_to_text(&target, &target_attachments).expect("re-exportar");
        assert_eq!(text, round_trip);

        fs::remove_dir_all(source_attachments).unwrap();
        fs::remove_dir_all(target_attachments).unwrap();
    }

    #[test]
    fn rejects_an_altered_checksum() {
        let attachments = temp_dir("checksum");
        let source = migrated_store();
        let mut text = export_to_text(&source, &attachments).unwrap();
        let position = text.find("body-sha256: ").unwrap() + "body-sha256: ".len();
        let replacement = if &text[position..position + 1] == "a" {
            "b"
        } else {
            "a"
        };
        text.replace_range(position..position + 1, replacement);

        let target = migrated_store();
        let error = import_from_text(&target, &attachments, &text, ImportMode::EmptyOnly)
            .expect_err("checksum alterado");
        assert!(matches!(error, TransferError::ChecksumMismatch { .. }));

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn rejects_future_format_and_schema_versions() {
        let attachments = temp_dir("versions");
        let source = migrated_store();
        let text = export_to_text(&source, &attachments).unwrap();
        let target = migrated_store();

        let future_format = text.replace("format-version: 1", "format-version: 2");
        let error = import_from_text(&target, &attachments, &future_format, ImportMode::EmptyOnly)
            .expect_err("formato futuro");
        assert!(matches!(error, TransferError::InvalidFormat(_)));

        let future_schema = text.replace("schema-version: 1", "schema-version: 2");
        let error = import_from_text(&target, &attachments, &future_schema, ImportMode::EmptyOnly)
            .expect_err("esquema futuro");
        assert!(matches!(error, TransferError::InvalidFormat(_)));

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn rejects_missing_or_extra_columns_with_line_number() {
        let attachments = temp_dir("columns");
        let source = seeded_store();
        let text = export_to_text(&source, &attachments).unwrap();
        let mut document = parse_export(&text).unwrap();

        let task_row = document
            .rows
            .iter_mut()
            .find(|row| row.table == "task")
            .expect("hay tareas");
        task_row.row.remove("status");
        let broken = export_text(&document).unwrap();

        let target = migrated_store();
        let error = import_from_text(&target, &attachments, &broken, ImportMode::EmptyOnly)
            .expect_err("columna faltante");
        assert!(error.to_string().contains("línea"));

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn rejects_boolean_values() {
        let attachments = temp_dir("bool");
        let source = seeded_store();
        let text = export_to_text(&source, &attachments).unwrap();
        let mut document = parse_export(&text).unwrap();

        let setting = document
            .rows
            .iter_mut()
            .find(|row| row.table == "setting")
            .expect("hay settings");
        setting
            .row
            .insert("value_json".to_string(), Value::Bool(true));
        let broken = export_text(&document).unwrap();

        let target = migrated_store();
        let error = import_from_text(&target, &attachments, &broken, ImportMode::EmptyOnly)
            .expect_err("booleano inválido");
        assert!(error.to_string().contains("strings, enteros o null"));

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn invalid_foreign_key_rolls_back_and_leaves_vault_empty() {
        let attachments = temp_dir("fk");
        let source = seeded_store();
        let text = export_to_text(&source, &attachments).unwrap();
        let mut document = parse_export(&text).unwrap();

        let task = document
            .rows
            .iter_mut()
            .find(|row| row.table == "task")
            .expect("hay tareas");
        task.row.insert(
            "subject_id".to_string(),
            Value::String("no-existe".to_string()),
        );
        let broken = export_text(&document).unwrap();

        let target = migrated_store();
        let error = import_from_text(&target, &attachments, &broken, ImportMode::EmptyOnly)
            .expect_err("FK inválida");
        assert!(error.to_string().contains("claves foráneas"));
        assert!(is_empty_vault(&target).unwrap());

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn attachment_hash_mismatch_rolls_back_without_writing_files() {
        let (source, source_attachments, _) = fixture_with_all_tables();
        let text = export_to_text(&source, &source_attachments).unwrap();
        let mut document = parse_export(&text).unwrap();

        let wrong = "0".repeat(64);
        for row in &mut document.rows {
            if row.table == "attachment" {
                row.row
                    .insert("sha256".to_string(), Value::String(wrong.clone()));
            }
            if row.table == "attachment_blob" {
                row.row
                    .insert("sha256".to_string(), Value::String(wrong.clone()));
            }
        }
        let broken = export_text(&document).unwrap();

        let target = migrated_store();
        let target_attachments = temp_dir("bad-hash-target");
        let error = import_from_text(&target, &target_attachments, &broken, ImportMode::EmptyOnly)
            .expect_err("hash inválido");
        assert!(error.to_string().contains("SHA-256"));
        assert!(is_empty_vault(&target).unwrap());
        assert_eq!(fs::read_dir(&target_attachments).unwrap().count(), 0);

        fs::remove_dir_all(source_attachments).unwrap();
        fs::remove_dir_all(target_attachments).unwrap();
    }

    #[test]
    fn refuses_to_import_into_a_non_empty_vault() {
        let attachments = temp_dir("not-empty");
        let source = migrated_store();
        let text = export_to_text(&source, &attachments).unwrap();
        let target = migrated_store();
        target.create_task("Ya hay datos").unwrap();

        let error = import_from_text(&target, &attachments, &text, ImportMode::EmptyOnly)
            .expect_err("bóveda no vacía");
        assert!(matches!(error, TransferError::VaultNotEmpty));

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn rejects_rows_out_of_canonical_order() {
        let attachments = temp_dir("order");
        let source = seeded_store();
        let text = export_to_text(&source, &attachments).unwrap();
        let mut document = parse_export(&text).unwrap();
        document.rows.swap(0, 1);
        let broken = export_text(&document).unwrap();

        let target = migrated_store();
        let error = import_from_text(&target, &attachments, &broken, ImportMode::EmptyOnly)
            .expect_err("orden inválido");
        assert!(error.to_string().contains("orden canónico"));

        fs::remove_dir_all(attachments).unwrap();
    }

    #[test]
    fn rejects_crlf_and_bom() {
        let source = migrated_store();
        let attachments = temp_dir("encodings");
        let text = export_to_text(&source, &attachments).unwrap();

        let target = migrated_store();
        assert!(matches!(
            import_from_text(
                &target,
                &attachments,
                &text.replace('\n', "\r\n"),
                ImportMode::EmptyOnly
            ),
            Err(TransferError::InvalidFormat(_))
        ));
        assert!(matches!(
            import_from_text(
                &target,
                &attachments,
                &format!("\u{feff}{text}"),
                ImportMode::EmptyOnly
            ),
            Err(TransferError::InvalidFormat(_))
        ));

        fs::remove_dir_all(attachments).unwrap();
    }
}
