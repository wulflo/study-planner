use omega_core::{AppInfo, app_info};
use omega_store::Store;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SchemaCheck {
    schema_version: i64,
    integrity: String,
    foreign_key_violations: usize,
    vault_id: String,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    app_info()
}

/// Autodiagnóstico sin tocar la bóveda real: migra una base en memoria.
#[tauri::command]
fn schema_self_check() -> Result<SchemaCheck, String> {
    let mut store = Store::open_in_memory().map_err(|error| error.to_string())?;
    store.migrate().map_err(|error| error.to_string())?;
    Ok(SchemaCheck {
        schema_version: store.schema_version().map_err(|error| error.to_string())?,
        integrity: store.integrity_check().map_err(|error| error.to_string())?,
        foreign_key_violations: store
            .foreign_key_violations()
            .map_err(|error| error.to_string())?,
        vault_id: store.vault_id().map_err(|error| error.to_string())?,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_info, schema_self_check])
        .run(tauri::generate_context!())
        .expect("error al ejecutar Omega");
}
