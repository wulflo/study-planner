//! Comandos Tauri del updater aislado.
//!
//! La webview no recibe URLs, firmas ni rutas configurables: solo invoca
//! operaciones sin argumentos de red y recibe un resumen listo para mostrar.
//! Toda la lógica sensible vive en `study-planner-updater` y en Rust.

use std::path::PathBuf;

use serde::Serialize;
use study_planner_store::{NetworkEvent, Store};
use study_planner_updater::{
    ReqwestFetcher, UPDATE_HOST, UpdateCandidate, UpdateError, appimage_backup_path,
    check_for_update_with_size, download_update, install_appimage, public_key_configured,
    rollback_appimage,
};
use tauri::State;

use crate::vault::AppState;

const AUTO_CHECK_KEY: &str = "updater.auto_check";

/// Estado del updater para la vista «Ajustes».
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub public_key_configured: bool,
    pub current_version: String,
    pub update_host: String,
    pub auto_check: bool,
    pub running_as_appimage: bool,
    pub managed_by_snap: bool,
    pub can_rollback: bool,
}

/// Actualización disponible, sin exponer la URL ni la firma a la webview.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub pub_date: Option<String>,
    pub notes: String,
}

/// Resultado de instalar una actualización.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutcome {
    pub version: String,
    pub restart_required: bool,
}

fn current_version() -> String {
    study_planner_core::app_info().version.to_string()
}

/// Entorno de ejecución relevante para actualizar la app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Runtime {
    /// La app corre desde un AppImage y puede reemplazarse a sí misma.
    AppImage,
    /// La app corre desde un snap; snapd gestiona las actualizaciones.
    Snap,
    /// `tauri dev` u otro entorno sin AppImage ni snap.
    Development,
}

fn runtime_from(appimage: bool, snap: bool) -> Runtime {
    if snap {
        Runtime::Snap
    } else if appimage {
        Runtime::AppImage
    } else {
        Runtime::Development
    }
}

fn runtime() -> Runtime {
    runtime_from(
        std::env::var_os("APPIMAGE").is_some(),
        std::env::var_os("SNAP").is_some(),
    )
}

fn appimage_path() -> Result<PathBuf, String> {
    match runtime() {
        Runtime::Snap => Err(UpdateError::ManagedBySnap.to_string()),
        Runtime::AppImage => Ok(PathBuf::from(
            std::env::var_os("APPIMAGE").expect("APPIMAGE comprobado por runtime()"),
        )),
        Runtime::Development => Err(UpdateError::NotAppImage.to_string()),
    }
}

/// Lee un ajuste booleano; si no existe, usa `false` (opt-in).
fn read_bool(store: &Store, key: &str) -> Result<bool, String> {
    let raw = store.get_setting(key).map_err(|error| error.to_string())?;
    Ok(raw.is_some_and(|value| value.trim() == "true"))
}

fn write_bool(store: &Store, key: &str, value: bool) -> Result<(), String> {
    store
        .set_setting(key, if value { "true" } else { "false" }, "portable")
        .map_err(|error| error.to_string())
}

fn record_event(
    store: &Store,
    kind: &str,
    host: &str,
    bytes_sent: i64,
    bytes_received: i64,
    outcome: &str,
    detail: Option<&str>,
) {
    // El registro es una comodidad de auditoría: no debe tumbar la operación.
    let _ = store.log_network_event(
        kind,
        host,
        bytes_sent,
        bytes_received,
        outcome,
        detail.map(truncate_detail),
    );
}

fn truncate_detail(detail: &str) -> &str {
    detail.get(..detail.len().min(500)).unwrap_or(detail)
}

fn host_of(url: &str) -> &str {
    url.strip_prefix("https://")
        .and_then(|rest| rest.split('/').next())
        .and_then(|authority| authority.split(':').next())
        .unwrap_or(UPDATE_HOST)
}

