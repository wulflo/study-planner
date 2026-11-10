import { type CSSProperties, useEffect, useState } from "react";
import {
  type BackgroundStatus,
  checkUpdates,
  getBackgroundStatus,
  getNotificationStatus,
  getUpdaterStatus,
  installUpdate,
  listNetworkEvents,
  type NetworkEvent,
  type NotificationStatus,
  rollbackUpdate,
  runNotificationScan,
  sendTestNotification,
  setBackgroundMode,
  setNotificationsEnabled,
  setNotificationsSound,
  setUpdaterAutoCheck,
  type UpdateInfo,
  type UpdaterRuntime,
  type UpdaterStatus,
  type VaultStatus,
} from "../lib/ipc";
import {
  ACCENTS,
  type AccentId,
  THEME_MODES,
  type ThemeMode,
} from "../lib/theme";

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

const kindLabels: Record<string, string> = {
  update_check: "Comprobación",
  update_download: "Descarga",
  update_install: "Instalación",
  update_rollback: "Rollback",
};

const runtimeLabels: Record<UpdaterRuntime, string> = {
  appimage: "AppImage",
  snap: "Snap",
  deb: "paquete .deb",
  rpm: "paquete .rpm",
  windows: "Windows",
  macos: "macOS",
  development: "desarrollo",
};

const notificationKindLabels: Record<string, string> = {
  class: "Clase",
  task: "Tarea",
  exam: "Examen",
  review: "Repaso",
  backup: "Backup",
  update: "Actualización",
};

function bytesText(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} kB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function eventTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? value : date.toLocaleString("es-AR");
}

