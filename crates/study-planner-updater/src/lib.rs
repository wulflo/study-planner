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
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(any(target_os = "linux", windows, target_os = "macos"))]
use std::process::Command;

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

/// Host del endpoint fijo.
pub const UPDATE_HOST: &str = "github.com";

/// Clave pública minisign embebida al compilar.
///
/// Se reemplaza `updater.pub` cuando existe la clave de producción. Mientras
/// el archivo solo tenga comentarios, el updater está deshabilitado.
pub const UPDATE_PUBLIC_KEY: &str = include_str!("../updater.pub");

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

/// Verdadero cuando `updater.pub` contiene una clave real y no solo comentarios.
pub fn public_key_configured() -> bool {
    let value = UPDATE_PUBLIC_KEY.trim();
    !value.is_empty() && !value.starts_with('#')
}

/// Entorno desde el que corre la app, relevante para actualizarla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    /// AppImage: el propio binario se reemplaza de forma atómica.
    AppImage,
    /// Snap: snapd gestiona las actualizaciones.
    Snap,
    /// Paquete `.deb` instalado por el sistema.
    Deb,
    /// Paquete `.rpm` instalado por el sistema.
    Rpm,
    /// Instalación de Windows (NSIS o MSI).
    Windows,
    /// Paquete MSIX de Windows (Microsoft Store o sideloading).
    Msix,
    /// Aplicación de macOS instalada como `.app`.
    #[serde(rename = "macos")]
    MacOs,
    /// `tauri dev` u otro entorno sin paquete publicado.
    Development,
}

impl Runtime {
    pub fn as_str(self) -> &'static str {
        match self {
            Runtime::AppImage => "appimage",
            Runtime::Snap => "snap",
            Runtime::Deb => "deb",
            Runtime::Rpm => "rpm",
            Runtime::Windows => "windows",
            Runtime::Msix => "msix",
            Runtime::MacOs => "macos",
            Runtime::Development => "development",
        }
    }

    /// Solo estos entornos pueden descargar e instalar desde el manifiesto.
    pub fn can_install(self) -> bool {
        matches!(
            self,
            Runtime::AppImage | Runtime::Deb | Runtime::Rpm | Runtime::Windows | Runtime::MacOs
        )
    }
}

/// Detecta el entorno de ejecución sin aceptar configuración externa.
pub fn detect_runtime() -> Runtime {
    if std::env::var_os("SNAP").is_some() {
        return Runtime::Snap;
    }
    if detect_appimage().is_some() {
        return Runtime::AppImage;
    }

    #[cfg(windows)]
    {
        if detect_msix() {
            return Runtime::Msix;
        }
        return Runtime::Windows;
    }

    #[cfg(target_os = "macos")]
    {
        return Runtime::MacOs;
    }

    #[cfg(target_os = "linux")]
    {
        return detect_linux_package();
    }

    #[allow(unreachable_code)]
    Runtime::Development
}

/// Detecta un paquete MSIX por la ubicación del ejecutable.
///
/// Las apps empaquetadas con MSIX corren desde `C:\Program Files\WindowsApps\...`.
/// Es la señal más simple y evita depender de una llamada Win32.
#[cfg(windows)]
fn detect_msix() -> bool {
    std::env::current_exe()
        .map(|path| {
            path.to_string_lossy()
                .to_ascii_lowercase()
                .contains("windowsapps")
        })
        .unwrap_or(false)
}

fn is_appimage_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("appimage"))
}

/// Resuelve la ruta del AppImage en ejecución.
///
/// El runtime de AppImage define `APPIMAGE`. Algunos lanzadores solo dejan
/// `ARGV0`; como último recurso se revisa el ejecutable actual.
pub fn detect_appimage() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os("APPIMAGE").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(raw));
    }

    if let Some(raw) = std::env::var_os("ARGV0").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(raw);
        if is_appimage_path(&path) {
            return Some(path);
        }
    }

    let executable = std::env::current_exe().ok()?;
    is_appimage_path(&executable).then_some(executable)
}

#[cfg(target_os = "linux")]
fn detect_linux_package() -> Runtime {
    let Ok(executable) = std::env::current_exe() else {
        return Runtime::Development;
    };
    let executable = executable.to_string_lossy();

    if command_succeeds("dpkg-query", &["-S", &executable]) {
        return Runtime::Deb;
    }
    if command_succeeds("rpm", &["-qf", &executable]) {
        return Runtime::Rpm;
    }

    Runtime::Development
}

