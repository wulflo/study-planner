import { useEffect, useRef, useState } from "react";
import { ResizeHandles } from "./components/ResizeHandles";
import { WindowControls } from "./components/WindowControls";
import {
  type AppInfo,
  chooseVault,
  createBackup,
  createTask,
  deleteTask,
  exportVault,
  getAppInfo,
  getVaultStatus,
  importVault,
  listTasks,
  restoreMainBackup,
  runningInTauri,
  runSchemaSelfCheck,
  type SchemaCheck,
  setTaskDone,
  type Task,
  type VaultStatus,
} from "./lib/ipc";
import { AcademicView } from "./views/AcademicView";
import { CalendarView } from "./views/CalendarView";
import { ExamsView } from "./views/ExamsView";
import { GradesView } from "./views/GradesView";
import { ScheduleView } from "./views/ScheduleView";
import { SettingsView } from "./views/SettingsView";
import { StatsView } from "./views/StatsView";
import { StrengthsView } from "./views/StrengthsView";
import { StudyView } from "./views/StudyView";
import { TasksView } from "./views/TasksView";
import { TodaySchedule } from "./views/TodaySchedule";
import "./styles.css";

const navItems = [
  { id: "today", label: "Hoy", icon: "today" },
  { id: "subjects", label: "Materias", icon: "subjects" },
  { id: "schedule", label: "Horario", icon: "schedule" },
  { id: "calendar", label: "Calendario", icon: "calendar" },
  { id: "tasks", label: "Tareas", icon: "tasks" },
  { id: "exams", label: "Exámenes", icon: "exams" },
  { id: "grades", label: "Notas", icon: "grades" },
  { id: "study", label: "Estudio", icon: "study" },
  { id: "strengths", label: "Fortalezas", icon: "strengths" },
  { id: "stats", label: "Estadísticas", icon: "stats" },
  { id: "vault", label: "Bóveda", icon: "vault" },
  { id: "settings", label: "Ajustes", icon: "settings" },
] as const;

type SectionId = (typeof navItems)[number]["id"];

const projectChecklist = [
  { done: true, label: "Scaffold Tauri 2 + React + TypeScript" },
  { done: true, label: "Esquema SQLite v1 y migración con checksum" },
  { done: true, label: "Permisos mínimos y CSP base" },
  { done: true, label: "Bóveda local elegible y tareas persistentes" },
  { done: true, label: "Contrato de exportación v1" },
  { done: true, label: "Backups verificados y restauración" },
  { done: true, label: "Exportación e importación canónica" },
  { done: true, label: "Materias, horario, rotaciones y asistencia" },
  { done: true, label: "Exámenes, notas, tareas avanzadas y calendario" },
  { done: true, label: "Autoevaluación, fortalezas y repaso SM-2" },
  { done: true, label: "Pomodoro, prioridad explicable y estadísticas" },
];

