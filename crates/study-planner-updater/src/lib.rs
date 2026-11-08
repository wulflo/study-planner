//! Núcleo del updater aislado.
//!
//! Este crate es la única parte de Study Planner que puede hablar con la red y
//! lo hace con reglas deliberadamente estrictas:
//!
//! - URL del manifiesto fija en compilación;
//! - solo HTTPS y hosts de confianza;
//! - versión semver creciente;
//! - SHA-256 obligatorio del artefacto;
//! - firma minisign obligatoria del artefacto;
//! - la firma debe declarar la misma versión que el manifiesto.
//!
//! La descarga y la instalación viven en el paso siguiente; acá solo se
//! validan datos para poder probar la seguridad sin tocar la red.

use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Endpoint fijo del manifiesto. Nunca se lee de settings ni de argumentos.
pub const UPDATE_URL: &str =
    "https://github.com/wulflo/study-planner/releases/latest/download/latest.json";

/// Plataforma soportada por el updater.
pub const PLATFORM: &str = "linux-x86_64";

/// Tamaño máximo aceptado para el manifiesto.
pub const MANIFEST_MAX_BYTES: usize = 256 * 1024;

/// Tamaño máximo aceptado para el AppImage descargado.
pub const ARTIFACT_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Hosts permitidos para el manifiesto y los artefactos.
///
/// GitHub redirige las descargas de releases a sus CDN; se listan solo los
/// hosts que participan de ese flujo.
pub const ALLOWED_HOSTS: [&str; 3] = [
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

/// Verdadero cuando el módulo está implementado y habilitado.
pub const UPDATER_IMPLEMENTED: bool = true;

/// Error del updater.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UpdateError {
    #[error("el manifiesto supera el tamaño máximo permitido")]
    ManifestTooLarge,
    #[error("manifiesto inválido: {0}")]
    InvalidManifest(String),
    #[error("versión inválida: {0}")]
    InvalidVersion(String),
    #[error("no hay actualización: {current} es igual o mayor que {candidate}")]
    NotNewer { current: String, candidate: String },
    #[error("el manifiesto no incluye la plataforma {0}")]
    UnsupportedPlatform(String),
    #[error("URL no permitida: {0}")]
    InvalidUrl(String),
    #[error("hash SHA-256 inválido")]
    InvalidHash,
    #[error("el hash no coincide: esperado {expected}, obtenido {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("el artefacto supera el tamaño máximo permitido")]
    ArtifactTooLarge,
    #[error("clave pública minisign inválida")]
    InvalidPublicKey,
    #[error("firma minisign inválida")]
    InvalidSignature,
    #[error("la firma no declara una versión")]
    SignatureVersionMissing,
    #[error("la firma declara la versión {signed}, pero el manifiesto anuncia {expected}")]
    SignatureVersionMismatch { signed: String, expected: String },
}

/// Artefacto de una plataforma dentro del manifiesto.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UpdatePlatform {
    pub url: String,
    pub signature: String,
    pub sha256: String,
}

/// Manifiesto de actualización.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub notes: String,
    pub platforms: BTreeMap<String, UpdatePlatform>,
}

/// Actualización validada y lista para mostrar al usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCandidate {
    pub version: String,
    pub pub_date: Option<String>,
    pub notes: String,
    pub url: String,
    pub signature: String,
    pub sha256: String,
}

/// Comprueba si `candidate` es una versión semver mayor que `current`.
pub fn is_newer(current: &str, candidate: &str) -> Result<bool, UpdateError> {
    let current = Version::parse(current.trim())
        .map_err(|_| UpdateError::InvalidVersion(current.to_string()))?;
    let candidate = Version::parse(candidate.trim())
        .map_err(|_| UpdateError::InvalidVersion(candidate.to_string()))?;
    Ok(candidate > current)
}