#[cfg(target_os = "linux")]
fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "linux")]
fn command_exists(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(program).is_file())
}

/// Nombre de arquitectura usado en las claves del manifiesto.
pub fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "x86" => "i686",
        other => other,
    }
}

/// Claves del manifiesto que acepta este entorno, en orden de preferencia.
pub fn platform_keys(runtime: Runtime) -> Vec<String> {
    let arch = arch_name();
    match runtime {
        Runtime::AppImage => vec![format!("linux-{arch}")],
        Runtime::Deb => vec![format!("linux-{arch}-deb")],
        Runtime::Rpm => vec![format!("linux-{arch}-rpm")],
        Runtime::Windows => windows_platform_keys(arch),
        Runtime::MacOs => vec![format!("darwin-{arch}")],
        Runtime::Snap | Runtime::Msix | Runtime::Development => Vec::new(),
    }
}

fn windows_platform_keys(arch: &str) -> Vec<String> {
    // NSIS instala por usuario en %LOCALAPPDATA%; MSI lo hace por máquina en
    // %ProgramFiles%. Si no se puede distinguir, se prefiere NSIS.
    let prefer_msi = cfg!(windows) && executable_under_program_files();
    if prefer_msi {
        vec![format!("windows-{arch}-msi"), format!("windows-{arch}")]
    } else {
        vec![format!("windows-{arch}"), format!("windows-{arch}-msi")]
    }
}

fn executable_under_program_files() -> bool {
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    let executable = executable.to_string_lossy().to_lowercase();
    ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|value| value.to_lowercase())
        .any(|root| {
            !root.is_empty()
                && executable.starts_with(&format!("{}\\", root.trim_end_matches('\\')))
        })
}

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
    #[error("error de red: {0}")]
    Http(String),
    #[error("respuesta HTTP {0}")]
    HttpStatus(u16),
    #[error("la respuesta supera el tamaño máximo permitido")]
    ResponseTooLarge,
    #[error("error de instalación: {0}")]
    Install(String),
    #[error("la clave pública de actualizaciones no está configurada")]
    PublicKeyNotConfigured,
    #[error("las actualizaciones solo se instalan desde un AppImage")]
    NotAppImage,
    #[error("esta instalación no admite actualizaciones automáticas")]
    UnsupportedRuntime,
    #[error("esta instalación se actualiza desde Snap; el updater de AppImage está deshabilitado")]
    ManagedBySnap,
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
    pub platform: String,
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
    evaluate_manifest_for_platforms(bytes, current_version, &[PLATFORM.to_string()])
}

/// Igual que [`evaluate_manifest`], pero elige la primera plataforma disponible.
///
/// El orden de `platforms` define la preferencia (por ejemplo MSI antes que
/// NSIS en Windows). Si ninguna clave existe, se informa como no soportada.
pub fn evaluate_manifest_for_platforms(
    bytes: &[u8],
    current_version: &str,
    platforms: &[String],
) -> Result<Option<UpdateCandidate>, UpdateError> {
    let manifest = parse_manifest(bytes)?;
    if !is_newer(current_version, &manifest.version)? {
        return Ok(None);
    }
    let (platform, artifact) = platforms
        .iter()
        .find_map(|platform| {
            manifest
                .platforms
                .get(platform)
                .map(|artifact| (platform, artifact))
        })
        .ok_or_else(|| UpdateError::UnsupportedPlatform(platforms.join(", ")))?;

    Ok(Some(UpdateCandidate {
        version: manifest.version.trim().to_string(),
        pub_date: manifest.pub_date,
        notes: manifest.notes.chars().take(2_000).collect(),
        platform: platform.clone(),
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

/// Transporte HTTP abstracto para poder probar la lógica sin red.
pub trait HttpFetcher {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, UpdateError>;
}

/// Cliente HTTP real del updater.
///
/// Solo se usa desde Rust; no se expone en la webview ni acepta URLs
/// configurables. Las redirecciones se limitan a los hosts permitidos.
pub struct ReqwestFetcher {
    client: reqwest::blocking::Client,
}

impl ReqwestFetcher {
    /// Construye el cliente con un `User-Agent` que solo incluye la versión.
    pub fn new(app_version: &str) -> Result<Self, UpdateError> {
        let user_agent = format!("StudyPlanner/{}", app_version.trim());
        let client = reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(45))
            .connect_timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("demasiadas redirecciones")
                } else if !allowed_https_url(attempt.url()) {
                    attempt.error("redirección a un host no permitido")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|error| UpdateError::Http(error.to_string()))?;
        Ok(Self { client })
    }
}

impl HttpFetcher for ReqwestFetcher {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, UpdateError> {
        validate_https_url(url)?;
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|error| UpdateError::Http(error.to_string()))?;
        if !response.status().is_success() {
            return Err(UpdateError::HttpStatus(response.status().as_u16()));
        }
        if let Some(length) = response.content_length()
            && length > max_bytes as u64
        {
            return Err(UpdateError::ResponseTooLarge);
        }

        let mut bytes = Vec::new();
        response
            .take(max_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| UpdateError::Http(error.to_string()))?;
        if bytes.len() > max_bytes {
            return Err(UpdateError::ResponseTooLarge);
        }
        Ok(bytes)
    }
}

/// Resultado de consultar el manifiesto fijo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub candidate: Option<UpdateCandidate>,
    /// Bytes recibidos del manifiesto; se registran en `network_event`.
    pub manifest_bytes: usize,
}

