import { useEffect, useState } from "react";
import {
  checkUpdates,
  getUpdaterStatus,
  installUpdate,
  listNetworkEvents,
  type NetworkEvent,
  rollbackUpdate,
  setUpdaterAutoCheck,
  type UpdateInfo,
  type UpdaterStatus,
  type VaultStatus,
} from "../lib/ipc";

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

const kindLabels: Record<string, string> = {
  update_check: "Comprobación",
  update_download: "Descarga",
  update_install: "Instalación",
  update_rollback: "Rollback",
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
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
}) {
  const ready = vault?.ready === true;
  const [status, setStatus] = useState<UpdaterStatus | null>(null);
  const [events, setEvents] = useState<NetworkEvent[]>([]);
  const [candidate, setCandidate] = useState<UpdateInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);

  async function load() {
    try {
      const [loadedStatus, loadedEvents] = await Promise.all([
        getUpdaterStatus(),
        listNetworkEvents(),
      ]);
      setStatus(loadedStatus);
      setEvents(loadedEvents);
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function loadInitial() {
      try {
        const [loadedStatus, loadedEvents] = await Promise.all([
          getUpdaterStatus(),
          listNetworkEvents(),
        ]);
        if (!cancelled) {
          setStatus(loadedStatus);
          setEvents(loadedEvents);
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
      setEvents(await listNetworkEvents());
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

  const busy = saving || checking || installing;

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Ajustes · local y sin telemetría</p>
        <h1>Ajustes</h1>
        <p className="hero-copy">
          Preferencias de la app y el updater aislado. La webview no hace
          peticiones de red: todo pasa por Rust, con URL fija y firma
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

      {!ready ? (
        <section className="next-card">
          <p className="eyebrow">Bóveda requerida</p>
          <h2>Primero elegí dónde guardar tus datos</h2>
          <p>
            Los ajustes y el registro de red se guardan en la bóveda local.
            Elegí una carpeta en la sección Bóveda.
          </p>
          <div className="hero-actions">
            <button type="button" className="button filled" onClick={onGoVault}>
              Ir a Bóveda
            </button>
          </div>
        </section>
      ) : (
        <>
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
              className="status-card settings-card"
              aria-labelledby="updater-title"
            >
              <div>
                <p className="eyebrow">Updater aislado</p>
                <h2 id="updater-title">Actualizaciones</h2>
                <p className="quiet">
                  Versión {status?.currentVersion ?? "desconocida"} · host fijo{" "}
                  {status?.updateHost ?? "github.com"}
                </p>
              </div>

              {!status?.publicKeyConfigured ? (
                <p className="notice">
                  Esta compilación no tiene una clave pública de producción. El
                  updater queda deshabilitado hasta configurarla; nunca se
                  descarga nada sin firma válida.
                </p>
              ) : null}

              <label className="setting-toggle">
                <input
                  type="checkbox"
                  checked={status?.autoCheck ?? false}
                  disabled={busy || !status}
                  onChange={(event) =>
                    void handleToggleAutoCheck(event.target.checked)
                  }
                />
                <span>
                  <strong>Buscar actualizaciones al iniciar</strong>
                  <span className="quiet">
                    Desactivado por defecto. Solo consulta el manifiesto fijo;
                    no envía datos de la bóveda.
                  </span>
                </span>
              </label>

              <div className="hero-actions">
                <button
                  type="button"
                  className="button filled"
                  disabled={
                    busy ||
                    !status?.publicKeyConfigured ||
                    !status.runningAsAppImage
                  }
                  onClick={() => void handleCheck()}
                >
                  {checking ? "Buscando…" : "Buscar actualizaciones"}
                </button>
                {candidate ? (
                  <button
                    type="button"
                    className="button tonal"
                    disabled={busy || !status?.runningAsAppImage}
                    onClick={() => void handleInstall()}
                  >
                    {installing
                      ? "Instalando…"
                      : `Instalar ${candidate.version}`}
                  </button>
                ) : null}
                {status?.canRollback ? (
                  <button
                    type="button"
                    className="button"
                    disabled={busy}
                    onClick={() => void handleRollback()}
                  >
                    Restaurar versión anterior
                  </button>
                ) : null}
              </div>

              {!status?.runningAsAppImage ? (
                <p className="quiet">
                  Las actualizaciones solo se instalan desde un AppImage
                  publicado. En desarrollo la comprobación queda deshabilitada.
                </p>
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
          </div>
        </>
      )}
    </>
  );
}