/// Parsea y valida la estructura del manifiesto.
pub fn parse_manifest(bytes: &[u8]) -> Result<UpdateManifest, UpdateError> {
    if bytes.len() > MANIFEST_MAX_BYTES {
        return Err(UpdateError::ManifestTooLarge);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| UpdateError::InvalidManifest("no es UTF-8 válido".to_string()))?;
    let manifest: UpdateManifest = serde_json::from_str(text)
        .map_err(|error| UpdateError::InvalidManifest(error.to_string()))?;

    Version::parse(manifest.version.trim())
        .map_err(|_| UpdateError::InvalidVersion(manifest.version.clone()))?;

    for (platform, artifact) in &manifest.platforms {
        validate_https_url(&artifact.url)?;
        if artifact.sha256.len() != 64
            || !artifact
                .sha256
                .chars()
                .all(|value| value.is_ascii_hexdigit())
        {
            return Err(UpdateError::InvalidHash);
        }
        if artifact.signature.trim().is_empty() {
            return Err(UpdateError::InvalidManifest(format!(
                "la plataforma {platform} no tiene firma"
            )));
        }
    }

    Ok(manifest)
}

/// Devuelve la actualización para Linux si es más nueva que `current_version`.
///
/// `Ok(None)` significa «ya estás al día»; no es un error.
pub fn evaluate_manifest(
    bytes: &[u8],
    current_version: &str,
) -> Result<Option<UpdateCandidate>, UpdateError> {
    let manifest = parse_manifest(bytes)?;
    if !is_newer(current_version, &manifest.version)? {
        return Ok(None);
    }
    let artifact = manifest
        .platforms
        .get(PLATFORM)
        .ok_or_else(|| UpdateError::UnsupportedPlatform(PLATFORM.to_string()))?;

    Ok(Some(UpdateCandidate {
        version: manifest.version.trim().to_string(),
        pub_date: manifest.pub_date,
        notes: manifest.notes.chars().take(2_000).collect(),
        url: artifact.url.clone(),
        signature: artifact.signature.clone(),
        sha256: artifact.sha256.to_ascii_lowercase(),
    }))
}

/// Calcula el SHA-256 en hexadecimal minúscula.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verifica tamaño, SHA-256, firma minisign y versión firmada.
pub fn verify_artifact(
    public_key: &str,
    artifact: &[u8],
    signature: &str,
    expected_version: &str,
    expected_sha256: &str,
) -> Result<(), UpdateError> {
    if artifact.len() as u64 > ARTIFACT_MAX_BYTES {
        return Err(UpdateError::ArtifactTooLarge);
    }

    let actual = sha256_hex(artifact);
    if actual != expected_sha256.to_ascii_lowercase() {
        return Err(UpdateError::HashMismatch {
            expected: expected_sha256.to_ascii_lowercase(),
            actual,
        });
    }

    let public_key = decode_public_key(public_key)?;
    let signature = decode_signature(signature)?;

    public_key
        .verify(artifact, &signature, false)
        .map_err(|_| UpdateError::InvalidSignature)?;

    let signed_version = version_from_trusted_comment(signature.trusted_comment())
        .ok_or(UpdateError::SignatureVersionMissing)?;
    if signed_version != expected_version.trim() {
        return Err(UpdateError::SignatureVersionMismatch {
            signed: signed_version.to_string(),
            expected: expected_version.trim().to_string(),
        });
    }

    Ok(())
}

/// Acepta tanto el texto minisign como el base64 que produce `tauri signer`.
pub fn decode_public_key(raw: &str) -> Result<PublicKey, UpdateError> {
    let trimmed = raw.trim();
    if trimmed.starts_with("untrusted comment") {
        return PublicKey::decode(trimmed).map_err(|_| UpdateError::InvalidPublicKey);
    }

    if let Ok(decoded) = STANDARD.decode(trimmed)
        && let Ok(text) = String::from_utf8(decoded)
        && text.contains("untrusted comment")
    {
        return PublicKey::decode(&text).map_err(|_| UpdateError::InvalidPublicKey);
    }

    PublicKey::from_base64(trimmed).map_err(|_| UpdateError::InvalidPublicKey)
}

/// Acepta tanto el texto minisign como el base64 que produce `tauri signer`.
pub fn decode_signature(raw: &str) -> Result<Signature, UpdateError> {
    let trimmed = raw.trim();
    if trimmed.starts_with("untrusted comment") {
        return Signature::decode(trimmed).map_err(|_| UpdateError::InvalidSignature);
    }

    let decoded = STANDARD
        .decode(trimmed)
        .map_err(|_| UpdateError::InvalidSignature)?;
    let text = String::from_utf8(decoded).map_err(|_| UpdateError::InvalidSignature)?;
    Signature::decode(&text).map_err(|_| UpdateError::InvalidSignature)
}

