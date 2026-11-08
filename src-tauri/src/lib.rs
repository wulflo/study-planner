use serde::Serialize;
use study_planner_core::{AppInfo, app_info};
use study_planner_store::Store;
use tauri::Manager;

mod vault;

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = vault::load_state(app.handle());
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            schema_self_check,
            vault::get_vault_status,
            vault::choose_vault,
            vault::create_backup,
            vault::export_vault,
            vault::import_vault,
            vault::restore_main_backup,
            vault::list_tasks,
            vault::create_task,
            vault::set_task_done,
            vault::delete_task,
            vault::list_academic_years,
            vault::create_academic_year,
            vault::update_academic_year,
            vault::delete_academic_year,
            vault::list_terms,
            vault::create_term,
            vault::update_term,
            vault::delete_term,
            vault::list_subjects,
            vault::create_subject,
            vault::update_subject,
            vault::delete_subject,
            vault::list_class_slots,
            vault::create_class_slot,
            vault::update_class_slot,
            vault::delete_class_slot,
            vault::get_rotation,
            vault::save_rotation,
            vault::delete_rotation,
            vault::list_holidays,
            vault::create_holiday,
            vault::delete_holiday,
            vault::list_class_exceptions,
            vault::create_class_exception,
            vault::delete_class_exception,
            vault::list_attendance,
            vault::set_attendance,
            vault::clear_attendance,
            vault::get_week_schedule,
            vault::get_next_class
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar Study Planner");
}
