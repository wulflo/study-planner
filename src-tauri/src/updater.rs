//! Comandos Tauri del updater aislado.
//!
//! La webview no recibe URLs, firmas ni rutas configurables: solo invoca
//! operaciones sin argumentos de red y recibe un resumen listo para mostrar.
//! Toda la lógica sensible vive en `study-planner-updater` y en Rust.

use std::path::PathBuf;

use serde::Serialize;
use study_planner_store::{NetworkEvent, Store};
use study_planner_updater::{
    ReqwestFetcher, Runtime, UPDATE_HOST, UpdateCandidate, UpdateError, appimage_backup_path,
    check_for_update_for_platforms, detect_appimage, detect_runtime, download_update,
    install_update_artifact, platform_keys, public_key_configured, rollback_appimage,
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
    pub runtime: String,
    /// El frontend espera `runningAsAppImage` con «I» mayúscula; el
    /// `rename_all` de serde generaría `runningAsAppimage`.
    #[serde(rename = "runningAsAppImage")]
    pub running_as_appimage: bool,
    pub managed_by_snap: bool,
    pub can_install: bool,
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

/// Ruta del AppImage solo cuando el entorno es realmente AppImage.
fn appimage_path(runtime: Runtime) -> Result<Option<PathBuf>, String> {
    if runtime == Runtime::Snap {
        return Err(UpdateError::ManagedBySnap.to_string());
    }
    if runtime != Runtime::AppImage {
        return Ok(None);
    }
    detect_appimage()
        .map(Some)
        .ok_or_else(|| UpdateError::NotAppImage.to_string())
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

/// Registra un evento de red sin exigir una bóveda abierta.
fn log_event(
    state: &AppState,
    kind: &str,
    host: &str,
    bytes_sent: i64,
    bytes_received: i64,
    outcome: &str,
    detail: Option<&str>,
) {
    let _ = state.with_store_opt(|store| {
        if let Some(store) = store {
            record_event(
                store,
                kind,
                host,
                bytes_sent,
                bytes_received,
                outcome,
                detail,
            );
        }
        Ok(())
    });
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
    let auto_check = state.with_store_opt(|store| {
        store.map_or(Ok(false), |store| read_bool(store, AUTO_CHECK_KEY))
    })?;
    let runtime = detect_runtime();
    let appimage = if runtime == Runtime::AppImage {
        detect_appimage()
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
        runtime: runtime.as_str().to_string(),
        running_as_appimage: runtime == Runtime::AppImage,
        managed_by_snap: runtime == Runtime::Snap,
        can_install: runtime.can_install(),
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
    let runtime = detect_runtime();
    if runtime == Runtime::Snap {
        return Err(UpdateError::ManagedBySnap.to_string());
    }
    if !runtime.can_install() {
        return Err(UpdateError::UnsupportedRuntime.to_string());
    }
    if !public_key_configured() {
        return Err(UpdateError::PublicKeyNotConfigured.to_string());
    }

    let version = current_version();
    let fetcher = ReqwestFetcher::new(&version).map_err(|error| error.to_string())?;
    let platforms = platform_keys(runtime);
    let result = check_for_update_for_platforms(&fetcher, &version, &platforms);
    let bytes_received = result
        .as_ref()
        .map_or(0, |check| check.manifest_bytes as i64);

    match &result {
        Ok(check) => match &check.candidate {
            Some(candidate) => log_event(
                state,
                "update_check",
                UPDATE_HOST,
                0,
                bytes_received,
                "ok",
                Some(&format!("actualización {} disponible", candidate.version)),
            ),
            None => log_event(
                state,
                "update_check",
                UPDATE_HOST,
                0,
                bytes_received,
                "ok",
                Some("sin actualizaciones"),
            ),
        },
        Err(error) => log_event(
            state,
            "update_check",
            UPDATE_HOST,
            0,
            bytes_received,
            "error",
            Some(&error.to_string()),
        ),
    }

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

    let runtime = detect_runtime();
    if !runtime.can_install() {
        return;
    }

    let state = app.state::<AppState>();
    let enabled = state
        .with_store_opt(|store| store.map_or(Ok(false), |store| read_bool(store, AUTO_CHECK_KEY)))
        .unwrap_or(false);
    if enabled && public_key_configured() {
        let _ = run_check(&state);
    }
}

/// Descarga, verifica e instala. Vuelve a consultar el manifiesto para no
/// aceptar URL ni firma desde la webview.
#[tauri::command]
pub fn install_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<InstallOutcome, String> {
    if !public_key_configured() {
        return Err(UpdateError::PublicKeyNotConfigured.to_string());
    }
    let runtime = detect_runtime();
    if runtime == Runtime::Snap {
        return Err(UpdateError::ManagedBySnap.to_string());
    }
    if !runtime.can_install() {
        return Err(UpdateError::UnsupportedRuntime.to_string());
    }
    let target = appimage_path(runtime)?;
    let version = current_version();
    let fetcher = ReqwestFetcher::new(&version).map_err(|error| error.to_string())?;
    let platforms = platform_keys(runtime);

    let check =
        check_for_update_for_platforms(&fetcher, &version, &platforms).map_err(|error| {
            log_event(
                &state,
                "update_check",
                UPDATE_HOST,
                0,
                0,
                "error",
                Some(&error.to_string()),
            );
            error.to_string()
        })?;

    let candidate = check
        .candidate
        .ok_or_else(|| "No hay una actualización disponible para instalar.".to_string())?;

    log_event(
        &state,
        "update_check",
        UPDATE_HOST,
        0,
        check.manifest_bytes as i64,
        "ok",
        Some(&format!("actualización {} disponible", candidate.version)),
    );

    let artifact = match download_update(
        &fetcher,
        &candidate,
        study_planner_updater::UPDATE_PUBLIC_KEY,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            log_event(
                &state,
                "update_download",
                host_of(&candidate.url),
                0,
                0,
                "error",
                Some(&error.to_string()),
            );
            return Err(error.to_string());
        }
    };

    log_event(
        &state,
        "update_download",
        host_of(&candidate.url),
        0,
        artifact.len() as i64,
        "ok",
        Some(&format!("artefacto {} verificado", candidate.version)),
    );

    install_update_artifact(&artifact, runtime, &candidate.platform, target.as_deref()).map_err(
        |error| {
            log_event(
                &state,
                "update_install",
                UPDATE_HOST,
                0,
                0,
                "error",
                Some(&error.to_string()),
            );
            error.to_string()
        },
    )?;

    log_event(
        &state,
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

    if matches!(runtime, Runtime::Windows | Runtime::MacOs) {
        schedule_restart(app);
    }

    Ok(InstallOutcome {
        version: candidate.version,
        restart_required: true,
    })
}

/// En Windows y macOS el instalador necesita que la app libere sus archivos.
fn schedule_restart(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1_500));
        app.exit(0);
    });
}