const iconPaths: Record<string, string> = {
  today:
    "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Zm2 3v2h10V7H7Zm0 4v2h4v-2H7Zm6 0v2h4v-2h-4Zm-6 4v2h4v-2H7Z",
  subjects:
    "M6 3h11a1 1 0 0 1 1 1v16l-6.5-3.5L5 20V4a1 1 0 0 1 1-1Zm1 2v12.5l4.5-2.4 4.5 2.4V5H7Z",
  schedule: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm1 4v5l3 2-1 1.7-4-2.7V7h2Z",
  calendar:
    "M7 3h2v2h6V3h2v2h2a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h2V3Zm12 7H5v9h14v-9ZM6 11h3v3H6v-3Z",
  tasks:
    "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Zm3 4v2h2V8H8Zm4 0v2h5V8h-5Zm-4 4v2h2v-2H8Zm4 0v2h5v-2h-5Z",
  exams: "M6 3h9l4 4v14H6V3Zm8 2v3h3l-3-3ZM8 12h8v2H8v-2Zm0 4h8v2H8v-2Z",
  grades: "M4 20V10h4v10H4Zm6 0V4h4v16h-4Zm6 0v-7h4v7h-4Z",
  study:
    "M12 5.5C10 4.2 7.6 3.6 5 3.6H3v14h2c2.6 0 5 .6 7 1.9 2-1.3 4.4-1.9 7-1.9h2v-14h-2c-2.6 0-5 .6-7 1.9Zm1 12.2V7.2c1.6-.8 3.3-1.2 5-1.2v10.5c-1.8 0-3.5.4-5 1.2Z",
  strengths:
    "M12 3l2.7 5.5 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1L3.2 9.4l6.1-.9L12 3Z",
  stats: "M4 19h16v2H4v-2Zm1-7h3v5H5v-5Zm5-5h3v10h-3V7Zm5 3h3v7h-3v-7Z",
  vault:
    "M12 2l8 4v6c0 5-3.4 8.6-8 10-4.6-1.4-8-5-8-10V6l8-4Zm0 3.2L7 7.5V12c0 3.4 2 6 5 7.2 3-1.2 5-3.8 5-7.2V7.5l-5-2.3Z",
  settings:
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm8.4 4a6.6 6.6 0 0 0-.1-1l2-1.5-2-3.4-2.3.9a7.8 7.8 0 0 0-1.7-1L16 3.5h-4l-.4 2.4a7.8 7.8 0 0 0-1.7 1l-2.3-.9-2 3.4 2 1.5a6.6 6.6 0 0 0 0 2l-2 1.5 2 3.4 2.3-.9c.5.4 1.1.8 1.7 1l.4 2.4h4l.4-2.4c.6-.2 1.2-.6 1.7-1l2.3.9 2-3.4-2-1.5c.1-.3.1-.7.1-1Z",
};

const dateFormatter = new Intl.DateTimeFormat("es", {
  day: "numeric",
  month: "short",
});

const dateTimeFormatter = new Intl.DateTimeFormat("es", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});

function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "" : dateFormatter.format(date);
}

function formatDateTime(value: string | number): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "" : dateTimeFormatter.format(date);
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function Icon({ name }: { name: string }) {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d={iconPaths[name] ?? iconPaths.today} />
    </svg>
  );
}

function ChecklistDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    closeRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("keydown", onKeyDown);

    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <div className="modal-backdrop">
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="checklist-title"
      >
        <div className="modal-head">
          <div>
            <p className="eyebrow">Proyecto</p>
            <h2 id="checklist-title">Checklist persistente</h2>
          </div>
          <button
            ref={closeRef}
            type="button"
            className="icon-button"
            aria-label="Cerrar checklist"
            onClick={onClose}
          >
            ×
          </button>
        </div>
        <p className="modal-copy">
          Resumen del estado actual. El detalle completo vive en{" "}
          <code>docs/CHECKLIST.md</code>.
        </p>
        <ul className="checklist modal-checklist">
          {projectChecklist.map((item) => (
            <li key={item.label} className={item.done ? "done" : undefined}>
              {item.label}
            </li>
          ))}
        </ul>
        <div className="modal-actions">
          <button type="button" className="button filled" onClick={onClose}>
            Cerrar
          </button>
        </div>
      </div>
    </div>
  );
}

function TaskItem({
  task,
  onToggle,
  onDelete,
}: {
  task: Task;
  onToggle: (task: Task) => void;
  onDelete?: (task: Task) => void;
}) {
  const done = task.status === "done";
  const action = done ? "Reabrir" : "Completar";

  return (
    <li className={`task-row${done ? " is-done" : ""}`}>
      <input
        type="checkbox"
        className="task-check"
        checked={done}
        onChange={() => onToggle(task)}
        aria-label={`${action} «${task.title}»`}
      />
      <span className="task-title">{task.title}</span>
      <span className="task-meta">
        {done
          ? "Hecha"
          : task.status === "in_progress"
            ? "En curso"
            : formatDate(task.createdAt)}
      </span>
      {onDelete ? (
        <button
          type="button"
          className="task-delete"
          onClick={() => onDelete(task)}
          aria-label={`Eliminar «${task.title}»`}
        >
          ×
        </button>
      ) : null}
    </li>
  );
}