fn version_from_trusted_comment(comment: &str) -> Option<&str> {
    comment
        .split_whitespace()
        .find_map(|token| token.strip_prefix("version:"))
        .filter(|value| !value.is_empty())
}

fn validate_https_url(raw: &str) -> Result<(), UpdateError> {
    let value = raw.trim();
    let rest = value
        .strip_prefix("https://")
        .ok_or_else(|| UpdateError::InvalidUrl(raw.to_string()))?;
    if rest.is_empty() || rest.contains('@') || rest.contains(' ') || rest.contains('#') {
        return Err(UpdateError::InvalidUrl(raw.to_string()));
    }
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty() {
        return Err(UpdateError::InvalidUrl(raw.to_string()));
    }
    let host = authority.split(':').next().unwrap_or_default();
    if !ALLOWED_HOSTS.contains(&host) {
        return Err(UpdateError::InvalidUrl(raw.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clave pública de test generada con `tauri signer`; no es secreta.
    const TEST_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDNGNTU1NTE1NjgxMjg4MjQKUldRa2lCSm9GVlZWUHpqTElyM29hbnlXdzN5b2t2aUFtSWFzNEw2MWhFeUNtVzQvWmJYR2Z6VHQK";

    /// Firma de `artifact-bytes` con versión 0.2.0; tampoco es secreta.
    const TEST_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRa2lCSm9GVlZWUC9PSDJPQ1VwazdjSWdaT244dVU5TzFxcXpnNE8ySUhjajJtLzZQbVY2R01Gc3EzNjExaTVBQnJ1Qy9DZjJobDdxSVJoWlNTMkdKRVVmMlRmSzdlcHdNPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0MTY1MzU1CWZpbGU6YXJ0aWZhY3QuYmluCXZlcnNpb246MC4yLjAKemd6bmpLWHIxbVQxM3FTeUdPWWw5NmNEQUJtSGtGMWdPSkhzRnZmUkJqUXZiOWxRSEdLcHpITGF4NmZDVUUxQjFGVElhcHB4SW5lVU1CNlgzMFFmRHc9PQo=";

    const TEST_SHA256: &str = "6521df166eb07efaf36eba5b6bedefd9d6a252e9c80bab1c99653700ec71473c";
    const TEST_ARTIFACT: &[u8] = b"artifact-bytes";

    fn manifest(version: &str) -> String {
        format!(
            r#"{{
                "version": "{version}",
                "pub_date": "2026-12-01T00:00:00Z",
                "notes": "Cambios de la versión",
                "platforms": {{
                    "linux-x86_64": {{
                        "url": "https://github.com/wulflo/study-planner/releases/download/v{version}/Study_Planner.AppImage",
                        "signature": "{TEST_SIGNATURE}",
                        "sha256": "{TEST_SHA256}"
                    }}
                }}
            }}"#
        )
    }

    #[test]
    fn validates_a_newer_signed_manifest() {
        let candidate = evaluate_manifest(manifest("0.2.0").as_bytes(), "0.1.0")
            .expect("manifiesto válido")
            .expect("actualización disponible");

        assert_eq!(candidate.version, "0.2.0");
        assert_eq!(candidate.sha256, TEST_SHA256);
        assert!(candidate.url.starts_with("https://github.com/"));
        assert!(candidate.notes.contains("Cambios"));
    }

    #[test]
    fn refuses_same_or_older_versions() {
        assert!(
            evaluate_manifest(manifest("0.1.0").as_bytes(), "0.1.0")
                .expect("manifiesto válido")
                .is_none()
        );
        assert!(
            evaluate_manifest(manifest("0.0.9").as_bytes(), "0.1.0")
                .expect("manifiesto válido")
                .is_none()
        );
    }

    #[test]
    fn rejects_insecure_or_foreign_urls() {
        for url in [
            "http://github.com/wulflo/study-planner/releases/download/v0.2.0/app.AppImage",
            "https://evil.example/app.AppImage",
            "https://github.com.evil.example/app.AppImage",
            "https://user@github.com/app.AppImage",
            "https://github.com/app.AppImage#fragment",
        ] {
            let payload = manifest("0.2.0").replace(
                "https://github.com/wulflo/study-planner/releases/download/v0.2.0/Study_Planner.AppImage",
                url,
            );
            assert!(
                matches!(
                    evaluate_manifest(payload.as_bytes(), "0.1.0"),
                    Err(UpdateError::InvalidUrl(_))
                ),
                "debió rechazar {url}"
            );
        }
    }

    #[test]
    fn rejects_invalid_hashes_and_missing_platform() {
        let payload = manifest("0.2.0").replace(TEST_SHA256, "1234");
        assert_eq!(
            evaluate_manifest(payload.as_bytes(), "0.1.0"),
            Err(UpdateError::InvalidHash)
        );

        let payload = manifest("0.2.0").replace("linux-x86_64", "windows-x86_64");
        assert_eq!(
            evaluate_manifest(payload.as_bytes(), "0.1.0"),
            Err(UpdateError::UnsupportedPlatform(PLATFORM.to_string()))
        );
    }

    #[test]
    fn rejects_invalid_semver_and_oversized_manifests() {
        let payload = manifest("no-es-semver");
        assert!(matches!(
            evaluate_manifest(payload.as_bytes(), "0.1.0"),
            Err(UpdateError::InvalidVersion(_))
        ));

        let oversized = vec![b' '; MANIFEST_MAX_BYTES + 1];
        assert_eq!(
            parse_manifest(&oversized),
            Err(UpdateError::ManifestTooLarge)
        );
    }

    #[test]
    fn verifies_artifact_hash_signature_and_version() {
        verify_artifact(
            TEST_PUBLIC_KEY,
            TEST_ARTIFACT,
            TEST_SIGNATURE,
            "0.2.0",
            TEST_SHA256,
        )
        .expect("artefacto válido");

        assert_eq!(sha256_hex(TEST_ARTIFACT), TEST_SHA256);

        assert!(matches!(
            verify_artifact(
                TEST_PUBLIC_KEY,
                b"tampered",
                TEST_SIGNATURE,
                "0.2.0",
                TEST_SHA256
            ),
            Err(UpdateError::HashMismatch { .. })
        ));
        assert!(matches!(
            verify_artifact(
                TEST_PUBLIC_KEY,
                TEST_ARTIFACT,
                TEST_SIGNATURE,
                "0.3.0",
                TEST_SHA256
            ),
            Err(UpdateError::SignatureVersionMismatch { .. })
        ));
    }

    #[test]
    fn rejects_a_tampered_signature() {
        let raw = String::from_utf8(STANDARD.decode(TEST_SIGNATURE).unwrap()).unwrap();
        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
        // Alterar la línea que contiene la firma; el resto queda intacto.
        let signature_line = &mut lines[1];
        let first = signature_line.remove(0);
        signature_line.insert(0, if first == 'R' { 'S' } else { 'R' });
        let tampered = STANDARD.encode(lines.join("\n"));

        assert!(matches!(
            verify_artifact(
                TEST_PUBLIC_KEY,
                TEST_ARTIFACT,
                &tampered,
                "0.2.0",
                TEST_SHA256
            ),
            Err(UpdateError::InvalidSignature)
        ));
    }

    #[test]
    fn accepts_raw_minisign_text_as_well_as_base64() {
        let raw_public = String::from_utf8(STANDARD.decode(TEST_PUBLIC_KEY).unwrap()).unwrap();
        let raw_signature = String::from_utf8(STANDARD.decode(TEST_SIGNATURE).unwrap()).unwrap();

        verify_artifact(
            &raw_public,
            TEST_ARTIFACT,
            &raw_signature,
            "0.2.0",
            TEST_SHA256,
        )
        .expect("formato minisign crudo");
    }

    #[test]
    fn update_url_is_fixed_and_secure() {
        assert!(UPDATE_URL.starts_with("https://github.com/"));
        validate_https_url(UPDATE_URL).expect("URL fija válida");
    }
}