/// Consulta el manifiesto fijo y devuelve la actualización si es más nueva.
pub fn check_for_update(
    fetcher: &dyn HttpFetcher,
    current_version: &str,
) -> Result<Option<UpdateCandidate>, UpdateError> {
    Ok(check_for_update_with_size(fetcher, current_version)?.candidate)
}

/// Igual que [`check_for_update`], pero informa el tamaño del manifiesto.
pub fn check_for_update_with_size(
    fetcher: &dyn HttpFetcher,
    current_version: &str,
) -> Result<UpdateCheck, UpdateError> {
    check_for_update_for_platforms(fetcher, current_version, &[PLATFORM.to_string()])
}

/// Consulta el manifiesto fijo y elige la primera plataforma disponible.
pub fn check_for_update_for_platforms(
    fetcher: &dyn HttpFetcher,
    current_version: &str,
    platforms: &[String],
) -> Result<UpdateCheck, UpdateError> {
    let bytes = fetcher.fetch(UPDATE_URL, MANIFEST_MAX_BYTES)?;
    let manifest_bytes = bytes.len();
    Ok(UpdateCheck {
        candidate: evaluate_manifest_for_platforms(&bytes, current_version, platforms)?,
        manifest_bytes,
    })
}

/// Descarga el artefacto y lo verifica antes de devolverlo.
pub fn download_update(
    fetcher: &dyn HttpFetcher,
    candidate: &UpdateCandidate,
    public_key: &str,
) -> Result<Vec<u8>, UpdateError> {
    validate_https_url(&candidate.url)?;
    let max_bytes =
        usize::try_from(ARTIFACT_MAX_BYTES).map_err(|_| UpdateError::ResponseTooLarge)?;
    let bytes = fetcher.fetch(&candidate.url, max_bytes)?;
    verify_artifact(
        public_key,
        &bytes,
        &candidate.signature,
        &candidate.version,
        &candidate.sha256,
    )?;
    Ok(bytes)
}

/// Resultado de instalar un AppImage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub installed: PathBuf,
    pub backup: PathBuf,
}

/// Reemplaza el AppImage actual de forma atómica y conserva el anterior.
///
/// El archivo nuevo se escribe y sincroniza al lado del actual. Solo después
/// se mueve el actual a `.old` y el nuevo a su lugar. Si el segundo renombre
/// falla, se restaura el anterior.
pub fn install_appimage(appimage: &[u8], target: &Path) -> Result<InstallReport, UpdateError> {
    if appimage.is_empty() {
        return Err(UpdateError::Install(
            "el AppImage descargado está vacío".to_string(),
        ));
    }
    if !target.is_file() {
        return Err(UpdateError::Install(format!(
            "el AppImage actual no existe: {}",
            target.display()
        )));
    }

    let temporary = sibling_path(target, "new");
    let backup = backup_path(target);
    remove_if_exists(&temporary)?;

    {
        let mut file = fs::File::create(&temporary).map_err(install_error)?;
        file.write_all(appimage).map_err(install_error)?;
        file.sync_all().map_err(install_error)?;
    }
    set_executable(&temporary)?;

    remove_if_exists(&backup)?;
    fs::rename(target, &backup).map_err(install_error)?;
    if let Err(error) = fs::rename(&temporary, target) {
        // Restaurar el binario anterior si el reemplazo no se pudo completar.
        let _ = fs::rename(&backup, target);
        let _ = fs::remove_file(&temporary);
        return Err(install_error(error));
    }

    Ok(InstallReport {
        installed: target.to_path_buf(),
        backup,
    })
}