#[tauri::command]
pub fn rollback_update(state: State<'_, AppState>) -> Result<(), String> {
    let runtime = detect_runtime();
    let target =
        appimage_path(runtime)?.ok_or_else(|| UpdateError::UnsupportedRuntime.to_string())?;
    rollback_appimage(&target).map_err(|error| error.to_string())?;
    log_event(
        &state,
        "update_rollback",
        UPDATE_HOST,
        0,
        0,
        "ok",
        Some("versión anterior restaurada"),
    );
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
    fn only_packaged_runtimes_can_install() {
        assert!(Runtime::AppImage.can_install());
        assert!(Runtime::Deb.can_install());
        assert!(Runtime::Rpm.can_install());
        assert!(Runtime::Windows.can_install());
        assert!(Runtime::MacOs.can_install());
        assert!(!Runtime::Snap.can_install());
        assert!(!Runtime::Development.can_install());
    }

    #[test]
    fn platform_keys_are_specific_to_the_runtime() {
        let arch = study_planner_updater::arch_name();
        assert_eq!(
            platform_keys(Runtime::AppImage),
            vec![format!("linux-{arch}")]
        );
        assert_eq!(
            platform_keys(Runtime::Deb),
            vec![format!("linux-{arch}-deb")]
        );
        assert_eq!(
            platform_keys(Runtime::Rpm),
            vec![format!("linux-{arch}-rpm")]
        );
        assert_eq!(
            platform_keys(Runtime::MacOs),
            vec![format!("darwin-{arch}")]
        );
        assert!(platform_keys(Runtime::Snap).is_empty());
        assert!(platform_keys(Runtime::Development).is_empty());
    }

    #[test]
    fn status_keeps_the_camel_case_appimage_key() {
        let status = UpdaterStatus {
            public_key_configured: true,
            current_version: "0.1.3".to_string(),
            update_host: UPDATE_HOST.to_string(),
            auto_check: false,
            runtime: "appimage".to_string(),
            running_as_appimage: true,
            managed_by_snap: false,
            can_install: true,
            can_rollback: false,
        };
        let value = serde_json::to_value(status).expect("serializar estado");
        assert_eq!(value["runningAsAppImage"], true);
        assert_eq!(value["runtime"], "appimage");
        assert!(value.get("runningAsAppimage").is_none());
    }
}
