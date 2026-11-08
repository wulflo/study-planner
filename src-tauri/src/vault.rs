//! Bóveda local: selección de raíz, validación, lock, backups y tareas.
//!
//! El frontend nunca recibe un `PathBuf` ni un handle de archivo. Solo recibe
//! estado listo para mostrar y las tareas.
//!
//! Garantías de esta capa:
//! - la raíz elegida existe, es un directorio escribible y no está dentro del
//!   repositorio;
//! - una sola instancia puede abrir una bóveda a la vez (lock de archivo);
//! - antes de migrar se crea y verifica un backup; si la migración falla, se
//!   restaura ese backup;
//! - se mantiene un backup principal verificado y una snapshot diaria por
//!   dispositivo, con retención de 7 días.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use study_planner_core::{
    AcademicYear, AttendanceRecord, ClassException, ClassSlot, Exam, ExamTopic, GradeCategory,
    GradeEntry, GradeScale, Holiday, ReviewItem, RotationConfig, SCHEMA_VERSION, SavedFilter,
    ScheduleOccurrence, SelfAssessment, StrengthOverride, StudySession, Subject, SubjectGrade, Tag,
    Task, TaskFilter, Term, Topic,
};
use study_planner_store::{
    BackupReport, ClassSlotInput, ExamInput, GradeCategoryInput, GradeEntryInput, GradeScaleInput,
    RotationInput, Store, StudyPriorityView, StudyStats, SubjectInput, TaskInput, TermInput,
    TopicInput, TopicStudy, UndoEntry, WeekSchedule,
};
use study_planner_transfer::{
    ExportReport, ImportMode, export_to_path, import_from_path, is_empty_vault,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

const SNAPSHOT_RETENTION: usize = 7;
const MAIN_BACKUP_NAME: &str = "study-planner.db";
const LOG_FILE_NAME: &str = "study-planner.log";

/// Estado global de la bóveda. La conexión SQLite vive protegida por el mutex.
pub struct AppState {
    vault: Mutex<Option<VaultSession>>,
    startup_error: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: Mutex::new(None),
            startup_error: Mutex::new(None),
        }
    }
}

struct VaultSession {
    store: Store,
    layout: VaultLayout,
    status: VaultStatus,
    /// Mantiene el lock del sistema operativo mientras la bóveda esté abierta.
    _lock: File,
}

#[derive(Debug, Clone)]
struct VaultLayout {
    root: PathBuf,
    omega: PathBuf,
    backup_dir: PathBuf,
    backup_database: PathBuf,
    longlife: PathBuf,
    database: PathBuf,
    attachments: PathBuf,
    device: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub schema_version: i64,
    pub integrity: String,
    pub foreign_key_violations: usize,
    pub modified_at_millis: u64,
}