fn info_of(candidate: &UpdateCandidate) -> UpdateInfo {
    UpdateInfo {
        version: candidate.version.clone(),
        pub_date: candidate.pub_date.clone(),
        notes: candidate.notes.clone(),
    }
}

fn build_status(state: &AppState) -> Result<UpdaterStatus, String> {
    let auto_check = state.with_store(|store| read_bool(store, AUTO_CHECK_KEY))?;
    let runtime = runtime();
    let appimage = if runtime == Runtime::AppImage {
        std::env::var_os("APPIMAGE").map(PathBuf::from)
    } else {
        None
    };
    let can_rollback = appimage
        .as_deref()
        .is_some_and(|path| appimage_backup_path(path).is_file());

    Ok(UpdaterStatus {
        public_key_configured: public_key_configured(),
        current_version: current_version(),
        update_host: UPDATE_HOST.to_string(),
        auto_check,
        running_as_appimage: runtime == Runtime::AppImage,
        managed_by_snap: runtime == Runtime::Snap,
        can_rollback,
    })
}

#[tauri::command]
pub fn get_updater_status(state: State<'_, AppState>) -> Result<UpdaterStatus, String> {
    build_status(&state)
}

/// Activa o desactiva la comprobación automática. Por defecto está apagada.
#[tauri::command]
pub fn set_updater_auto_check(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<UpdaterStatus, String> {
    state.with_store(|store| write_bool(store, AUTO_CHECK_KEY, enabled))?;
    build_status(&state)
}

#[tauri::command]
pub fn check_updates(state: State<'_, AppState>) -> Result<Option<UpdateInfo>, String> {
    run_check(&state)
}

fn run_check(state: &AppState) -> Result<Option<UpdateInfo>, String> {
    if runtime() == Runtime::Snap {
        return Err(UpdateError::ManagedBySnap.to_string());
    }
    if !public_key_configured() {
        return Err(UpdateError::PublicKeyNotConfigured.to_string());
    }

    let version = current_version();
    let fetcher = ReqwestFetcher::new(&version).map_err(|error| error.to_string())?;
    let result = check_for_update_with_size(&fetcher, &version);
    let bytes_received = result
        .as_ref()
        .map_or(0, |check| check.manifest_bytes as i64);

    state.with_store(|store| {
        match &result {
            Ok(check) => match &check.candidate {
                Some(candidate) => record_event(
                    store,
                    "update_check",
                    UPDATE_HOST,
                    0,
                    bytes_received,
                    "ok",
                    Some(&format!("actualización {} disponible", candidate.version)),
                ),
                None => record_event(
                    store,
                    "update_check",
                    UPDATE_HOST,
                    0,
                    bytes_received,
                    "ok",
                    Some("sin actualizaciones"),
                ),
            },
            Err(error) => record_event(
                store,
                "update_check",
                UPDATE_HOST,
                0,
                bytes_received,
                "error",
                Some(&error.to_string()),
            ),
        }
        Ok(())
    })?;

    result
        .map(|check| check.candidate.as_ref().map(info_of))
        .map_err(|error| error.to_string())
}

/// Comprobación automática al iniciar, solo si el usuario activó el opt-in.
///
/// Se ejecuta en un hilo aparte para no bloquear la ventana. El resultado
/// queda en `network_event`; la UI lo muestra en «Ajustes».
pub fn auto_check_on_startup(app: &tauri::AppHandle) {
    use tauri::Manager as _;

    if runtime() == Runtime::Snap {
        return;
    }

    let state = app.state::<AppState>();
    let enabled = state
        .with_store(|store| read_bool(store, AUTO_CHECK_KEY))
        .unwrap_or(false);
    if enabled && public_key_configured() {
        let _ = run_check(&state);
    }
}

/// Descarga, verifica e instala. Vuelve a consultar el manifiesto para no
/// aceptar URL ni firma desde la webview.
#[tauri::command]
pub fn install_update(state: State<'_, AppState>) -> Result<InstallOutcome, String> {
    if !public_key_configured() {
        return Err(UpdateError::PublicKeyNotConfigured.to_string());
    }
    let target = appimage_path()?;
    let version = current_version();
    let fetcher = ReqwestFetcher::new(&version).map_err(|error| error.to_string())?;

    let check = check_for_update_with_size(&fetcher, &version).map_err(|error| {
        state
            .with_store(|store| {
                record_event(
                    store,
                    "update_check",
                    UPDATE_HOST,
                    0,
                    0,
                    "error",
                    Some(&error.to_string()),
                );
                Ok(())
            })
            .ok();
        error.to_string()
    })?;

    let candidate = check
        .candidate
        .ok_or_else(|| "No hay una actualización disponible para instalar.".to_string())?;

    state.with_store(|store| {
        record_event(
            store,
            "update_check",
            UPDATE_HOST,
            0,
            check.manifest_bytes as i64,
            "ok",
            Some(&format!("actualización {} disponible", candidate.version)),
        );
        Ok(())
    })?;

    let artifact = match download_update(
        &fetcher,
        &candidate,
        study_planner_updater::UPDATE_PUBLIC_KEY,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            state.with_store(|store| {
                record_event(
                    store,
                    "update_download",
                    host_of(&candidate.url),
                    0,
                    0,
                    "error",
                    Some(&error.to_string()),
                );
                Ok(())
            })?;
            return Err(error.to_string());
        }
    };

    state.with_store(|store| {
        record_event(
            store,
            "update_download",
            host_of(&candidate.url),
            0,
            artifact.len() as i64,
            "ok",
            Some(&format!("artefacto {} verificado", candidate.version)),
        );
        Ok(())
    })?;

    install_appimage(&artifact, &target).map_err(|error| {
        state
            .with_store(|store| {
                record_event(
                    store,
                    "update_install",
                    UPDATE_HOST,
                    0,
                    0,
                    "error",
                    Some(&error.to_string()),
                );
                Ok(())
            })
            .ok();
        error.to_string()
    })?;

    state.with_store(|store| {
        record_event(
            store,
            "update_install",
            UPDATE_HOST,
            0,
            0,
            "ok",
            Some(&format!(
                "instalado {}; backup disponible",
                candidate.version
            )),
        );
        Ok(())
    })?;

    Ok(InstallOutcome {
        version: candidate.version,
        restart_required: true,
    })
}