/// Restaura el AppImage anterior guardado por [`install_appimage`].
///
/// El binario fallido se conserva como `.failed` para diagnóstico.
pub fn rollback_appimage(target: &Path) -> Result<PathBuf, UpdateError> {
    let backup = backup_path(target);
    if !backup.is_file() {
        return Err(UpdateError::Install(
            "no hay un AppImage anterior para restaurar".to_string(),
        ));
    }

    if target.is_file() {
        let failed = sibling_path(target, "failed");
        remove_if_exists(&failed)?;
        fs::rename(target, &failed).map_err(install_error)?;
    }
    fs::rename(&backup, target).map_err(install_error)?;
    set_executable(target)?;
    Ok(target.to_path_buf())
}

/// Ruta del backup que deja [`install_appimage`] junto al AppImage.
pub fn appimage_backup_path(target: &Path) -> PathBuf {
    backup_path(target)
}

/// Instala un artefacto verificado según el entorno detectado.
///
/// El AppImage necesita la ruta del binario actual. Los paquetes `.deb`/`.rpm`
/// se instalan con `pkexec`; Windows ejecuta el instalador y macOS reemplaza
/// el bundle `.app` después de que la app se cierre.
pub fn install_update_artifact(
    artifact: &[u8],
    runtime: Runtime,
    platform: &str,
    appimage_target: Option<&Path>,
) -> Result<(), UpdateError> {
    match runtime {
        Runtime::AppImage => {
            let target = appimage_target.ok_or_else(|| {
                UpdateError::Install("no se conoce la ruta del AppImage actual".to_string())
            })?;
            install_appimage(artifact, target)?;
            Ok(())
        }
        Runtime::Deb => install_deb(artifact),
        Runtime::Rpm => install_rpm(artifact),
        Runtime::Windows => install_windows(artifact, platform.ends_with("-msi")),
        Runtime::MacOs => install_macos(artifact),
        Runtime::Snap => Err(UpdateError::ManagedBySnap),
        Runtime::Msix => Err(UpdateError::UnsupportedRuntime),
        Runtime::Development => Err(UpdateError::UnsupportedRuntime),
    }
}

/// Escribe el artefacto verificado en un archivo temporal privado.
pub fn write_temp_artifact(artifact: &[u8], extension: &str) -> Result<PathBuf, UpdateError> {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    if artifact.is_empty() {
        return Err(UpdateError::Install(
            "el artefacto descargado está vacío".to_string(),
        ));
    }

    let directory = std::env::temp_dir().join("study-planner-updates");
    fs::create_dir_all(&directory).map_err(install_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&directory)
            .map_err(install_error)?
            .permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&directory, permissions).map_err(install_error)?;
    }

    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = directory.join(format!(
        "study-planner-{}-{millis}-{sequence}.{extension}",
        std::process::id()
    ));

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(install_error)?;
    file.write_all(artifact).map_err(install_error)?;
    file.sync_all().map_err(install_error)?;
    Ok(path)
}

#[cfg(target_os = "linux")]
fn install_deb(artifact: &[u8]) -> Result<(), UpdateError> {
    let path = write_temp_artifact(artifact, "deb")?;
    let result = run_pkexec("dpkg", &["-i"], &path);
    let _ = fs::remove_file(&path);
    result
}

#[cfg(not(target_os = "linux"))]
fn install_deb(_artifact: &[u8]) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedRuntime)
}

#[cfg(target_os = "linux")]
fn install_rpm(artifact: &[u8]) -> Result<(), UpdateError> {
    let path = write_temp_artifact(artifact, "rpm")?;
    let result = if command_exists("dnf") {
        run_pkexec("dnf", &["install", "-y", "--nogpgcheck"], &path)
    } else if command_exists("zypper") {
        run_pkexec(
            "zypper",
            &["--non-interactive", "install", "--allow-unsigned-rpm"],
            &path,
        )
    } else {
        run_pkexec("rpm", &["-U", "--replacepkgs"], &path)
    };
    let _ = fs::remove_file(&path);
    result
}

#[cfg(not(target_os = "linux"))]
fn install_rpm(_artifact: &[u8]) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedRuntime)
}

#[cfg(target_os = "linux")]
fn run_pkexec(program: &str, args: &[&str], artifact: &Path) -> Result<(), UpdateError> {
    let status = Command::new("pkexec")
        .arg(program)
        .args(args)
        .arg(artifact)
        .status()
        .map_err(|error| UpdateError::Install(format!("no se pudo ejecutar pkexec: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(UpdateError::Install(format!(
            "el instalador {program} terminó con {status}"
        )))
    }
}