impl From<BackupReport> for BackupInfo {
    fn from(report: BackupReport) -> Self {
        Self {
            file_name: report.file_name,
            sha256: report.sha256,
            size_bytes: report.size_bytes,
            schema_version: report.schema_version,
            integrity: report.integrity,
            foreign_key_violations: report.foreign_key_violations,
            modified_at_millis: report.modified_at_millis,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportInfo {
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub rows: usize,
    pub snapshot_at: String,
}

impl From<ExportReport> for ExportInfo {
    fn from(report: ExportReport) -> Self {
        Self {
            file_name: report.file_name,
            sha256: report.sha256,
            size_bytes: report.size_bytes,
            rows: report.rows,
            snapshot_at: report.snapshot_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub status: VaultStatus,
    pub file_name: String,
    pub rows: usize,
    pub attachments: usize,
    pub vault_id: String,
    pub snapshot_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub date: String,
    pub file_name: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub schema_version: i64,
    pub integrity: String,
    pub foreign_key_violations: usize,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotMetadata {
    created_at: String,
    schema_version: i64,
    integrity: String,
    foreign_key_violations: usize,
    sha256: String,
    size_bytes: u64,
    vault_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub configured: bool,
    pub ready: bool,
    pub name: Option<String>,
    pub device: Option<String>,
    pub schema_version: Option<i64>,
    pub integrity: Option<String>,
    pub foreign_key_violations: Option<usize>,
    pub vault_id: Option<String>,
    pub has_data: bool,
    pub backup: Option<BackupInfo>,
    pub snapshots: Vec<SnapshotInfo>,
    pub message: Option<String>,
}

impl VaultStatus {
    fn not_configured() -> Self {
        Self {
            configured: false,
            ready: false,
            name: None,
            device: None,
            schema_version: None,
            integrity: None,
            foreign_key_violations: None,
            vault_id: None,
            has_data: false,
            backup: None,
            snapshots: Vec::new(),
            message: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            configured: true,
            ready: false,
            name: None,
            device: None,
            schema_version: None,
            integrity: None,
            foreign_key_violations: None,
            vault_id: None,
            has_data: false,
            backup: None,
            snapshots: Vec::new(),
            message: Some(message),
        }
    }
}

impl VaultLayout {
    fn resolve(root: PathBuf) -> Result<Self, String> {
        let root = validate_root(&root)?;
        let device = device_name();
        let omega = root.join("omega");
        let longlife = root.join("longlife").join(&device);
        let backup_dir = omega.join("backup");
        let database = omega.join(MAIN_BACKUP_NAME);
        let backup_database = backup_dir.join(MAIN_BACKUP_NAME);
        let attachments = omega.join("attachments");

        Ok(Self {
            root,
            omega,
            backup_dir,
            backup_database,
            longlife,
            database,
            attachments,
            device,
        })
    }

    fn prepare(&self) -> Result<(), String> {
        for directory in [
            &self.omega,
            &self.backup_dir,
            &self.longlife,
            &self.attachments,
        ] {
            fs::create_dir_all(directory).map_err(|error| {
                format!(
                    "No se pudo crear la carpeta {} de la bóveda: {error}",
                    directory.display()
                )
            })?;
            restrict_directory(directory);
        }
        Ok(())
    }
}

impl VaultSession {
    fn open(root: PathBuf) -> Result<Self, String> {
        let layout = VaultLayout::resolve(root)?;
        layout.prepare()?;
        let lock = acquire_lock(&layout.omega)?;
        let store = open_store(&layout)?;

        // En una bóveda recién creada la primera snapshot también actúa como
        // backup inicial, así que no hace falta duplicar el trabajo.
        let backup = if layout.backup_database.exists() {
            ensure_main_backup(&store, &layout)?
        } else {
            Some(create_main_backup(&store, &layout)?)
        };
        let snapshots = ensure_daily_snapshot(&store, &layout)?;
        let status = build_status(&layout, &store, backup, snapshots)?;

        Ok(Self {
            store,
            layout,
            status,
            _lock: lock,
        })
    }
}

fn build_status(
    layout: &VaultLayout,
    store: &Store,
    backup: Option<BackupInfo>,
    snapshots: Vec<SnapshotInfo>,
) -> Result<VaultStatus, String> {
    Ok(VaultStatus {
        configured: true,
        ready: true,
        name: layout
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string),
        device: Some(layout.device.clone()),
        schema_version: Some(store.schema_version().map_err(|error| error.to_string())?),
        integrity: Some(store.integrity_check().map_err(|error| error.to_string())?),
        foreign_key_violations: Some(
            store
                .foreign_key_violations()
                .map_err(|error| error.to_string())?,
        ),
        vault_id: Some(store.vault_id().map_err(|error| error.to_string())?),
        has_data: !is_empty_vault(store).map_err(|error| error.to_string())?,
        backup,
        snapshots,
        message: None,
    })
}

fn open_store(layout: &VaultLayout) -> Result<Store, String> {
    let existed = layout.database.exists();
    let mut store = Store::open(&layout.database)
        .map_err(|error| format!("No se pudo abrir la base de la bóveda: {error}"))?;

    if !existed {
        store
            .migrate()
            .map_err(|error| format!("No se pudo crear el esquema de la bóveda: {error}"))?;
        return Ok(store);
    }

    let current = store
        .schema_version()
        .map_err(|error| format!("No se pudo leer el esquema de la bóveda: {error}"))?;
    let supported = i64::from(SCHEMA_VERSION);

    if current < supported {
        let backup_name = format!("pre-migration-v{current}-{}.db", unix_millis());
        let backup_path = layout.backup_dir.join(&backup_name);
        let report = store.backup_to(&backup_path).map_err(|error| {
            format!("No se pudo crear el backup previo a la migración: {error}")
        })?;

        if let Err(error) = store.migrate() {
            drop(store);
            restore_database(&layout.database, &backup_path).map_err(|restore_error| {
                format!(
                    "La migración falló ({error}) y no se pudo restaurar el backup {}: {restore_error}",
                    report.file_name
                )
            })?;

            return Err(format!(
                "La migración falló y se restauró el backup previo {}: {error}",
                report.file_name
            ));
        }
    } else {
        store
            .migrate()
            .map_err(|error| format!("No se pudo migrar la bóveda: {error}"))?;
    }

    Ok(store)
}

fn restore_database(database: &Path, backup: &Path) -> Result<(), String> {
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = database.as_os_str().to_owned();
        sidecar.push(suffix);
        let _ = fs::remove_file(PathBuf::from(sidecar));
    }

    fs::copy(backup, database)
        .map_err(|error| format!("No se pudo copiar el backup sobre la base: {error}"))?;

    OpenOptions::new()
        .write(true)
        .open(database)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("No se pudo sincronizar la base restaurada: {error}"))?;

    Ok(())
}

fn ensure_main_backup(store: &Store, layout: &VaultLayout) -> Result<Option<BackupInfo>, String> {
    let current_schema = store.schema_version().map_err(|error| error.to_string())?;
    let backup = match Store::verify_backup(&layout.backup_database) {
        Ok(report) if report.schema_version == current_schema => report,
        _ => store
            .backup_to(&layout.backup_database)
            .map_err(|error| format!("No se pudo verificar el backup principal: {error}"))?,
    };
    Ok(Some(BackupInfo::from(backup)))
}

fn create_main_backup(store: &Store, layout: &VaultLayout) -> Result<BackupInfo, String> {
    let report = store
        .backup_to(&layout.backup_database)
        .map_err(|error| format!("No se pudo crear el backup principal: {error}"))?;
    Ok(BackupInfo::from(report))
}

fn ensure_daily_snapshot(store: &Store, layout: &VaultLayout) -> Result<Vec<SnapshotInfo>, String> {
    let date = store
        .local_date()
        .map_err(|error| format!("No se pudo calcular la fecha local: {error}"))?;
    let schema = store.schema_version().map_err(|error| error.to_string())?;
    let directory = layout.longlife.join(&date);
    let metadata_path = directory.join("snapshot.json");
    let database_path = directory.join(MAIN_BACKUP_NAME);

    let existing = read_snapshot_metadata(&metadata_path);
    let needs_snapshot = !(metadata_path.exists()
        && database_path.exists()
        && existing
            .as_ref()
            .is_some_and(|meta| meta.schema_version == schema));

    if needs_snapshot {
        if directory.exists() {
            fs::remove_dir_all(&directory).map_err(|error| {
                format!("No se pudo descartar una snapshot incompleta: {error}")
            })?;
        }
        fs::create_dir_all(&directory)
            .map_err(|error| format!("No se pudo crear la snapshot diaria: {error}"))?;
        restrict_directory(&directory);

        let report = store
            .backup_to(&database_path)
            .map_err(|error| format!("No se pudo crear la snapshot diaria: {error}"))?;
        let metadata = SnapshotMetadata {
            created_at: store.utc_now().map_err(|error| error.to_string())?,
            schema_version: report.schema_version,
            integrity: report.integrity,
            foreign_key_violations: report.foreign_key_violations,
            sha256: report.sha256,
            size_bytes: report.size_bytes,
            vault_id: store.vault_id().map_err(|error| error.to_string())?,
        };
        let raw = serde_json::to_vec_pretty(&metadata)
            .map_err(|error| format!("No se pudo serializar la snapshot: {error}"))?;
        fs::write(&metadata_path, raw)
            .map_err(|error| format!("No se pudo escribir snapshot.json: {error}"))?;
        restrict_file(&metadata_path);
    }

    prune_snapshots(&layout.longlife)?;
    read_snapshots(&layout.longlife)
}

fn read_snapshot_metadata(path: &Path) -> Option<SnapshotMetadata> {
    let raw = fs::read(path).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn prune_snapshots(base: &Path) -> Result<(), String> {
    let mut dates = date_directories(base)?;
    dates.sort();

    while dates.len() > SNAPSHOT_RETENTION {
        let oldest = dates.remove(0);
        fs::remove_dir_all(base.join(&oldest)).map_err(|error| {
            format!("No se pudo aplicar la retención de snapshots ({oldest}): {error}")
        })?;
    }

    Ok(())
}

fn read_snapshots(base: &Path) -> Result<Vec<SnapshotInfo>, String> {
    let mut snapshots = Vec::new();

    for date in date_directories(base)? {
        let directory = base.join(&date);
        let database_path = directory.join(MAIN_BACKUP_NAME);
        if !database_path.exists() {
            continue;
        }
        let Some(metadata) = read_snapshot_metadata(&directory.join("snapshot.json")) else {
            continue;
        };

        snapshots.push(SnapshotInfo {
            date,
            file_name: MAIN_BACKUP_NAME.to_string(),
            sha256: metadata.sha256,
            size_bytes: metadata.size_bytes,
            schema_version: metadata.schema_version,
            integrity: metadata.integrity,
            foreign_key_violations: metadata.foreign_key_violations,
            created_at: metadata.created_at,
        });
    }

    snapshots.sort_by(|left, right| right.date.cmp(&left.date));
    Ok(snapshots)
}

fn date_directories(base: &Path) -> Result<Vec<String>, String> {
    if !base.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(base)
        .map_err(|error| format!("No se pudo leer el directorio de snapshots: {error}"))?;
    let mut dates = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("No se pudo leer una snapshot: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("No se pudo leer una snapshot: {error}"))?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if is_date_folder(name) {
            dates.push(name.to_string());
        }
    }

    Ok(dates)
}

fn is_date_folder(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn validate_root(root: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(root)
        .map_err(|error| format!("La carpeta elegida no existe o no se puede leer: {error}"))?;

    if !canonical.is_dir() {
        return Err("La bóveda tiene que ser una carpeta, no un archivo.".to_string());
    }
    if canonical.parent().is_none() {
        return Err(
            "No podés usar la raíz del sistema de archivos como bóveda. Elegí una carpeta propia."
                .to_string(),
        );
    }
    if is_inside_repository(&canonical) {
        return Err(
            "La bóveda no puede estar dentro del repositorio de Study Planner. Elegí una carpeta fuera del código."
                .to_string(),
        );
    }

    let probe = canonical.join(format!(
        ".study-planner-write-test-{}-{}",
        std::process::id(),
        unix_millis()
    ));
    fs::write(&probe, b"ok").map_err(|error| format!("La carpeta no es escribible: {error}"))?;
    let _ = fs::remove_file(&probe);

    Ok(canonical)
}

fn is_inside_repository(path: &Path) -> bool {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(workspace) = manifest.parent() else {
        return false;
    };
    let Ok(workspace) = fs::canonicalize(workspace) else {
        return false;
    };

    path == workspace || path.starts_with(workspace)
}

fn acquire_lock(omega: &Path) -> Result<File, String> {
    let path = omega.join(".study-planner.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(|error| format!("No se pudo crear el lock de la bóveda: {error}"))?;

    match file.try_lock() {
        Ok(()) => {
            restrict_file(&path);
            Ok(file)
        }
        Err(std::fs::TryLockError::WouldBlock) => Err(
            "La bóveda ya está abierta en otra instancia de Study Planner. Cerrá la otra ventana e intentá de nuevo."
                .to_string(),
        ),
        Err(std::fs::TryLockError::Error(error)) => {
            Err(format!("No se pudo bloquear la bóveda: {error}"))
        }
    }
}

fn restrict_directory(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn restrict_file(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Agrega una línea al log local de la bóveda. Nunca incluye rutas absolutas:
/// solo eventos y nombres de archivo. Un fallo al escribir no interrumpe la
/// operación principal.
fn append_log(layout: &VaultLayout, store: &Store, event: &str, detail: &str) {
    let timestamp = store
        .utc_now()
        .unwrap_or_else(|_| "1970-01-01T00:00:00.000Z".to_string());
    let path = layout.omega.join(LOG_FILE_NAME);
    let line = format!("{timestamp} event={event} {detail}\n");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        restrict_file(&path);
        let _ = file.write_all(line.as_bytes());
    }
}

fn pick_save_path(app: &AppHandle, file_name: &str) -> Result<Option<PathBuf>, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Exportar la bóveda de Study Planner")
        .set_file_name(file_name)
        .add_filter("Exportación de Study Planner", &["txt"])
        .save_file(move |path| {
            let _ = sender.send(path);
        });

    let path = receiver
        .recv()
        .map_err(|_| "El diálogo se cerró sin respuesta.".to_string())?;
    match path {
        Some(path) => Ok(Some(path.into_path().map_err(|error| error.to_string())?)),
        None => Ok(None),
    }
}

fn pick_open_path(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Importar una bóveda en Study Planner")
        .add_filter("Exportación de Study Planner", &["txt"])
        .pick_file(move |path| {
            let _ = sender.send(path);
        });

    let path = receiver
        .recv()
        .map_err(|_| "El diálogo se cerró sin respuesta.".to_string())?;
    match path {
        Some(path) => Ok(Some(path.into_path().map_err(|error| error.to_string())?)),
        None => Ok(None),
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("archivo")
        .to_string()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis())
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultConfig {
    root: String,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("No se pudo resolver la carpeta de configuración: {error}"))?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("No se pudo crear la carpeta de configuración: {error}"))?;
    Ok(directory.join("vault.json"))
}

fn read_root(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("No se pudo leer la configuración de la bóveda: {error}"))?;
    let config: VaultConfig = serde_json::from_str(&raw)
        .map_err(|error| format!("La configuración de la bóveda no es válida: {error}"))?;

    Ok(Some(PathBuf::from(config.root)))
}

fn write_root(app: &AppHandle, root: &Path) -> Result<(), String> {
    let path = config_path(app)?;
    let root = root
        .to_str()
        .ok_or_else(|| "La ruta elegida no es UTF-8.".to_string())?;
    let raw = serde_json::to_string_pretty(&VaultConfig {
        root: root.to_string(),
    })
    .map_err(|error| format!("No se pudo serializar la bóveda: {error}"))?;

    fs::write(&path, raw)
        .map_err(|error| format!("No se pudo guardar la configuración de la bóveda: {error}"))
}

/// Carga la bóveda configurada al arrancar. Un fallo no bloquea la app: la UI
/// muestra el error y permite elegir otra carpeta.
pub fn load_state(app: &AppHandle) -> AppState {
    let mut vault = None;
    let mut startup_error = None;

    match read_root(app) {
        Ok(Some(root)) => match VaultSession::open(root) {
            Ok(session) => vault = Some(session),
            Err(error) => startup_error = Some(error),
        },
        Ok(None) => {}
        Err(error) => startup_error = Some(error),
    }

    AppState {
        vault: Mutex::new(vault),
        startup_error: Mutex::new(startup_error),
    }
}

fn device_name() -> String {
    let raw = fs::read_to_string("/etc/hostname").unwrap_or_default();
    let sanitized: String = raw
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .take(64)
        .collect();

    if sanitized.is_empty() {
        "equipo".to_string()
    } else {
        sanitized
    }
}

fn no_vault_error() -> String {
    "Primero elegí una carpeta de bóveda en la sección Bóveda.".to_string()
}

fn with_store<T>(
    state: &State<'_, AppState>,
    operation: impl FnOnce(&Store) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;
    let session = guard.as_ref().ok_or_else(no_vault_error)?;
    operation(&session.store)
}

#[tauri::command]
pub fn get_vault_status(state: State<'_, AppState>) -> VaultStatus {
    if let Ok(guard) = state.vault.lock()
        && let Some(session) = guard.as_ref()
    {
        return build_status(
            &session.layout,
            &session.store,
            session.status.backup.clone(),
            session.status.snapshots.clone(),
        )
        .unwrap_or_else(|_| session.status.clone());
    }

    let message = state
        .startup_error
        .lock()
        .ok()
        .and_then(|error| error.as_deref().map(str::to_string));

    match message {
        Some(message) => VaultStatus::error(message),
        None => VaultStatus::not_configured(),
    }
}

/// Abre el selector nativo de carpetas. Devuelve `None` si el usuario cancela.
#[tauri::command]
pub async fn choose_vault(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<VaultStatus>, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Elegir carpeta para la bóveda de Study Planner")
        .pick_folder(move |folder| {
            let _ = sender.send(folder);
        });

    let folder = receiver
        .recv()
        .map_err(|_| "El diálogo se cerró sin respuesta.".to_string())?;
    let Some(folder) = folder else {
        return Ok(None);
    };

    let path = folder.into_path().map_err(|error| error.to_string())?;
    let canonical = validate_root(&path)?;

    {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;
        if let Some(current) = guard.as_ref()
            && current.layout.root == canonical
        {
            return Ok(Some(current.status.clone()));
        }
    }

    let session = VaultSession::open(canonical.clone())?;
    write_root(&app, &canonical)?;

    let status = session.status.clone();
    {
        let mut guard = state
            .vault
            .lock()
            .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;
        *guard = Some(session);
    }
    if let Ok(mut error) = state.startup_error.lock() {
        *error = None;
    }

    Ok(Some(status))
}

/// Crea o reemplaza el backup principal verificado de la bóveda abierta.
#[tauri::command]
pub fn create_backup(state: State<'_, AppState>) -> Result<VaultStatus, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;
    let session = guard.as_mut().ok_or_else(no_vault_error)?;

    let report = session
        .store
        .backup_to(&session.layout.backup_database)
        .map_err(|error| format!("No se pudo crear el backup: {error}"))?;
    session.status.backup = Some(BackupInfo::from(report));
    append_log(&session.layout, &session.store, "backup", "principal");

    Ok(session.status.clone())
}

/// Exporta toda la bóveda a un `.txt` canónico elegido por el usuario.
#[tauri::command]
pub async fn export_vault(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ExportInfo>, String> {
    let Some(path) = pick_save_path(&app, "study-planner-export.txt")? else {
        return Ok(None);
    };

    let guard = state
        .vault
        .lock()
        .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;
    let session = guard.as_ref().ok_or_else(no_vault_error)?;

    match export_to_path(&session.store, &session.layout.attachments, &path) {
        Ok(report) => {
            append_log(
                &session.layout,
                &session.store,
                "export",
                &format!(
                    "file={} rows={} sha256={}",
                    report.file_name, report.rows, report.sha256
                ),
            );
            Ok(Some(ExportInfo::from(report)))
        }
        Err(error) => {
            append_log(
                &session.layout,
                &session.store,
                "export_error",
                &format!("file={} error={error}", file_name_of(&path)),
            );
            Err(format!("No se pudo exportar la bóveda: {error}"))
        }
    }
}

/// Importa un `.txt` canónico en una bóveda vacía. La operación es
/// transaccional: si falla, la bóveda queda exactamente como estaba.
#[tauri::command]
pub async fn import_vault(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ImportOutcome>, String> {
    let Some(path) = pick_open_path(&app)? else {
        return Ok(None);
    };

    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;
    let session = guard.as_mut().ok_or_else(no_vault_error)?;
    let file_name = file_name_of(&path);

    match import_from_path(
        &session.store,
        &session.layout.attachments,
        &path,
        ImportMode::EmptyOnly,
    ) {
        Ok(report) => {
            append_log(
                &session.layout,
                &session.store,
                "import",
                &format!(
                    "file={} rows={} attachments={} vault_id={}",
                    file_name, report.rows, report.attachments, report.vault_id
                ),
            );

            let backup = match create_main_backup(&session.store, &session.layout) {
                Ok(backup) => Some(backup),
                Err(error) => {
                    append_log(
                        &session.layout,
                        &session.store,
                        "backup_error",
                        &format!("after_import error={error}"),
                    );
                    session.status.backup.clone()
                }
            };
            let snapshots = ensure_daily_snapshot(&session.store, &session.layout)?;
            session.status = build_status(&session.layout, &session.store, backup, snapshots)?;

            Ok(Some(ImportOutcome {
                status: session.status.clone(),
                file_name,
                rows: report.rows,
                attachments: report.attachments,
                vault_id: report.vault_id,
                snapshot_at: report.snapshot_at,
            }))
        }
        Err(error) => {
            append_log(
                &session.layout,
                &session.store,
                "import_error",
                &format!("file={file_name} error={error}"),
            );
            Err(format!("No se pudo importar la bóveda: {error}"))
        }
    }
}

/// Restaura el backup principal verificado. Antes de tocarlo crea un backup
/// `pre-restore-*` del estado actual y conserva el lock de la bóveda.
#[tauri::command]
pub fn restore_main_backup(state: State<'_, AppState>) -> Result<VaultStatus, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "El estado de la bóveda está bloqueado.".to_string())?;

    let (report, pre_restore_name) = {
        let session = guard.as_ref().ok_or_else(no_vault_error)?;
        if !session.layout.backup_database.exists() {
            return Err("Todavía no hay un backup principal para restaurar.".to_string());
        }

        let report = Store::verify_backup(&session.layout.backup_database).map_err(|error| {
            format!("El backup principal no es válido y no se puede restaurar: {error}")
        })?;
        if report.schema_version != i64::from(SCHEMA_VERSION) {
            return Err(format!(
                "El backup principal usa el esquema {} y esta app espera {}.",
                report.schema_version, SCHEMA_VERSION
            ));
        }
        if report.integrity != "ok" || report.foreign_key_violations != 0 {
            return Err(
                "El backup principal no pasó las verificaciones y no se restauró.".to_string(),
            );
        }

        let name = format!("pre-restore-{}.db", unix_millis());
        let path = session.layout.backup_dir.join(&name);
        session.store.backup_to(&path).map_err(|error| {
            format!("No se pudo crear el backup previo a la restauración: {error}")
        })?;
        append_log(
            &session.layout,
            &session.store,
            "pre_restore_backup",
            &format!("file={name}"),
        );

        (report, name)
    };

    let owned = guard.take().ok_or_else(no_vault_error)?;
    let VaultSession {
        store,
        layout,
        _lock,
        ..
    } = owned;
    drop(store);

    restore_database(&layout.database, &layout.backup_database)
        .map_err(|error| format!("No se pudo restaurar el backup principal: {error}"))?;

    let store = open_store(&layout)?;
    let backup = Some(BackupInfo::from(report));
    let snapshots = ensure_daily_snapshot(&store, &layout)?;
    let status = build_status(&layout, &store, backup, snapshots)?;
    append_log(
        &layout,
        &store,
        "restore",
        &format!(
            "file={} pre_restore={pre_restore_name}",
            status
                .backup
                .as_ref()
                .map_or("desconocido", |backup| backup.file_name.as_str())
        ),
    );

    *guard = Some(VaultSession {
        store,
        layout,
        status: status.clone(),
        _lock,
    });

    Ok(status)
}

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    with_store(&state, |store| {
        store.list_tasks().map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_task(title: String, state: State<'_, AppState>) -> Result<Task, String> {
    with_store(&state, |store| {
        store.create_task(&title).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn set_task_done(id: String, done: bool, state: State<'_, AppState>) -> Result<Task, String> {
    with_store(&state, |store| {
        store
            .set_task_done(&id, done)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_task(&id).map_err(|error| error.to_string())
    })
}

// ---------------------------------------------------------------------------
// Tareas completas, etiquetas, filtros y undo
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn create_task_full(input: TaskInput, state: State<'_, AppState>) -> Result<Task, String> {
    with_store(&state, |store| {
        store
            .create_task_full(&input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_task(
    id: String,
    input: TaskInput,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    with_store(&state, |store| {
        store
            .update_task(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn set_task_status(
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    with_store(&state, |store| {
        store
            .set_task_status(&id, &status)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn search_tasks(filter: TaskFilter, state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    with_store(&state, |store| {
        store
            .search_tasks(&filter)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<Tag>, String> {
    with_store(&state, |store| {
        store.list_tags().map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_tag(
    name: String,
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<Tag, String> {
    with_store(&state, |store| {
        store
            .create_tag(&name, color.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_tag(
    id: String,
    name: String,
    color: Option<String>,
    state: State<'_, AppState>,
) -> Result<Tag, String> {
    with_store(&state, |store| {
        store
            .update_tag(&id, &name, color.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_tag(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_tag(&id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn set_task_tags(
    task_id: String,
    tag_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .set_task_tags(&task_id, &tag_ids)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_saved_filters(
    screen: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SavedFilter>, String> {
    with_store(&state, |store| {
        store
            .list_saved_filters(screen.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_saved_filter(
    screen: String,
    name: String,
    query_json: String,
    state: State<'_, AppState>,
) -> Result<SavedFilter, String> {
    with_store(&state, |store| {
        store
            .create_saved_filter(&screen, &name, &query_json)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_saved_filter(
    id: String,
    name: String,
    query_json: String,
    state: State<'_, AppState>,
) -> Result<SavedFilter, String> {
    with_store(&state, |store| {
        store
            .update_saved_filter(&id, &name, &query_json)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_saved_filter(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_saved_filter(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_undo_entries(state: State<'_, AppState>) -> Result<Vec<UndoEntry>, String> {
    with_store(&state, |store| {
        store.list_undo_entries().map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn undo_entry(id: String, state: State<'_, AppState>) -> Result<Option<Task>, String> {
    with_store(&state, |store| {
        store.undo_entry(&id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn purge_expired_undo(state: State<'_, AppState>) -> Result<usize, String> {
    with_store(&state, |store| {
        store
            .purge_expired_undo()
            .map_err(|error| error.to_string())
    })
}

// ---------------------------------------------------------------------------
// Exámenes y temas
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_exams(
    term_id: Option<String>,
    subject_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Exam>, String> {
    with_store(&state, |store| {
        store
            .list_exams(term_id.as_deref(), subject_id.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_exam(
    subject_id: String,
    term_id: String,
    input: ExamInput,
    state: State<'_, AppState>,
) -> Result<Exam, String> {
    with_store(&state, |store| {
        store
            .create_exam(&subject_id, &term_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_exam(
    id: String,
    input: ExamInput,
    state: State<'_, AppState>,
) -> Result<Exam, String> {
    with_store(&state, |store| {
        store
            .update_exam(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_exam(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_exam(&id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_topics(subject_id: String, state: State<'_, AppState>) -> Result<Vec<Topic>, String> {
    with_store(&state, |store| {
        store
            .list_topics(&subject_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_topic(
    subject_id: String,
    input: TopicInput,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    with_store(&state, |store| {
        store
            .create_topic(&subject_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_topic(
    id: String,
    input: TopicInput,
    state: State<'_, AppState>,
) -> Result<Topic, String> {
    with_store(&state, |store| {
        store
            .update_topic(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_topic(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_topic(&id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_exam_topics(
    exam_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ExamTopic>, String> {
    with_store(&state, |store| {
        store
            .list_exam_topics(&exam_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn link_exam_topic(
    exam_id: String,
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<ExamTopic, String> {
    with_store(&state, |store| {
        store
            .link_exam_topic(&exam_id, &topic_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn unlink_exam_topic(
    exam_id: String,
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .unlink_exam_topic(&exam_id, &topic_id)
            .map_err(|error| error.to_string())
    })
}

// ---------------------------------------------------------------------------
// Notas
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_grade_scales(
    subject_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<GradeScale>, String> {
    with_store(&state, |store| {
        store
            .list_grade_scales(subject_id.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_grade_scale(
    input: GradeScaleInput,
    state: State<'_, AppState>,
) -> Result<GradeScale, String> {
    with_store(&state, |store| {
        store
            .create_grade_scale(&input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_grade_scale(
    id: String,
    input: GradeScaleInput,
    state: State<'_, AppState>,
) -> Result<GradeScale, String> {
    with_store(&state, |store| {
        store
            .update_grade_scale(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_grade_scale(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_grade_scale(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_grade_categories(
    subject_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<GradeCategory>, String> {
    with_store(&state, |store| {
        store
            .list_grade_categories(&subject_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_grade_category(
    subject_id: String,
    input: GradeCategoryInput,
    state: State<'_, AppState>,
) -> Result<GradeCategory, String> {
    with_store(&state, |store| {
        store
            .create_grade_category(&subject_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_grade_category(
    id: String,
    input: GradeCategoryInput,
    state: State<'_, AppState>,
) -> Result<GradeCategory, String> {
    with_store(&state, |store| {
        store
            .update_grade_category(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_grade_category(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_grade_category(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_grade_entries(
    subject_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<GradeEntry>, String> {
    with_store(&state, |store| {
        store
            .list_grade_entries(&subject_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_grade_entry(
    subject_id: String,
    input: GradeEntryInput,
    state: State<'_, AppState>,
) -> Result<GradeEntry, String> {
    with_store(&state, |store| {
        store
            .create_grade_entry(&subject_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_grade_entry(
    id: String,
    input: GradeEntryInput,
    state: State<'_, AppState>,
) -> Result<GradeEntry, String> {
    with_store(&state, |store| {
        store
            .update_grade_entry(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_grade_entry(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_grade_entry(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn get_subject_grade(
    subject_id: String,
    state: State<'_, AppState>,
) -> Result<SubjectGrade, String> {
    with_store(&state, |store| {
        store
            .get_subject_grade(&subject_id)
            .map_err(|error| error.to_string())
    })
}

// ---------------------------------------------------------------------------
// Estructura académica
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_academic_years(state: State<'_, AppState>) -> Result<Vec<AcademicYear>, String> {
    with_store(&state, |store| {
        store
            .list_academic_years()
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_academic_year(
    name: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<AcademicYear, String> {
    with_store(&state, |store| {
        store
            .create_academic_year(&name, &start_date, &end_date)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_academic_year(
    id: String,
    name: String,
    start_date: String,
    end_date: String,
    state: State<'_, AppState>,
) -> Result<AcademicYear, String> {
    with_store(&state, |store| {
        store
            .update_academic_year(&id, &name, &start_date, &end_date)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_academic_year(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_academic_year(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_terms(
    year_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Term>, String> {
    with_store(&state, |store| {
        store
            .list_terms(year_id.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_term(
    year_id: String,
    input: TermInput,
    state: State<'_, AppState>,
) -> Result<Term, String> {
    with_store(&state, |store| {
        store
            .create_term(&year_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_term(
    id: String,
    input: TermInput,
    state: State<'_, AppState>,
) -> Result<Term, String> {
    with_store(&state, |store| {
        store
            .update_term(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_term(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_term(&id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_subjects(term_id: String, state: State<'_, AppState>) -> Result<Vec<Subject>, String> {
    with_store(&state, |store| {
        store
            .list_subjects(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_subject(
    term_id: String,
    input: SubjectInput,
    state: State<'_, AppState>,
) -> Result<Subject, String> {
    with_store(&state, |store| {
        store
            .create_subject(&term_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_subject(
    id: String,
    input: SubjectInput,
    state: State<'_, AppState>,
) -> Result<Subject, String> {
    with_store(&state, |store| {
        store
            .update_subject(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_subject(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_subject(&id).map_err(|error| error.to_string())
    })
}

// ---------------------------------------------------------------------------
// Horario, rotaciones, feriados, excepciones y asistencia
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_class_slots(
    term_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClassSlot>, String> {
    with_store(&state, |store| {
        store
            .list_class_slots(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_class_slot(
    subject_id: String,
    input: ClassSlotInput,
    state: State<'_, AppState>,
) -> Result<ClassSlot, String> {
    with_store(&state, |store| {
        store
            .create_class_slot(&subject_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn update_class_slot(
    id: String,
    input: ClassSlotInput,
    state: State<'_, AppState>,
) -> Result<ClassSlot, String> {
    with_store(&state, |store| {
        store
            .update_class_slot(&id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_class_slot(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_class_slot(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn get_rotation(
    term_id: String,
    state: State<'_, AppState>,
) -> Result<Option<RotationConfig>, String> {
    with_store(&state, |store| {
        store
            .get_rotation(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn save_rotation(
    term_id: String,
    input: RotationInput,
    state: State<'_, AppState>,
) -> Result<RotationConfig, String> {
    with_store(&state, |store| {
        store
            .save_rotation(&term_id, &input)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_rotation(term_id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_rotation(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_holidays(term_id: String, state: State<'_, AppState>) -> Result<Vec<Holiday>, String> {
    with_store(&state, |store| {
        store
            .list_holidays(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_holiday(
    term_id: Option<String>,
    date: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<Holiday, String> {
    with_store(&state, |store| {
        store
            .create_holiday(term_id.as_deref(), &date, &name)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_holiday(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store.delete_holiday(&id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_class_exceptions(
    term_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ClassException>, String> {
    with_store(&state, |store| {
        store
            .list_class_exceptions(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_class_exception(
    slot_id: String,
    date: String,
    kind: String,
    new_start_time: Option<String>,
    new_end_time: Option<String>,
    new_room: Option<String>,
    state: State<'_, AppState>,
) -> Result<ClassException, String> {
    with_store(&state, |store| {
        store
            .create_class_exception(
                &slot_id,
                &date,
                &kind,
                new_start_time.as_deref(),
                new_end_time.as_deref(),
                new_room.as_deref(),
            )
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_class_exception(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_class_exception(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_attendance(
    term_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AttendanceRecord>, String> {
    with_store(&state, |store| {
        store
            .list_attendance(&term_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn set_attendance(
    slot_id: String,
    date: String,
    status: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<AttendanceRecord, String> {
    with_store(&state, |store| {
        store
            .set_attendance(&slot_id, &date, &status, note.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn clear_attendance(
    slot_id: String,
    date: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .clear_attendance(&slot_id, &date)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn get_week_schedule(
    term_id: String,
    week_start: String,
    state: State<'_, AppState>,
) -> Result<WeekSchedule, String> {
    with_store(&state, |store| {
        store
            .week_schedule(&term_id, &week_start)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn get_next_class(
    term_id: String,
    from_date: String,
    from_time: String,
    state: State<'_, AppState>,
) -> Result<Option<ScheduleOccurrence>, String> {
    with_store(&state, |store| {
        store
            .next_class(&term_id, &from_date, &from_time)
            .map_err(|error| error.to_string())
    })
}

// ---------------------------------------------------------------------------
// Estudio, autoevaluación, repaso y estadísticas
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_self_assessments(
    topic_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<SelfAssessment>, String> {
    with_store(&state, |store| {
        store
            .list_self_assessments(&topic_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_self_assessment(
    topic_id: String,
    score: i64,
    note: Option<String>,
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<SelfAssessment, String> {
    let source = source.as_deref().unwrap_or("manual");
    with_store(&state, |store| {
        store
            .create_self_assessment(&topic_id, score, note.as_deref(), source)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_self_assessment(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_self_assessment(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_strength_overrides(
    state: State<'_, AppState>,
) -> Result<Vec<StrengthOverride>, String> {
    with_store(&state, |store| {
        store
            .list_strength_overrides()
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn set_strength_override(
    topic_id: String,
    classification: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<StrengthOverride, String> {
    with_store(&state, |store| {
        store
            .set_strength_override(&topic_id, &classification, reason.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn clear_strength_override(topic_id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .clear_strength_override(&topic_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_review_items(
    subject_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewItem>, String> {
    with_store(&state, |store| {
        store
            .list_review_items(subject_id.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn review_topic(
    topic_id: String,
    quality: i64,
    today: String,
    state: State<'_, AppState>,
) -> Result<ReviewItem, String> {
    with_store(&state, |store| {
        store
            .review_topic(&topic_id, quality, &today)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn set_review_due(
    topic_id: String,
    due_date: String,
    state: State<'_, AppState>,
) -> Result<ReviewItem, String> {
    with_store(&state, |store| {
        store
            .set_review_due(&topic_id, &due_date)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_review_item(topic_id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_review_item(&topic_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_study_sessions(
    from: Option<String>,
    to: Option<String>,
    subject_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<StudySession>, String> {
    with_store(&state, |store| {
        store
            .list_study_sessions(from.as_deref(), to.as_deref(), subject_id.as_deref())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn start_study_session(
    subject_id: Option<String>,
    topic_id: Option<String>,
    mode: String,
    planned_minutes: Option<i64>,
    state: State<'_, AppState>,
) -> Result<StudySession, String> {
    with_store(&state, |store| {
        store
            .start_study_session(
                subject_id.as_deref(),
                topic_id.as_deref(),
                &mode,
                planned_minutes,
            )
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn finish_study_session(
    id: String,
    actual_minutes: Option<i64>,
    interruptions: Option<i64>,
    notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<StudySession, String> {
    with_store(&state, |store| {
        store
            .finish_study_session(
                &id,
                actual_minutes,
                interruptions,
                notes.as_deref(),
            )
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn delete_study_session(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_store(&state, |store| {
        store
            .delete_study_session(&id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_topic_studies(
    subject_id: Option<String>,
    today: String,
    state: State<'_, AppState>,
) -> Result<Vec<TopicStudy>, String> {
    with_store(&state, |store| {
        store
            .list_topic_studies(subject_id.as_deref(), &today)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn list_study_priorities(
    today: String,
    state: State<'_, AppState>,
) -> Result<Vec<StudyPriorityView>, String> {
    with_store(&state, |store| {
        store
            .list_study_priorities(&today)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn get_study_stats(
    from: Option<String>,
    today: String,
    state: State<'_, AppState>,
) -> Result<StudyStats, String> {
    with_store(&state, |store| {
        store
            .study_stats(from.as_deref(), &today)
            .map_err(|error| error.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "study-planner-vault-{tag}-{}-{}",
            std::process::id(),
            unix_millis()
        ));
        fs::create_dir_all(&path).expect("crear directorio temporal");
        path
    }

    #[test]
    fn accepts_writable_directory_outside_repository() {
        let directory = temp_dir("valid");
        let canonical = validate_root(&directory).expect("raíz válida");
        assert_eq!(canonical, fs::canonicalize(&directory).unwrap());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_files_and_missing_paths() {
        let directory = temp_dir("file");
        let file = directory.join("archivo.txt");
        fs::write(&file, b"x").unwrap();

        assert!(validate_root(&file).is_err());
        assert!(validate_root(&directory.join("no-existe")).is_err());

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_path_inside_repository() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let error = validate_root(manifest).expect_err("el repo debe ser rechazado");
        assert!(error.contains("repositorio"));
    }

    #[test]
    fn date_folder_pattern_is_strict() {
        assert!(is_date_folder("2026-11-08"));
        assert!(!is_date_folder("2026-1-08"));
        assert!(!is_date_folder("2026-11-08-copia"));
        assert!(!is_date_folder("../../etc"));
    }

    #[test]
    fn snapshots_are_read_newest_first() {
        let directory = temp_dir("snapshots");
        for (date, checksum) in [
            ("2026-11-07", "a"),
            ("2026-11-08", "b"),
            ("no-es-fecha", "c"),
        ] {
            let snapshot = directory.join(date);
            fs::create_dir_all(&snapshot).unwrap();
            if date != "no-es-fecha" {
                fs::write(snapshot.join(MAIN_BACKUP_NAME), b"db").unwrap();
                let metadata = SnapshotMetadata {
                    created_at: format!("{date}T00:00:00.000Z"),
                    schema_version: 1,
                    integrity: "ok".to_string(),
                    foreign_key_violations: 0,
                    sha256: checksum.repeat(64),
                    size_bytes: 2,
                    vault_id: "vault".to_string(),
                };
                fs::write(
                    snapshot.join("snapshot.json"),
                    serde_json::to_vec(&metadata).unwrap(),
                )
                .unwrap();
            }
        }

        let snapshots = read_snapshots(&directory).expect("leer snapshots");
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].date, "2026-11-08");
        assert_eq!(snapshots[1].date, "2026-11-07");

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn retention_keeps_newest_snapshots() {
        let directory = temp_dir("retention");
        for day in 1..=9 {
            let date = format!("2026-11-{day:02}");
            let snapshot = directory.join(date);
            fs::create_dir_all(&snapshot).unwrap();
            fs::write(snapshot.join(MAIN_BACKUP_NAME), b"db").unwrap();
        }

        prune_snapshots(&directory).expect("podar snapshots");
        let remaining = date_directories(&directory).unwrap();
        assert_eq!(remaining.len(), SNAPSHOT_RETENTION);
        assert!(!remaining.contains(&"2026-11-01".to_string()));
        assert!(remaining.contains(&"2026-11-09".to_string()));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn lock_rejects_a_second_instance() {
        let directory = temp_dir("lock");
        let first = acquire_lock(&directory).expect("primer lock");
        let second = acquire_lock(&directory).expect_err("segundo lock debe fallar");
        assert!(second.contains("otra instancia"));

        drop(first);
        let third = acquire_lock(&directory).expect("tercer lock tras liberar");
        drop(third);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn vault_session_opens_an_empty_vault_with_attachments_dir() {
        let directory = temp_dir("session");
        let session = VaultSession::open(directory.clone()).expect("abrir bóveda");

        assert!(session.status.ready);
        assert!(!session.status.has_data);
        assert!(session.layout.attachments.is_dir());
        assert!(session.layout.backup_database.exists());
        assert_eq!(session.layout.device, device_name());
        drop(session);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn export_import_round_trip_uses_the_same_bytes() {
        let directory = temp_dir("transfer");
        let source_database = directory.join("source.db");
        let attachments = directory.join("attachments");
        let export_path = directory.join("export.txt");
        let reexport_path = directory.join("reexport.txt");

        let mut source = Store::open(&source_database).expect("abrir origen");
        source.migrate().expect("migrar origen");
        source.create_task("Tarea exportada").expect("crear tarea");
        export_to_path(&source, &attachments, &export_path).expect("exportar");
        let first = fs::read(&export_path).expect("leer exportación");
        drop(source);

        let target_database = directory.join("target.db");
        let mut target = Store::open(&target_database).expect("abrir destino");
        target.migrate().expect("migrar destino");
        let report = import_from_path(&target, &attachments, &export_path, ImportMode::EmptyOnly)
            .expect("importar");
        assert_eq!(report.attachments, 0);
        let tasks = target.list_tasks().expect("listar tareas importadas");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Tarea exportada");
        export_to_path(&target, &attachments, &reexport_path).expect("re-exportar");
        let second = fs::read(&reexport_path).expect("leer re-exportación");
        assert_eq!(first, second);
        drop(target);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn local_log_never_contains_absolute_paths() {
        let directory = temp_dir("log");
        let database = directory.join("study-planner.db");
        let mut store = Store::open(&database).expect("abrir base");
        store.migrate().expect("migrar");
        let layout = VaultLayout {
            root: directory.clone(),
            omega: directory.clone(),
            backup_dir: directory.join("backup"),
            backup_database: directory.join("backup").join(MAIN_BACKUP_NAME),
            longlife: directory.join("longlife"),
            database: database.clone(),
            attachments: directory.join("attachments"),
            device: "equipo".to_string(),
        };

        append_log(&layout, &store, "export", "file=export.txt rows=1");
        let raw = fs::read_to_string(directory.join(LOG_FILE_NAME)).expect("leer log");
        assert!(raw.contains("event=export"));
        assert!(!raw.contains(directory.to_str().unwrap()));
        drop(store);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn restore_database_replaces_the_live_database() {
        let directory = temp_dir("restore");
        let live = directory.join("study-planner.db");
        let backup = directory.join("pre-migration.db");

        let mut store = Store::open(&live).expect("abrir base viva");
        store.migrate().expect("migrar");
        store.create_task("Original").expect("crear tarea");
        store.backup_to(&backup).expect("crear backup previo");
        store
            .create_task("Posterior")
            .expect("crear tarea posterior");
        drop(store);

        restore_database(&live, &backup).expect("restaurar backup");

        let restored = Store::open(&live).expect("abrir base restaurada");
        let tasks = restored.list_tasks().expect("listar tareas");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Original");
        drop(restored);

        fs::remove_dir_all(directory).unwrap();
    }
}