function TodayView({
  info,
  schema,
  error,
  vault,
  tasks,
  taskError,
  onOpenChecklist,
  onOpenTasks,
  onOpenVault,
  onGoSubjects,
  onToggleTask,
}: {
  info: AppInfo | null;
  schema: SchemaCheck | null;
  error: string | null;
  vault: VaultStatus | null;
  tasks: Task[];
  taskError: string | null;
  onOpenChecklist: () => void;
  onOpenTasks: () => void;
  onOpenVault: () => void;
  onGoSubjects: () => void;
  onToggleTask: (task: Task) => void;
}) {
  const ready = vault?.ready === true;
  const openTasks = tasks.filter((task) => task.status !== "done");

  return (
    <>
      <section className="hero">
        <p className="eyebrow">
          {ready
            ? `Bóveda local · ${vault?.name ?? "abierta"}`
            : "Primer paso · Hoy"}
        </p>
        <h1>
          {ready
            ? "Tu año académico, en una bóveda local."
            : "Empezá por elegir tu bóveda local."}
        </h1>
        <p className="hero-copy">
          {ready
            ? "Tareas, horarios, exámenes y notas se guardan en la carpeta que elegiste. Sin nube, sin cuentas y sin que tus datos salgan del equipo."
            : "Study Planner guarda todo en una carpeta de tu equipo: los datos vivos en omega y los snapshots en longlife. Sin nube, sin cuentas y sin telemetría."}
        </p>
        <div className="hero-actions">
          <button
            type="button"
            className="button filled"
            onClick={ready ? onOpenTasks : onOpenVault}
          >
            {ready ? "Ver tareas" : "Configurar bóveda"}
          </button>
          <button
            type="button"
            className="button tonal"
            onClick={onOpenChecklist}
          >
            Ver checklist
          </button>
        </div>
      </section>

      {ready ? (
        <TodaySchedule vault={vault} onGoSubjects={onGoSubjects} />
      ) : null}

      {ready ? (
        <section className="status-card" aria-labelledby="pending-title">
          <div>
            <p className="eyebrow">Hoy</p>
            <h2 id="pending-title">Pendientes</h2>
            <p>
              {openTasks.length === 0
                ? "No hay tareas abiertas."
                : `${openTasks.length} ${openTasks.length === 1 ? "tarea abierta" : "tareas abiertas"}.`}
            </p>
            <button
              type="button"
              className="button tonal"
              onClick={onOpenTasks}
            >
              Gestionar tareas
            </button>
          </div>
          {openTasks.length === 0 ? (
            <p className="notice">
              Creá la primera tarea en la sección Tareas o desde el botón de
              arriba.
            </p>
          ) : (
            <ul className="task-list compact" aria-label="Tareas pendientes">
              {openTasks.slice(0, 4).map((task) => (
                <TaskItem key={task.id} task={task} onToggle={onToggleTask} />
              ))}
            </ul>
          )}
        </section>
      ) : null}

      <section className="cards" aria-label="Principios del proyecto">
        <article className="card">
          <span className="card-kicker">Datos</span>
          <h2>Bóveda local</h2>
          <p>
            Datos vivos en <code>&lt;bóveda&gt;/omega</code> y snapshots futuros
            en <code>&lt;bóveda&gt;/longlife</code>.
          </p>
        </article>
        <article className="card">
          <span className="card-kicker">Portabilidad</span>
          <h2>Exportación total</h2>
          <p>
            Un único <code>.txt</code> canónico, con checksum y round-trip byte
            a byte.
          </p>
        </article>
        <article className="card">
          <span className="card-kicker">Privacidad</span>
          <h2>Sin red</h2>
          <p>
            La webview no tiene permisos de archivos, SQL ni HTTP. El único
            módulo de red será el updater.
          </p>
        </article>
        <article className="card">
          <span className="card-kicker">Continuidad</span>
          <h2>Reanudable</h2>
          <p>
            Cada fase deja tests, changelog y un checklist para retomar sin
            romper lo anterior.
          </p>
        </article>
      </section>

      <section className="status-card" aria-labelledby="status-title">
        <div>
          <p className="eyebrow">Autodiagnóstico</p>
          <h2 id="status-title">Estado técnico</h2>
        </div>
        {!runningInTauri ? (
          <p className="notice">
            Modo navegador: el IPC de Tauri no está disponible. Abrí la app con{" "}
            <code>npm run tauri dev</code> para ejecutar el autodiagnóstico.
          </p>
        ) : error ? (
          <p className="notice error">{error}</p>
        ) : (
          <dl className="status-grid">
            <div>
              <dt>Esquema</dt>
              <dd>{schema ? `v${schema.schemaVersion}` : "…"}</dd>
            </div>
            <div>
              <dt>Integridad</dt>
              <dd>{schema?.integrity ?? "…"}</dd>
            </div>
            <div>
              <dt>Claves foráneas</dt>
              <dd>
                {schema ? `${schema.foreignKeyViolations} violaciones` : "…"}
              </dd>
            </div>
            <div>
              <dt>Formato export</dt>
              <dd>{info ? `v${info.exportFormatVersion}` : "…"}</dd>
            </div>
            <div className="span-2">
              <dt>ID de bóveda de prueba</dt>
              <dd className="mono">{schema?.vaultId ?? "…"}</dd>
            </div>
          </dl>
        )}
      </section>

      <section className="next-card" aria-labelledby="next-title">
        <p className="eyebrow">Siguiente</p>
        <h2 id="next-title">Primer flujo funcional</h2>
        <ul className="checklist">
          <li className="done">Bóveda local con SQLite propia</li>
          <li className="done">Tareas con crear, completar y eliminar</li>
          <li>Backups verificados en longlife</li>
          <li>Horario, rotaciones y asistencia</li>
        </ul>
        {taskError ? <p className="notice error">{taskError}</p> : null}
      </section>
    </>
  );
}