#[cfg(windows)]
fn install_windows(artifact: &[u8], msi: bool) -> Result<(), UpdateError> {
    // `Start-Process -Verb RunAs` eleva el instalador cuando el paquete se
    // instaló por máquina; si el usuario cancela el UAC, PowerShell falla y no
    // cerramos la app.
    let (extension, command) = if msi {
        (
            "msi",
            "Start-Process -FilePath msiexec -ArgumentList @('/i', $env:SP_UPDATE_ARTIFACT, '/qn', '/norestart') -Verb RunAs",
        )
    } else {
        (
            "exe",
            "Start-Process -FilePath $env:SP_UPDATE_ARTIFACT -ArgumentList '/S' -Verb RunAs",
        )
    };
    let path = write_temp_artifact(artifact, extension)?;
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", command])
        .env("SP_UPDATE_ARTIFACT", &path)
        .status()
        .map_err(|error| {
            UpdateError::Install(format!("no se pudo iniciar el instalador: {error}"))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(UpdateError::Install(
            "el instalador de Windows no pudo iniciarse (¿se canceló el UAC?)".to_string(),
        ))
    }
}

#[cfg(not(windows))]
fn install_windows(_artifact: &[u8], _msi: bool) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedRuntime)
}

#[cfg(target_os = "macos")]
fn install_macos(artifact: &[u8]) -> Result<(), UpdateError> {
    let dmg = write_temp_artifact(artifact, "dmg")?;
    let bundle = current_app_bundle().ok_or_else(|| {
        UpdateError::Install("no se pudo localizar la aplicación .app en ejecución".to_string())
    })?;
    let script = write_macos_update_script(&dmg)?;
    Command::new("/bin/sh")
        .arg(&script)
        .arg(&dmg)
        .arg(&bundle)
        .arg(std::process::id().to_string())
        .spawn()
        .map_err(|error| {
            UpdateError::Install(format!("no se pudo iniciar el actualizador: {error}"))
        })?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn install_macos(_artifact: &[u8]) -> Result<(), UpdateError> {
    Err(UpdateError::UnsupportedRuntime)
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(Path::to_path_buf)
}

#[cfg(target_os = "macos")]
fn write_macos_update_script(dmg: &Path) -> Result<PathBuf, UpdateError> {
    let script_path = dmg.with_extension("sh");
    let script = r#"#!/bin/sh
set -eu
DMG="$1"
BUNDLE="$2"
PARENT_PID="$3"
MOUNT="$(mktemp -d "${TMPDIR:-/tmp}/study-planner-dmg.XXXXXX")"
cleanup() {
  hdiutil detach "$MOUNT" -quiet 2>/dev/null || true
  rmdir "$MOUNT" 2>/dev/null || true
}
trap cleanup EXIT
count=0
while kill -0 "$PARENT_PID" 2>/dev/null; do
  count=$((count + 1))
  [ "$count" -gt 300 ] && break
  sleep 0.2
done
hdiutil attach "$DMG" -nobrowse -readonly -mountpoint "$MOUNT" -quiet
APP="$(find "$MOUNT" -maxdepth 2 -name '*.app' -print -quit)"
[ -n "$APP" ] || { echo "no se encontró la app en el DMG" >&2; exit 1; }
STAGE="$(dirname "$BUNDLE")/.study-planner-update-$$.app"
ditto "$APP" "$STAGE"
rm -rf "$BUNDLE.old"
mv "$BUNDLE" "$BUNDLE.old"
mv "$STAGE" "$BUNDLE"
rm -rf "$BUNDLE.old"
rm -f "$DMG"
open "$BUNDLE"
"#;
    let mut file = fs::File::create(&script_path).map_err(install_error)?;
    file.write_all(script.as_bytes()).map_err(install_error)?;
    file.sync_all().map_err(install_error)?;
    set_executable(&script_path)?;
    Ok(script_path)
}

fn backup_path(target: &Path) -> PathBuf {
    sibling_path(target, "old")
}

fn sibling_path(target: &Path, suffix: &str) -> PathBuf {
    let file_name = target
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "StudyPlanner.AppImage".to_string());
    target.with_file_name(format!("{file_name}.{suffix}"))
}

fn remove_if_exists(path: &Path) -> Result<(), UpdateError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(install_error(error)),
    }
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), UpdateError> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).map_err(install_error)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(install_error)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), UpdateError> {
    Ok(())
}

