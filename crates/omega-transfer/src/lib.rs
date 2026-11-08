//! Formato de exportación total de Omega.
//!
//! La implementación completa del codificador canónico y del importador
//! corresponde a la Fase 1. En la Fase 0 se fija el contrato y las constantes.

use thiserror::Error;

/// Primera línea obligatoria del archivo de exportación.
pub const EXPORT_MAGIC: &str = "OMEGA-EXPORT";

/// Versión del formato de archivo. Es independiente de `schema_version`.
pub const EXPORT_FORMAT_VERSION: u32 = omega_core::EXPORT_FORMAT_VERSION;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("formato de exportación inválido: {0}")]
    InvalidFormat(String),
    #[error("checksum inválido: esperado {expected}, obtenido {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("json inválido: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_y_version_son_estables() {
        assert_eq!(EXPORT_MAGIC, "OMEGA-EXPORT");
        assert_eq!(EXPORT_FORMAT_VERSION, 1);
    }
}