function VaultView({
  vault,
  error,
  notice,
  choosing,
  backingUp,
  exporting,
  importing,
  restoring,
  onChoose,
  onCreateBackup,
  onExport,
  onImport,
  onRestore,
}: {
  vault: VaultStatus | null;
  error: string | null;
  notice: string | null;
  choosing: boolean;
  backingUp: boolean;
  exporting: boolean;
  importing: boolean;
  restoring: boolean;
  onChoose: () => void;
  onCreateBackup: () => void;
  onExport: () => void;
  onImport: () => void;
  onRestore: () => void;
}) {
  const latestSnapshot = vault?.snapshots[0] ?? null;
  const [confirmingRestore, setConfirmingRestore] = useState(false);
  const busy = choosing || backingUp || exporting || importing || restoring;

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Bóveda local · Fase 1</p>
        <h1>Bóveda local</h1>
        <p className="hero-copy">
          Elegí una carpeta raíz fuera del repositorio. Study Planner valida la
          ruta, bloquea la bóveda contra otras instancias y mantiene backups
          verificados; la ruta completa no se expone a la interfaz.
        </p>
        <div className="hero-actions">
          <button
            type="button"
            className="button filled"
            onClick={onChoose}
            disabled={!runningInTauri || choosing}
            title={
              runningInTauri
                ? undefined
                : "El selector de carpetas solo funciona en la app de escritorio"
            }
          >
            {choosing
              ? "Abriendo selector…"
              : vault?.ready
                ? "Cambiar carpeta"
                : "Elegir carpeta"}
          </button>
        </div>
      </section>

      {!runningInTauri ? (
        <p className="notice">
          Modo navegador: el selector de carpetas solo funciona en la app de
          escritorio con <code>npm run tauri dev</code>.
        </p>
      ) : error ? (
        <p className="notice error" role="alert">
          {error}
        </p>
      ) : notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      <section className="vault-grid" aria-label="Partes de la bóveda">
        <article className="card">
          <span className="card-kicker">Estado</span>
          <h2>
            {vault?.ready
              ? (vault.name ?? "Bóveda abierta")
              : vault?.message
                ? "No se pudo abrir"
                : "Sin configurar"}
          </h2>
          <p>
            {vault?.ready
              ? "La base está abierta, bloqueada para esta instancia y lista para guardar tareas."
              : (vault?.message ??
                "Todavía no elegiste una carpeta raíz para la bóveda.")}
          </p>
        </article>
        <article className="card">
          <span className="card-kicker">Datos vivos</span>
          <h2>omega</h2>
          <p>
            Base SQLite con tareas, horarios, notas y ajustes, más el backup
            principal verificado. Es la única fuente de verdad.
          </p>
        </article>
        <article className="card">
          <span className="card-kicker">Continuidad</span>
          <h2>longlife</h2>
          <p>
            Snapshots diarias verificadas por dispositivo. Se conservan las
            últimas 7 y las incompletas se descartan.
          </p>
        </article>
      </section>

      {vault?.ready ? (
        <section className="status-card" aria-labelledby="backup-title">
          <div>
            <p className="eyebrow">Respaldos</p>
            <h2 id="backup-title">Backup principal</h2>
            <p>
              Copia online de SQLite con SHA-256, <code>integrity_check</code> y{" "}
              <code>foreign_key_check</code> antes de darla por válida.
            </p>
            <div className="hero-actions">
              <button
                type="button"
                className="button tonal"
                onClick={onCreateBackup}
                disabled={busy}
              >
                {backingUp ? "Creando backup…" : "Crear backup ahora"}
              </button>
              <button
                type="button"
                className="button tonal"
                onClick={() => setConfirmingRestore(true)}
                disabled={busy || !vault.backup}
              >
                Restaurar backup principal
              </button>
            </div>
            {confirmingRestore ? (
              <div
                className="confirm-card"
                role="alertdialog"
                aria-labelledby="restore-confirm-title"
              >
                <p id="restore-confirm-title">
                  Se reemplazarán los datos vivos por el backup principal
                  verificado. Antes se crea un backup <code>pre-restore-*</code>{" "}
                  del estado actual.
                </p>
                <div className="hero-actions">
                  <button
                    type="button"
                    className="button"
                    onClick={() => setConfirmingRestore(false)}
                    disabled={restoring}
                  >
                    Cancelar
                  </button>
                  <button
                    type="button"
                    className="button filled"
                    onClick={() => {
                      setConfirmingRestore(false);
                      onRestore();
                    }}
                    disabled={restoring}
                  >
                    {restoring ? "Restaurando…" : "Sí, restaurar backup"}
                  </button>
                </div>
              </div>
            ) : null}
          </div>
          <dl className="status-grid">
            <div>
              <dt>Tamaño</dt>
              <dd>
                {vault.backup ? formatBytes(vault.backup.sizeBytes) : "—"}
              </dd>
            </div>
            <div>
              <dt>Integridad</dt>
              <dd>{vault.backup?.integrity ?? "—"}</dd>
            </div>
            <div>
              <dt>Modificado</dt>
              <dd>
                {vault.backup
                  ? formatDateTime(vault.backup.modifiedAtMillis)
                  : "—"}
              </dd>
            </div>
            <div>
              <dt>Snapshots</dt>
              <dd>
                {vault.snapshots.length} / 7
                {latestSnapshot ? ` · ${latestSnapshot.date}` : ""}
              </dd>
            </div>
            <div className="span-2">
              <dt>SHA-256 del backup</dt>
              <dd className="mono">
                {vault.backup
                  ? `${vault.backup.sha256.slice(0, 24)}…`
                  : "Todavía no hay backup"}
              </dd>
            </div>
          </dl>
        </section>
      ) : null}

      {vault?.ready ? (
        <section className="status-card" aria-labelledby="transfer-title">
          <div>
            <p className="eyebrow">Portabilidad</p>
            <h2 id="transfer-title">Exportar e importar</h2>
            <p>
              La exportación es un único <code>.txt</code> canónico con todas
              las tablas, adjuntos y checksum. La importación solo se permite en
              una bóveda vacía y es transaccional: si algo falla, no deja datos
              a medias.
            </p>
            <p className="notice">
              El archivo no está cifrado: contiene todos tus datos en texto
              plano, incluidos los adjuntos. Guardalo en un lugar seguro.
            </p>
            <div className="hero-actions">
              <button
                type="button"
                className="button tonal"
                onClick={onExport}
                disabled={busy}
              >
                {exporting ? "Exportando…" : "Exportar bóveda"}
              </button>
              <button
                type="button"
                className="button tonal"
                onClick={onImport}
                disabled={busy || vault.hasData}
                title={
                  vault.hasData
                    ? "La importación total solo se permite en una bóveda vacía"
                    : undefined
                }
              >
                {importing ? "Importando…" : "Importar en bóveda vacía"}
              </button>
            </div>
            {vault.hasData ? (
              <p className="notice">
                Esta bóveda ya tiene datos. Para importar una exportación, elegí
                una carpeta nueva y vacía.
              </p>
            ) : (
              <p className="notice">
                Bóveda vacía: podés importar un <code>.txt</code> sin miedo a
                pisar datos existentes.
              </p>
            )}
          </div>
          <dl className="status-grid">
            <div>
              <dt>Datos de dominio</dt>
              <dd>{vault.hasData ? "Presentes" : "Ninguno"}</dd>
            </div>
            <div>
              <dt>Importación</dt>
              <dd>{vault.hasData ? "Bloqueada" : "Disponible"}</dd>
            </div>
            <div className="span-2">
              <dt>Garantía de round-trip</dt>
              <dd>
                Exportar → importar en bóveda vacía → exportar produce bytes
                idénticos dentro de la misma versión.
              </dd>
            </div>
          </dl>
        </section>
      ) : null}

      {vault?.ready ? (
        <section className="status-card" aria-labelledby="vault-title">
          <div>
            <p className="eyebrow">Diagnóstico</p>
            <h2 id="vault-title">Bóveda abierta</h2>
            <p>Integridad verificada por SQLite.</p>
          </div>
          <dl className="status-grid">
            <div>
              <dt>Esquema</dt>
              <dd>v{vault.schemaVersion}</dd>
            </div>
            <div>
              <dt>Integridad</dt>
              <dd>{vault.integrity}</dd>
            </div>
            <div>
              <dt>Claves foráneas</dt>
              <dd>{vault.foreignKeyViolations} violaciones</dd>
            </div>
            <div>
              <dt>Dispositivo</dt>
              <dd>{vault.device ?? "—"}</dd>
            </div>
            <div className="span-2">
              <dt>ID de bóveda</dt>
              <dd className="mono">{vault.vaultId}</dd>
            </div>
          </dl>
        </section>
      ) : null}
    </>
  );
}