export function SettingsView({
  vault,
  onGoVault,
  themeMode,
  accent,
  onThemeModeChange,
  onAccentChange,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  themeMode: ThemeMode;
  accent: AccentId;
  onThemeModeChange: (mode: ThemeMode) => void;
  onAccentChange: (accent: AccentId) => void;
}) {
  const ready = vault?.ready === true;
  const [status, setStatus] = useState<UpdaterStatus | null>(null);
  const [background, setBackground] = useState<BackgroundStatus | null>(null);
  const [events, setEvents] = useState<NetworkEvent[]>([]);
  const [notifications, setNotifications] = useState<NotificationStatus | null>(
    null,
  );
  const [candidate, setCandidate] = useState<UpdateInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [notifying, setNotifying] = useState(false);

  async function loadCore() {
    const [loadedStatus, loadedBackground] = await Promise.all([
      getUpdaterStatus(),
      getBackgroundStatus(),
    ]);
    setStatus(loadedStatus);
    setBackground(loadedBackground);
  }

  async function loadVaultData() {
    const [loadedEvents, loadedNotifications] = await Promise.all([
      listNetworkEvents(),
      getNotificationStatus(),
    ]);
    setEvents(loadedEvents);
    setNotifications(loadedNotifications);
  }

  async function load() {
    try {
      await loadCore();
      if (ready) {
        await loadVaultData();
      }
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function loadInitial() {
      try {
        const [loadedStatus, loadedBackground] = await Promise.all([
          getUpdaterStatus(),
          getBackgroundStatus(),
        ]);
        if (!cancelled) {
          setStatus(loadedStatus);
          setBackground(loadedBackground);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }

      if (!ready) {
        return;
      }

      try {
        const [loadedEvents, loadedNotifications] = await Promise.all([
          listNetworkEvents(),
          getNotificationStatus(),
        ]);
        if (!cancelled) {
          setEvents(loadedEvents);
          setNotifications(loadedNotifications);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadInitial();
    return () => {
      cancelled = true;
    };
  }, [ready]);

  async function handleToggleAutoCheck(enabled: boolean) {
    setSaving(true);
    setError(null);
    try {
      setStatus(await setUpdaterAutoCheck(enabled));
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setSaving(false);
    }
  }

  async function handleToggleBackground(enabled: boolean) {
    setSaving(true);
    setError(null);
    try {
      setBackground(await setBackgroundMode(enabled));
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setSaving(false);
    }
  }

  async function handleCheck() {
    setChecking(true);
    setError(null);
    setNotice(null);
    setCandidate(null);
    try {
      const found = await checkUpdates();
      setCandidate(found);
      setNotice(
        found
          ? `Hay una actualización disponible: ${found.version}.`
          : "La versión instalada es la más nueva.",
      );
      setEvents(await listNetworkEvents().catch(() => []));
    } catch (reason) {
      setError(messageOf(reason));
      setEvents(await listNetworkEvents().catch(() => []));
    } finally {
      setChecking(false);
    }
  }

  async function handleInstall() {
    setInstalling(true);
    setError(null);
    setNotice(null);
    try {
      const outcome = await installUpdate();
      setCandidate(null);
      setNotice(
        `Actualización ${outcome.version} instalada. ${
          outcome.restartRequired ? "Reiniciá Study Planner para usarla." : ""
        }`,
      );
      await load();
    } catch (reason) {
      setError(messageOf(reason));
      setEvents(await listNetworkEvents().catch(() => []));
    } finally {
      setInstalling(false);
    }
  }

  async function handleRollback() {
    setInstalling(true);
    setError(null);
    setNotice(null);
    try {
      await rollbackUpdate();
      setNotice("Se restauró la versión anterior. Reiniciá Study Planner.");
      await load();
    } catch (reason) {
      setError(messageOf(reason));
      setEvents(await listNetworkEvents().catch(() => []));
    } finally {
      setInstalling(false);
    }
  }

  async function handleToggleNotifications(enabled: boolean) {
    setNotifying(true);
    setError(null);
    try {
      setNotifications(await setNotificationsEnabled(enabled));
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setNotifying(false);
    }
  }

  async function handleToggleSound(enabled: boolean) {
    setNotifying(true);
    setError(null);
    try {
      setNotifications(await setNotificationsSound(enabled));
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setNotifying(false);
    }
  }

  async function handleScanNotifications() {
    setNotifying(true);
    setError(null);
    setNotice(null);
    try {
      const outcome = await runNotificationScan();
      setNotifications(await getNotificationStatus());
      const failed =
        outcome.failed > 0 ? ` ${outcome.failed} no se pudieron entregar.` : "";
      setNotice(
        `Recordatorios revisados: ${outcome.delivered} entregados.${failed}`,
      );
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setNotifying(false);
    }
  }

  async function handleTestNotification() {
    setNotifying(true);
    setError(null);
    setNotice(null);
    try {
      await sendTestNotification();
      setNotice("Notificación de prueba enviada al escritorio.");
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setNotifying(false);
    }
  }

  const busy = saving || checking || installing || notifying;
  const snapManaged = status?.managedBySnap === true;
  const canInstall = status?.canInstall === true;
  const updaterBlockedReason = !status
    ? "Cargando el estado del updater…"
    : snapManaged
      ? "Esta instalación se actualiza desde la Snap Store."
      : !status.publicKeyConfigured
        ? "Esta compilación no tiene una clave pública de producción."
        : !canInstall
          ? status.runtime === "development"
            ? "Esta compilación no se ejecuta desde un paquete publicado."
            : "Esta instalación no admite actualizaciones automáticas."
          : null;
  const canCheckUpdates = !busy && updaterBlockedReason === null;
  const runtimeLabel = status ? runtimeLabels[status.runtime] : "desconocido";

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Ajustes · local y sin telemetría</p>
        <h1>Ajustes</h1>
        <p className="hero-copy">
          Apariencia, preferencias de la app y el updater aislado. La webview no
          hace peticiones de red: todo pasa por Rust, con URL fija y firma
          obligatoria.
        </p>
        <div className="hero-actions">
          {ready ? (
            <span className="status-pill is-ok">
              Bóveda: {vault?.name ?? "lista"}
            </span>
          ) : (
            <button type="button" className="button filled" onClick={onGoVault}>
              Configurar bóveda
            </button>
          )}
        </div>
      </section>

      {error ? (
        <p className="notice error" role="alert">
          {error}
        </p>
      ) : null}
      {notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      <div className="study-layout">
        <section
          className="status-card settings-card appearance-card"
          aria-labelledby="appearance-title"
        >
          <div>
            <p className="eyebrow">Apariencia</p>
            <h2 id="appearance-title">Tema y color</h2>
            <p className="quiet">
              El modo «Sistema» sigue la preferencia del escritorio. La elección
              se guarda en este equipo.
            </p>
          </div>

          <div className="preference-block">
            <span className="preference-label">Modo</span>
            <fieldset className="segmented" aria-label="Modo de color">
              {THEME_MODES.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className={`segment${themeMode === option.id ? " is-selected" : ""}`}
                  aria-pressed={themeMode === option.id}
                  onClick={() => onThemeModeChange(option.id)}
                >
                  {option.label}
                </button>
              ))}
            </fieldset>
          </div>

          <div className="preference-block">
            <span className="preference-label">Color de acento</span>
            <fieldset className="accent-grid" aria-label="Color de acento">
              {ACCENTS.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className={`accent-swatch${accent === option.id ? " is-selected" : ""}`}
                  style={{ "--swatch": option.swatch } as CSSProperties}
                  aria-label={option.label}
                  aria-pressed={accent === option.id}
                  title={option.label}
                  onClick={() => onAccentChange(option.id)}
                >
                  <span className="accent-dot" />
                  <span className="accent-name">{option.label}</span>
                </button>
              ))}
            </fieldset>
          </div>

          <label className="setting-toggle">
            <input
              type="checkbox"
              checked={background?.enabled ?? true}
              disabled={busy || !background}
              onChange={(event) =>
                void handleToggleBackground(event.target.checked)
              }
            />
            <span>
              <strong>Mantener en segundo plano</strong>
              <span className="quiet">
                Al cerrar la ventana, la app sigue en la bandeja y revisa
                recordatorios. Usá «Salir» en el icono de la bandeja para
                cerrarla del todo.
                {background && !background.trayAvailable
                  ? " En este escritorio no se detectó una bandeja; cerrar la ventana cierra la app."
                  : ""}
              </span>
            </span>
          </label>
          {!ready ? (
            <p className="quiet">
              La preferencia funciona ahora mismo; se guardará en la bóveda
              cuando la configures.
            </p>
          ) : null}
        </section>

        <section
          className="status-card settings-card"
          aria-labelledby="updater-title"
        >
          <div>
            <p className="eyebrow">Updater aislado</p>
            <h2 id="updater-title">Actualizaciones</h2>
            <p className="quiet">
              Versión {status?.currentVersion ?? "desconocida"} · host fijo{" "}
              {status?.updateHost ?? "github.com"} · runtime {runtimeLabel}
            </p>
          </div>

          {status && !status.publicKeyConfigured ? (
            <p className="notice">
              Esta compilación no tiene una clave pública de producción. El
              updater queda deshabilitado hasta configurarla; nunca se descarga
              nada sin firma válida.
            </p>
          ) : null}

          {snapManaged ? (
            <p className="notice">
              Esta instalación es un snap. Las actualizaciones las gestiona
              snapd desde la Snap Store; el updater automático queda
              deshabilitado para no reemplazar archivos fuera del confinamiento.
            </p>
          ) : null}

          {status?.runtime === "deb" || status?.runtime === "rpm" ? (
            <p className="quiet">
              La instalación pedirá tu contraseña de administrador y se aplicará
              con el gestor de paquetes del sistema. Después habrá que reiniciar
              Study Planner.
            </p>
          ) : null}

          {status?.runtime === "windows" || status?.runtime === "macos" ? (
            <p className="quiet">
              La app se cerrará para completar la instalación. En macOS se
              reabrirá sola; en Windows puede que tengas que abrirla de nuevo.
            </p>
          ) : null}

          <label className="setting-toggle">
            <input
              type="checkbox"
              checked={status?.autoCheck ?? false}
              disabled={busy || !status || !ready || snapManaged}
              onChange={(event) =>
                void handleToggleAutoCheck(event.target.checked)
              }
            />
            <span>
              <strong>Buscar actualizaciones al iniciar</strong>
              <span className="quiet">
                Desactivado por defecto. Solo consulta el manifiesto fijo; no
                envía datos de la bóveda.
                {!ready ? " Requiere una bóveda configurada." : ""}
              </span>
            </span>
          </label>

          <div className="hero-actions">
            <button
              type="button"
              className="button filled"
              disabled={!canCheckUpdates}
              onClick={() => void handleCheck()}
            >
              {checking ? "Buscando…" : "Buscar actualizaciones"}
            </button>
            {candidate ? (
              <button
                type="button"
                className="button tonal"
                disabled={busy || !canInstall}
                onClick={() => void handleInstall()}
              >
                {installing ? "Instalando…" : `Instalar ${candidate.version}`}
              </button>
            ) : null}
            {status?.canRollback ? (
              <button
                type="button"
                className="button"
                disabled={busy || snapManaged}
                onClick={() => void handleRollback()}
              >
                Restaurar versión anterior
              </button>
            ) : null}
          </div>

          {status &&
          updaterBlockedReason &&
          !snapManaged &&
          status.publicKeyConfigured ? (
            <p className="quiet">{updaterBlockedReason}</p>
          ) : null}

          {candidate ? (
            <div className="nested-card">
              <p className="eyebrow">Nueva versión</p>
              <h3>{candidate.version}</h3>
              {candidate.pubDate ? (
                <p className="quiet">Publicada: {candidate.pubDate}</p>
              ) : null}
              <p>{candidate.notes || "Sin notas de la versión."}</p>
            </div>
          ) : null}
        </section>

        {!ready ? (
          <section className="next-card">
            <p className="eyebrow">Bóveda requerida</p>
            <h2>Primero elegí dónde guardar tus datos</h2>
            <p>
              Los recordatorios y el registro de red se guardan en la bóveda
              local. Elegí una carpeta en la sección Bóveda.
            </p>
            <div className="hero-actions">
              <button
                type="button"
                className="button filled"
                onClick={onGoVault}
              >
                Ir a Bóveda
              </button>
            </div>
          </section>
        ) : (
          <>
            <section
              className="status-card settings-card"
              aria-labelledby="notifications-title"
            >
              <div>
                <p className="eyebrow">Recordatorios locales</p>
                <h2 id="notifications-title">Notificaciones</h2>
                <p className="quiet">
                  Clases, tareas, exámenes y repasos. Se calculan en la bóveda y
                  se entregan por D-Bus desde Rust: la webview no pide permisos
                  ni accede a la red.
                </p>
              </div>

              <label className="setting-toggle">
                <input
                  type="checkbox"
                  checked={notifications?.enabled ?? false}
                  disabled={busy || !notifications}
                  onChange={(event) =>
                    void handleToggleNotifications(event.target.checked)
                  }
                />
                <span>
                  <strong>Activar recordatorios</strong>
                  <span className="quiet">
                    Clases {notifications?.classLeadMinutes ?? 10} min antes ·
                    tareas {notifications?.taskLeadMinutes ?? 60} min antes ·
                    exámenes {notifications?.examLeadHours ?? 24} h antes.
                  </span>
                </span>
              </label>

              <label className="setting-toggle">
                <input
                  type="checkbox"
                  checked={notifications?.sound ?? true}
                  disabled={busy || !notifications}
                  onChange={(event) =>
                    void handleToggleSound(event.target.checked)
                  }
                />
                <span>
                  <strong>Sonido en los recordatorios</strong>
                  <span className="quiet">
                    Pide al escritorio el sonido estándar de notificación. Si no
                    se reproduce, revisá la configuración de sonido del sistema.
                  </span>
                </span>
              </label>

              <div className="hero-actions">
                <button
                  type="button"
                  className="button filled"
                  disabled={busy || !notifications?.enabled}
                  onClick={() => void handleScanNotifications()}
                >
                  {notifying ? "Revisando…" : "Revisar recordatorios"}
                </button>
                <button
                  type="button"
                  className="button tonal"
                  disabled={notifying}
                  onClick={() => void handleTestNotification()}
                >
                  Enviar notificación de prueba
                </button>
              </div>

              {notifications && notifications.recent.length > 0 ? (
                <ul className="mini-list" aria-label="Recordatorios entregados">
                  {notifications.recent.map((record) => (
                    <li key={record.id} className="mini-item">
                      <div>
                        <strong>
                          {notificationKindLabels[record.kind] ?? record.kind}
                        </strong>
                        <span className="quiet">
                          {eventTime(record.firedAt ?? record.scheduledAt)}
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="quiet">
                  Todavía no se entregaron recordatorios. El planificador revisa
                  cada minuto mientras la app está abierta o en segundo plano.
                </p>
              )}
            </section>

            <section
              className="status-card settings-card"
              aria-labelledby="network-title"
            >
              <div>
                <p className="eyebrow">Auditoría</p>
                <h2 id="network-title">Eventos de red</h2>
                <p className="quiet">
                  Solo tipo, host, bytes y resultado. Sin rutas, materias ni
                  identificadores de la bóveda.
                </p>
              </div>

              {events.length === 0 ? (
                <p className="quiet">Todavía no hay eventos de red.</p>
              ) : (
                <ul className="mini-list" aria-label="Eventos de red">
                  {events.map((event) => (
                    <li key={event.id} className="mini-item">
                      <div>
                        <strong>
                          {kindLabels[event.kind] ?? event.kind} ·{" "}
                          {event.outcome === "ok" ? "OK" : "Error"}
                        </strong>
                        <span className="quiet">
                          {event.host} · {bytesText(event.bytesReceived)}{" "}
                          recibidos · {eventTime(event.occurredAt)}
                        </span>
                        {event.detail ? (
                          <span className="quiet">{event.detail}</span>
                        ) : null}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </>
        )}
      </div>
    </>
  );
}
