use serde::Serialize;
use study_planner_core::{AppInfo, app_info};
use study_planner_store::Store;
use tauri::Manager;

mod background;
#[cfg(unix)]
mod instance;
mod notifications;
mod repository;
mod updater;
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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Una segunda instancia no abre otra ventana: le muestra la
            // principal a la que ya está corriendo (aunque esté en la bandeja).
            background::show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = vault::load_state(app.handle());
            let background = background::load(&state);
            app.manage(state);
            app.manage(background);

            // El plugin de instancia única cubre el caso normal. Este respaldo
            // evita una segunda ventana y un segundo icono cuando el plugin no
            // puede publicar su nombre D-Bus (snap `strict`, contenedores).
            #[cfg(unix)]
            match app.path().app_local_data_dir() {
                Ok(dir) => {
                    let handle = app.handle().clone();
                    match instance::acquire(&dir, move || {
                        let target = handle.clone();
                        let _ = handle.run_on_main_thread(move || {
                            background::show_main_window(&target);
                        });
                    }) {
                        Ok(instance::Acquired::Primary(guard)) => {
                            app.manage(guard);
                        }
                        Ok(instance::Acquired::Secondary { notified }) => {
                            if notified {
                                eprintln!(
                                    "Study Planner ya está en ejecución; se pidió mostrar la ventana existente."
                                );
                            } else {
                                eprintln!(
                                    "Study Planner ya está en ejecución y no respondió; no se abre otra instancia."
                                );
                            }
                            app.handle().exit(0);
                            return Ok(());
                        }
                        Err(error) => {
                            eprintln!("No se pudo comprobar la instancia única: {error}");
                        }
                    }
                }
                Err(error) => {
                    eprintln!("No se pudo resolver el directorio de datos: {error}");
                }
            }

            let tray_available = match background::setup_tray(app.handle()) {
                Ok(available) => available,
                Err(error) => {
                    eprintln!("No se pudo crear la bandeja del sistema: {error}");
                    false
                }
            };
            app.state::<background::BackgroundRuntime>()
                .set_tray_available(tray_available);

            let handle = app.handle().clone();
            std::thread::spawn(move || updater::auto_check_on_startup(&handle));
            notifications::start_scheduler(app.handle().clone());
            Ok(())
        })
        .on_window_event(background::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            schema_self_check,
            repository::open_repository,
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
            vault::create_task_full,
            vault::update_task,
            vault::set_task_status,
            vault::search_tasks,
            vault::list_tags,
            vault::create_tag,
            vault::update_tag,
            vault::delete_tag,
            vault::set_task_tags,
            vault::list_saved_filters,
            vault::create_saved_filter,
            vault::update_saved_filter,
            vault::delete_saved_filter,
            vault::list_undo_entries,
            vault::undo_entry,
            vault::purge_expired_undo,
            vault::list_exams,
            vault::create_exam,
            vault::update_exam,
            vault::delete_exam,
            vault::list_topics,
            vault::create_topic,
            vault::update_topic,
            vault::delete_topic,
            vault::list_exam_topics,
            vault::link_exam_topic,
            vault::unlink_exam_topic,
            vault::list_grade_scales,
            vault::create_grade_scale,
            vault::update_grade_scale,
            vault::delete_grade_scale,
            vault::list_grade_categories,
            vault::create_grade_category,
            vault::update_grade_category,
            vault::delete_grade_category,
            vault::list_grade_entries,
            vault::create_grade_entry,
            vault::update_grade_entry,
            vault::delete_grade_entry,
            vault::get_subject_grade,
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
            vault::get_next_class,
            vault::list_self_assessments,
            vault::create_self_assessment,
            vault::delete_self_assessment,
            vault::list_strength_overrides,
            vault::set_strength_override,
            vault::clear_strength_override,
            vault::list_review_items,
            vault::review_topic,
            vault::set_review_due,
            vault::delete_review_item,
            vault::list_study_sessions,
            vault::start_study_session,
            vault::finish_study_session,
            vault::delete_study_session,
            vault::list_topic_studies,
            vault::list_study_priorities,
            vault::get_study_stats,
            updater::get_updater_status,
            updater::set_updater_auto_check,
            updater::check_updates,
            updater::install_update,
            updater::rollback_update,
            updater::list_network_events,
            notifications::get_notification_status,
            notifications::set_notifications_enabled,
            notifications::set_notifications_sound,
            notifications::run_notification_scan,
            notifications::send_test_notification,
            background::get_background_status,
            background::set_background_mode
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar Study Planner");
}