function App() {
  const [active, setActive] = useState<SectionId>("today");
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [schema, setSchema] = useState<SchemaCheck | null>(null);
  const [vault, setVault] = useState<VaultStatus | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [vaultError, setVaultError] = useState<string | null>(null);
  const [vaultNotice, setVaultNotice] = useState<string | null>(null);
  const [taskError, setTaskError] = useState<string | null>(null);
  const [choosing, setChoosing] = useState(false);
  const [backingUp, setBackingUp] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [checklistOpen, setChecklistOpen] = useState(false);

  useEffect(() => {
    if (!runningInTauri) {
      return;
    }

    let cancelled = false;

    async function bootstrap() {
      try {
        const [appInfo, check] = await Promise.all([
          getAppInfo(),
          runSchemaSelfCheck(),
        ]);
        if (!cancelled) {
          setInfo(appInfo);
          setSchema(check);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }

      try {
        const status = await getVaultStatus();
        if (cancelled) {
          return;
        }
        setVault(status);
        if (status.ready) {
          const stored = await listTasks();
          if (!cancelled) {
            setTasks(stored);
          }
        }
      } catch (reason) {
        if (!cancelled) {
          setVaultError(messageOf(reason));
        }
      }
    }

    void bootstrap();

    return () => {
      cancelled = true;
    };
  }, []);

  async function handleChooseVault() {
    setChoosing(true);
    setVaultError(null);
    setVaultNotice(null);
    try {
      const status = await chooseVault();
      if (!status) {
        return;
      }
      setVault(status);
      setTasks(status.ready ? await listTasks() : []);
      setTaskError(null);
    } catch (reason) {
      setVaultError(messageOf(reason));
    } finally {
      setChoosing(false);
    }
  }

  async function handleCreateBackup() {
    setBackingUp(true);
    setVaultError(null);
    setVaultNotice(null);
    try {
      const status = await createBackup();
      setVault(status);
      setVaultNotice(
        "Backup creado y verificado con SHA-256 e integrity_check.",
      );
    } catch (reason) {
      setVaultError(messageOf(reason));
    } finally {
      setBackingUp(false);
    }
  }

  async function handleExportVault() {
    setExporting(true);
    setVaultError(null);
    setVaultNotice(null);
    try {
      const report = await exportVault();
      if (!report) {
        return;
      }
      setVaultNotice(
        `Exportación lista: ${report.fileName} · ${report.rows} filas · SHA-256 ${report.sha256.slice(0, 16)}…`,
      );
    } catch (reason) {
      setVaultError(messageOf(reason));
    } finally {
      setExporting(false);
    }
  }

  async function handleImportVault() {
    setImporting(true);
    setVaultError(null);
    setVaultNotice(null);
    try {
      const outcome = await importVault();
      if (!outcome) {
        return;
      }
      setVault(outcome.status);
      setTasks(await listTasks());
      setTaskError(null);
      setVaultNotice(
        `Importación completada desde ${outcome.fileName}: ${outcome.rows} filas y ${outcome.attachments} adjuntos.`,
      );
    } catch (reason) {
      setVaultError(messageOf(reason));
    } finally {
      setImporting(false);
    }
  }

  async function handleRestoreBackup() {
    setRestoring(true);
    setVaultError(null);
    setVaultNotice(null);
    try {
      const status = await restoreMainBackup();
      setVault(status);
      setTasks(await listTasks());
      setTaskError(null);
      setVaultNotice(
        "Backup principal restaurado. La bóveda se reabrió y quedó un backup pre-restore del estado anterior.",
      );
    } catch (reason) {
      setVaultError(messageOf(reason));
    } finally {
      setRestoring(false);
    }
  }

  async function handleCreateTask(title: string): Promise<boolean> {
    setTaskError(null);
    try {
      await createTask(title);
      setTasks(await listTasks());
      return true;
    } catch (reason) {
      setTaskError(messageOf(reason));
      return false;
    }
  }

  async function handleToggleTask(task: Task) {
    setTaskError(null);
    try {
      await setTaskDone(task.id, task.status !== "done");
      setTasks(await listTasks());
    } catch (reason) {
      setTaskError(messageOf(reason));
    }
  }

  async function handleDeleteTask(task: Task) {
    setTaskError(null);
    try {
      await deleteTask(task.id);
      setTasks(await listTasks());
    } catch (reason) {
      setTaskError(messageOf(reason));
    }
  }

  return (
    <div className="app">
      <ResizeHandles />
      <header className="topbar" data-tauri-drag-region="deep">
        <div className="brand">
          <img
            src="/study-planner.png"
            alt=""
            className="brand-mark"
            width={30}
            height={30}
          />
          <span className="brand-name">Study Planner</span>
          <span className="phase-chip">Local</span>
        </div>
        <div className="topbar-end">
          <div className="topbar-meta">
            <span className="quiet">Local · sin cuentas · sin telemetría</span>
            <span className="version">
              {info ? `v${info.version}` : "v0.1.0"}
            </span>
          </div>
          <WindowControls />
        </div>
      </header>

      <div className="shell">
        <nav className="nav-rail" aria-label="Navegación principal">
          {navItems.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`nav-item${active === item.id ? " is-active" : ""}`}
              aria-current={active === item.id ? "page" : undefined}
              onClick={() => setActive(item.id)}
            >
              <Icon name={item.icon} />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <main className="main">
          {active === "today" ? (
            <TodayView
              info={info}
              schema={schema}
              error={error}
              vault={vault}
              tasks={tasks}
              taskError={taskError}
              onOpenChecklist={() => setChecklistOpen(true)}
              onOpenTasks={() => setActive("tasks")}
              onOpenVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
              onToggleTask={handleToggleTask}
            />
          ) : null}

          {active === "tasks" ? (
            <TasksView
              vault={vault}
              tasks={tasks}
              error={taskError}
              onGoVault={() => setActive("vault")}
              onCreate={handleCreateTask}
              onToggle={handleToggleTask}
              onDelete={handleDeleteTask}
              onTasksChanged={setTasks}
            />
          ) : null}

          {active === "subjects" ? (
            <AcademicView vault={vault} onGoVault={() => setActive("vault")} />
          ) : null}

          {active === "schedule" ? (
            <ScheduleView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
            />
          ) : null}

          {active === "calendar" ? (
            <CalendarView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
              onBack={() => setActive("today")}
            />
          ) : null}

          {active === "exams" ? (
            <ExamsView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
            />
          ) : null}

          {active === "grades" ? (
            <GradesView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
            />
          ) : null}

          {active === "study" ? (
            <StudyView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
              onGoStrengths={() => setActive("strengths")}
            />
          ) : null}

          {active === "strengths" ? (
            <StrengthsView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoSubjects={() => setActive("subjects")}
              onGoExams={() => setActive("exams")}
            />
          ) : null}

          {active === "stats" ? (
            <StatsView
              vault={vault}
              onGoVault={() => setActive("vault")}
              onGoStrengths={() => setActive("strengths")}
            />
          ) : null}

          {active === "vault" ? (
            <VaultView
              vault={vault}
              error={vaultError ?? vault?.message ?? null}
              notice={vaultNotice}
              choosing={choosing}
              backingUp={backingUp}
              exporting={exporting}
              importing={importing}
              restoring={restoring}
              onChoose={handleChooseVault}
              onCreateBackup={handleCreateBackup}
              onExport={handleExportVault}
              onImport={handleImportVault}
              onRestore={handleRestoreBackup}
            />
          ) : null}

          {active === "settings" ? (
            <SettingsView vault={vault} onGoVault={() => setActive("vault")} />
          ) : null}
        </main>
      </div>

      <footer className="footer">
        <span>Study Planner · datos locales</span>
        <span>{runningInTauri ? "IPC conectado" : "Render en navegador"}</span>
      </footer>

      <ChecklistDialog
        open={checklistOpen}
        onClose={() => setChecklistOpen(false)}
      />
    </div>
  );
}

export default App;