#[tauri::command]
pub fn rollback_update(state: State<'_, AppState>) -> Result<(), String> {
    let target = appimage_path()?;
    rollback_appimage(&target).map_err(|error| error.to_string())?;
    state.with_store(|store| {
        record_event(
            store,
            "update_rollback",
            UPDATE_HOST,
            0,
            0,
            "ok",
            Some("versión anterior restaurada"),
        );
        Ok(())
    })?;
    Ok(())
}

#[tauri::command]
pub fn list_network_events(state: State<'_, AppState>) -> Result<Vec<NetworkEvent>, String> {
    state.with_store(|store| {
        store
            .list_network_events(50)
            .map_err(|error| error.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_is_extracted_from_allowed_urls() {
        assert_eq!(
            host_of("https://github.com/wulflo/study-planner/releases/app.AppImage"),
            "github.com"
        );
        assert_eq!(
            host_of("https://objects.githubusercontent.com/file"),
            "objects.githubusercontent.com"
        );
        assert_eq!(host_of("sin-esquema"), UPDATE_HOST);
    }

    #[test]
    fn details_are_truncated_for_storage() {
        let long = "x".repeat(800);
        assert_eq!(truncate_detail(&long).len(), 500);
        assert_eq!(truncate_detail("corto"), "corto");
    }

    #[test]
    fn snap_takes_precedence_and_disables_appimage_updates() {
        assert_eq!(runtime_from(false, false), Runtime::Development);
        assert_eq!(runtime_from(true, false), Runtime::AppImage);
        assert_eq!(runtime_from(false, true), Runtime::Snap);
        assert_eq!(runtime_from(true, true), Runtime::Snap);
    }
}