fn install_error(error: std::io::Error) -> UpdateError {
    UpdateError::Install(error.to_string())
}

fn allowed_https_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| ALLOWED_HOSTS.contains(&host))
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
    fn chooses_the_first_available_platform() {
        let payload = format!(
            r#"{{
                "version": "0.2.0",
                "platforms": {{
                    "linux-x86_64": {{
                        "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/app.AppImage",
                        "signature": "{TEST_SIGNATURE}",
                        "sha256": "{TEST_SHA256}"
                    }},
                    "linux-x86_64-deb": {{
                        "url": "https://github.com/wulflo/study-planner/releases/download/v0.2.0/app.deb",
                        "signature": "{TEST_SIGNATURE}",
                        "sha256": "{TEST_SHA256}"
                    }}
                }}
            }}"#
        );
        let candidate = evaluate_manifest_for_platforms(
            payload.as_bytes(),
            "0.1.0",
            &["linux-x86_64-deb".to_string(), "linux-x86_64".to_string()],
        )
        .expect("manifiesto válido")
        .expect("actualización disponible");

        assert_eq!(candidate.platform, "linux-x86_64-deb");
        assert!(candidate.url.ends_with("app.deb"));
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

    struct FakeFetcher {
        manifest: Vec<u8>,
        artifact: Vec<u8>,
    }

    impl HttpFetcher for FakeFetcher {
        fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, UpdateError> {
            let bytes = if url == UPDATE_URL {
                &self.manifest
            } else {
                &self.artifact
            };
            if bytes.len() > max_bytes {
                return Err(UpdateError::ResponseTooLarge);
            }
            Ok(bytes.clone())
        }
    }

    fn temporary_directory(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis());
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "study-planner-updater-{tag}-{}-{millis}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("crear directorio temporal");
        path
    }

    #[test]
    fn fetches_and_verifies_a_candidate_without_network() {
        let fetcher = FakeFetcher {
            manifest: manifest("0.2.0").into_bytes(),
            artifact: TEST_ARTIFACT.to_vec(),
        };

        let check = check_for_update_with_size(&fetcher, "0.1.0").expect("consulta válida");
        assert_eq!(check.manifest_bytes, manifest("0.2.0").len());
        let candidate = check.candidate.expect("actualización disponible");
        let bytes =
            download_update(&fetcher, &candidate, TEST_PUBLIC_KEY).expect("descarga válida");
        assert_eq!(bytes, TEST_ARTIFACT);
    }

    #[test]
    fn refuses_to_download_an_artifact_that_does_not_match() {
        let fetcher = FakeFetcher {
            manifest: manifest("0.2.0").into_bytes(),
            artifact: b"otro-artefacto".to_vec(),
        };
        let candidate = check_for_update(&fetcher, "0.1.0")
            .expect("consulta válida")
            .expect("actualización disponible");

        assert!(matches!(
            download_update(&fetcher, &candidate, TEST_PUBLIC_KEY),
            Err(UpdateError::HashMismatch { .. })
        ));
    }

    #[test]
    fn installs_atomically_and_rolls_back() {
        let directory = temporary_directory("install");
        let target = directory.join("Study Planner.AppImage");
        fs::write(&target, b"binario-anterior").expect("crear binario anterior");

        let report = install_appimage(b"binario-nuevo", &target).expect("instalar");
        assert_eq!(fs::read(&target).unwrap(), b"binario-nuevo");
        assert_eq!(fs::read(&report.backup).unwrap(), b"binario-anterior");
        assert_eq!(report.backup, appimage_backup_path(&target));

        let restored = rollback_appimage(&target).expect("rollback");
        assert_eq!(restored, target);
        assert_eq!(fs::read(&target).unwrap(), b"binario-anterior");
        assert_eq!(
            fs::read(directory.join("Study Planner.AppImage.failed")).unwrap(),
            b"binario-nuevo"
        );

        fs::remove_dir_all(directory).expect("limpiar temporal");
    }

    #[test]
    fn install_requires_an_existing_appimage() {
        let directory = temporary_directory("install-missing");
        let target = directory.join("Study Planner.AppImage");

        assert!(matches!(
            install_appimage(b"binario-nuevo", &target),
            Err(UpdateError::Install(_))
        ));
        assert!(matches!(
            rollback_appimage(&target),
            Err(UpdateError::Install(_))
        ));

        fs::remove_dir_all(directory).expect("limpiar temporal");
    }
}
