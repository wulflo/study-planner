//! Apertura del repositorio público en el navegador del sistema.
//!
//! La URL es una constante de compilación: la webview no la elige ni la recibe.
//! Se invoca el abridor del sistema desde Rust, sin plugins de Tauri ni
//! permisos de shell para el frontend.

use std::process::{Child, Command};

/// Repositorio público del proyecto.
const REPOSITORY_URL: &str = "https://github.com/wulflo/StudyPlanner";

/// Abre el repositorio en el navegador predeterminado del escritorio.
#[tauri::command]
pub fn open_repository() -> Result<(), String> {
    spawn_opener(REPOSITORY_URL)
        .map(|_: Child| ())
        .map_err(|error| format!("no se pudo abrir el repositorio: {error}"))
}

#[cfg(target_os = "linux")]
fn spawn_opener(url: &str) -> std::io::Result<Child> {
    Command::new("xdg-open").arg(url).spawn()
}

#[cfg(target_os = "macos")]
fn spawn_opener(url: &str) -> std::io::Result<Child> {
    Command::new("open").arg(url).spawn()
}

#[cfg(target_os = "windows")]
fn spawn_opener(url: &str) -> std::io::Result<Child> {
    Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn spawn_opener(_url: &str) -> std::io::Result<Child> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "plataforma no soportada",
    ))
}
