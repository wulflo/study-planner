//! Núcleo de dominio de Omega.
//!
//! Este crate no depende de Tauri, de la red ni del sistema de archivos.
//! Contiene constantes compartidas, identificadores y, más adelante, las
//! reglas puras del dominio (recurrencias, notas, prioridad de estudio).

use serde::Serialize;
use uuid::Uuid;

/// Nombre visible de la aplicación.
pub const APP_NAME: &str = "Omega";

/// Versión de la aplicación tomada del paquete Cargo.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versión del esquema SQLite. Se incrementa con cada migración.
pub const SCHEMA_VERSION: u32 = 1;

/// Versión del formato de exportación `.txt`.
pub const EXPORT_FORMAT_VERSION: u32 = 1;

/// Información estable que la UI puede mostrar sin tocar la bóveda.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub schema_version: u32,
    pub export_format_version: u32,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            name: APP_NAME,
            version: APP_VERSION,
            schema_version: SCHEMA_VERSION,
            export_format_version: EXPORT_FORMAT_VERSION,
        }
    }
}

/// Información de la app para el frontend.
pub fn app_info() -> AppInfo {
    AppInfo::default()
}

/// Genera un identificador UUIDv7, ordenable por tiempo.
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_is_stable() {
        let info = AppInfo::default();
        assert_eq!(info.name, "Omega");
        assert_eq!(info.schema_version, SCHEMA_VERSION);
        assert_eq!(info.export_format_version, EXPORT_FORMAT_VERSION);
    }

    #[test]
    fn ids_are_uuid_v7_and_unique() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        let parsed = Uuid::parse_str(&a).expect("uuid válido");
        assert_eq!(parsed.get_version_num(), 7);
    }
}
