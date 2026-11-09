//! Notificaciones locales calculadas y entregadas desde Rust.
//!
//! No se registra ningún plugin de notificaciones ni se concede permiso a la
//! webview. El cálculo vive en `study-planner-store` y la entrega usa D-Bus
//! (`org.freedesktop.Notifications`) directamente desde el host.

use std::time::Duration;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

use serde::Serialize;
use study_planner_store::{NotificationRecord, Reminder};
use tauri::{Manager, State};

use crate::vault::AppState;

const ENABLED_KEY: &str = "notifications.enabled";
const SOUND_KEY: &str = "notifications.sound";
const CLASS_LEAD_KEY: &str = "notifications.class_lead_minutes";
const TASK_LEAD_KEY: &str = "notifications.task_lead_minutes";
const EXAM_LEAD_KEY: &str = "notifications.exam_lead_hours";
const SCAN_INTERVAL: Duration = Duration::from_secs(60);

/// Estado de las notificaciones para la vista «Ajustes».
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub enabled: bool,
    pub sound: bool,
    pub class_lead_minutes: i64,
    pub task_lead_minutes: i64,
    pub exam_lead_hours: i64,
    pub recent: Vec<NotificationRecord>,
}

/// Resultado de una pasada del planificador.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOutcome {
    pub delivered: usize,
    pub failed: usize,
}

fn read_bool(store: &study_planner_store::Store, key: &str, default: bool) -> bool {
    store
        .get_setting(key)
        .ok()
        .flatten()
        .map_or(default, |value| value.trim() == "true")
}

fn read_i64(store: &study_planner_store::Store, key: &str, default: i64) -> i64 {
    store
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(default)
}

fn status_of(state: &AppState) -> Result<NotificationStatus, String> {
    state.with_store(|store| {
        Ok(NotificationStatus {
            enabled: read_bool(store, ENABLED_KEY, true),
            sound: read_bool(store, SOUND_KEY, true),
            class_lead_minutes: read_i64(store, CLASS_LEAD_KEY, 10),
            task_lead_minutes: read_i64(store, TASK_LEAD_KEY, 60),
            exam_lead_hours: read_i64(store, EXAM_LEAD_KEY, 24),
            recent: store
                .list_notifications(20)
                .map_err(|error| error.to_string())?,
        })
    })
}

/// Lee la preferencia de sonido sin exigir una bóveda abierta.
fn sound_enabled(state: &AppState) -> bool {
    state
        .with_store_opt(|store| Ok(store.is_none_or(|store| read_bool(store, SOUND_KEY, true))))
        .unwrap_or(true)
}

#[tauri::command]
pub fn get_notification_status(state: State<'_, AppState>) -> Result<NotificationStatus, String> {
    status_of(&state)
}

