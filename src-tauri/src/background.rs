//! Segundo plano y bandeja del sistema.
//!
//! La app puede seguir viva después de cerrar la ventana para revisar
//! recordatorios. El icono de bandeja ofrece mostrar, revisar ahora y salir.
//! Si el escritorio no tiene bandeja, cerrar la ventana cierra la app.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State};

use crate::notifications;
use crate::vault::AppState;

const BACKGROUND_KEY: &str = "background.enabled";
const TRAY_ID: &str = "study-planner-tray";

/// Estado del modo segundo plano para la vista «Ajustes».
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundStatus {
    pub enabled: bool,
    pub tray_available: bool,
}

/// Estado en memoria, independiente de que haya una bóveda configurada.
pub struct BackgroundRuntime {
    enabled: AtomicBool,
    tray_available: AtomicBool,
    quitting: AtomicBool,
}

impl BackgroundRuntime {
    fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            tray_available: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
        }
    }

    fn status(&self) -> BackgroundStatus {
        BackgroundStatus {
            enabled: self.enabled.load(Ordering::Relaxed),
            tray_available: self.tray_available.load(Ordering::Relaxed),
        }
    }

    pub fn set_tray_available(&self, available: bool) {
        self.tray_available.store(available, Ordering::Relaxed);
    }
}

fn stored_enabled(state: &AppState) -> Option<bool> {
    state
        .with_store_opt(|store| {
            Ok(store.and_then(|store| {
                store
                    .get_setting(BACKGROUND_KEY)
                    .ok()
                    .flatten()
                    .map(|value| value.trim() == "true")
            }))
        })
        .ok()
        .flatten()
}

/// Carga la preferencia al iniciar. Sin bóveda el modo queda activo por
/// defecto: así la ventana se oculta en la bandeja en cuanto haya una.
pub fn load(state: &AppState) -> BackgroundRuntime {
    BackgroundRuntime::new(stored_enabled(state).unwrap_or(true))
}

#[tauri::command]
pub fn get_background_status(runtime: State<'_, BackgroundRuntime>) -> BackgroundStatus {
    runtime.status()
}

#[tauri::command]
pub fn set_background_mode(
    enabled: bool,
    state: State<'_, AppState>,
    runtime: State<'_, BackgroundRuntime>,
) -> Result<BackgroundStatus, String> {
    runtime.enabled.store(enabled, Ordering::Relaxed);

    // Persistir es best-effort: sin bóveda el modo sigue funcionando en esta
    // sesión y se vuelve a intentar la próxima vez que haya una.
    let _ = state.with_store_opt(|store| {
        if let Some(store) = store {
            store
                .set_setting(
                    BACKGROUND_KEY,
                    if enabled { "true" } else { "false" },
                    "portable",
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    });

    Ok(runtime.status())
}

pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Crea el icono de bandeja. Devuelve si el escritorio permitió crearlo.
///
/// Es idempotente: si el icono ya existe (por ejemplo porque la ventana se
/// ocultó y se volvió a crear el runtime) no agrega un segundo icono.
pub fn setup_tray(app: &AppHandle) -> tauri::Result<bool> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(true);
    }

    let show = MenuItem::with_id(app, "show", "Mostrar Study Planner", true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", "Revisar recordatorios", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &scan, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Study Planner")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "scan" => {
                let app = app.clone();
                std::thread::spawn(move || notifications::scan_now(&app));
            }
            "quit" => {
                if let Some(runtime) = app.try_state::<BackgroundRuntime>() {
                    runtime.quitting.store(true, Ordering::Relaxed);
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(true)
}

/// Cierra a la bandeja cuando el modo segundo plano está activo.
pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    let app = window.app_handle();
    let Some(runtime) = app.try_state::<BackgroundRuntime>() else {
        return;
    };

    if runtime.quitting.load(Ordering::Relaxed) {
        return;
    }

    if runtime.enabled.load(Ordering::Relaxed) && runtime.tray_available.load(Ordering::Relaxed) {
        api.prevent_close();
        let _ = window.hide();
    }
}