#[tauri::command]
pub fn set_notifications_enabled(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<NotificationStatus, String> {
    state.with_store(|store| {
        store
            .set_setting(
                ENABLED_KEY,
                if enabled { "true" } else { "false" },
                "portable",
            )
            .map_err(|error| error.to_string())
    })?;
    status_of(&state)
}

#[tauri::command]
pub fn set_notifications_sound(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<NotificationStatus, String> {
    state.with_store(|store| {
        store
            .set_setting(
                SOUND_KEY,
                if enabled { "true" } else { "false" },
                "portable",
            )
            .map_err(|error| error.to_string())
    })?;
    status_of(&state)
}

/// Revisa y entrega ahora los recordatorios pendientes.
#[tauri::command]
pub fn run_notification_scan(state: State<'_, AppState>) -> Result<ScanOutcome, String> {
    scan_reminders(&state)
}

/// Variante para la bandeja del sistema: no propaga errores y respeta el
/// interruptor de notificaciones.
pub fn scan_now(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let enabled = state
        .with_store(|store| Ok(read_bool(store, ENABLED_KEY, true)))
        .unwrap_or(false);
    if enabled {
        let _ = scan_reminders(&state);
    }
}

/// Envía una notificación de prueba sin tocar la base ni la red.
#[tauri::command]
pub fn send_test_notification(state: State<'_, AppState>) -> Result<(), String> {
    send_notification(
        "Study Planner",
        "Las notificaciones locales funcionan. Este aviso no salió de la bóveda.",
        sound_enabled(&state),
    )
}

fn scan_reminders(state: &State<'_, AppState>) -> Result<ScanOutcome, String> {
    let sound = sound_enabled(state);
    let claims: Vec<(String, Reminder)> = state.with_store(|store| {
        let mut claims = Vec::new();
        for reminder in store.due_reminders().map_err(|error| error.to_string())? {
            if let Some(id) = store
                .claim_notification(&reminder)
                .map_err(|error| error.to_string())?
            {
                claims.push((id, reminder));
            }
        }
        Ok(claims)
    })?;

    let mut delivered = 0;
    let mut failed = 0;
    for (id, reminder) in claims {
        match send_notification(&reminder.title, &reminder.body, sound) {
            Ok(()) => {
                state
                    .with_store(|store| {
                        store
                            .mark_notification_fired(&id)
                            .map_err(|error| error.to_string())
                    })
                    .ok();
                delivered += 1;
            }
            Err(_) => {
                // Si el escritorio no está disponible, se libera el reclamo
                // para reintentarlo en la próxima pasada.
                state
                    .with_store(|store| {
                        store
                            .release_notification(&id)
                            .map_err(|error| error.to_string())
                    })
                    .ok();
                failed += 1;
            }
        }
    }

    Ok(ScanOutcome { delivered, failed })
}

/// Planificador en segundo plano. Corre mientras la app está abierta.
pub fn start_scheduler(app: tauri::AppHandle) {
    #[cfg(target_os = "linux")]
    start_linux_scheduler(app);
    #[cfg(not(target_os = "linux"))]
    {
        // La entrega usa D-Bus; en otras plataformas todavía no hay backend.
        let _ = app;
    }
}

#[cfg(target_os = "linux")]
fn start_linux_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            let state = app.state::<AppState>();
            let enabled = state
                .with_store(|store| Ok(read_bool(store, ENABLED_KEY, true)))
                .unwrap_or(false);
            if enabled {
                let _ = scan_reminders(&state);
            }
            std::thread::sleep(SCAN_INTERVAL);
        }
    });
}

#[cfg(target_os = "linux")]
fn send_notification(title: &str, body: &str, sound: bool) -> Result<(), String> {
    let connection = zbus::blocking::Connection::session()
        .map_err(|error| format!("No se pudo abrir la sesión D-Bus: {error}"))?;
    let mut hints: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
    if sound {
        // El escritorio decide qué sonido asociar a este nombre estándar.
        hints.insert(
            "sound-name",
            zbus::zvariant::Value::from("message-new-instant"),
        );
    }
    connection
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &(
                "Study Planner",
                0u32,
                "study-planner",
                title,
                body,
                Vec::<&str>::new(),
                hints,
                8000i32,
            ),
        )
        .map_err(|error| format!("El escritorio rechazó la notificación: {error}"))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn send_notification(_title: &str, _body: &str, _sound: bool) -> Result<(), String> {
    Err("Las notificaciones locales solo están disponibles en Linux.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_defaults_are_opt_in_but_enabled() {
        let store = study_planner_store::Store::open_in_memory().expect("memoria");
        let mut store = store;
        store.migrate().expect("migrar");
        assert!(read_bool(&store, ENABLED_KEY, true));
        assert!(read_bool(&store, SOUND_KEY, true));
        assert_eq!(read_i64(&store, CLASS_LEAD_KEY, 10), 10);
        assert_eq!(read_i64(&store, TASK_LEAD_KEY, 60), 60);
        assert_eq!(read_i64(&store, EXAM_LEAD_KEY, 24), 24);
    }
}
