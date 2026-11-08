import { useEffect, useState } from "react";
import {
  type AppInfo,
  getAppInfo,
  runningInTauri,
  runSchemaSelfCheck,
  type SchemaCheck,
} from "./lib/ipc";
import "./styles.css";

const navItems = [
  { id: "today", label: "Hoy", icon: "today" },
  { id: "schedule", label: "Horario", icon: "schedule" },
  { id: "calendar", label: "Calendario", icon: "calendar" },
  { id: "tasks", label: "Tareas", icon: "tasks" },
  { id: "exams", label: "Exámenes", icon: "exams" },
  { id: "grades", label: "Notas", icon: "grades" },
  { id: "strengths", label: "Fortalezas", icon: "strengths" },
  { id: "stats", label: "Estadísticas", icon: "stats" },
  { id: "vault", label: "Bóveda", icon: "vault" },
  { id: "settings", label: "Ajustes", icon: "settings" },
] as const;

const iconPaths: Record<string, string> = {
  today:
    "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Zm2 3v2h10V7H7Zm0 4v2h4v-2H7Zm6 0v2h4v-2h-4Zm-6 4v2h4v-2H7Z",
  schedule: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm1 4v5l3 2-1 1.7-4-2.7V7h2Z",
  calendar:
    "M7 3h2v2h6V3h2v2h2a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h2V3Zm12 7H5v9h14v-9ZM6 11h3v3H6v-3Z",
  tasks:
    "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1Zm3 4v2h2V8H8Zm4 0v2h5V8h-5Zm-4 4v2h2v-2H8Zm4 0v2h5v-2h-5Z",
  exams: "M6 3h9l4 4v14H6V3Zm8 2v3h3l-3-3ZM8 12h8v2H8v-2Zm0 4h8v2H8v-2Z",
  grades: "M4 20V10h4v10H4Zm6 0V4h4v16h-4Zm6 0v-7h4v7h-4Z",
  strengths:
    "M12 3l2.7 5.5 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1L3.2 9.4l6.1-.9L12 3Z",
  stats: "M4 19h16v2H4v-2Zm1-7h3v5H5v-5Zm5-5h3v10h-3V7Zm5 3h3v7h-3v-7Z",
  vault:
    "M12 2l8 4v6c0 5-3.4 8.6-8 10-4.6-1.4-8-5-8-10V6l8-4Zm0 3.2L7 7.5V12c0 3.4 2 6 5 7.2 3-1.2 5-3.8 5-7.2V7.5l-5-2.3Z",
  settings:
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm8.4 4a6.6 6.6 0 0 0-.1-1l2-1.5-2-3.4-2.3.9a7.8 7.8 0 0 0-1.7-1L16 3.5h-4l-.4 2.4a7.8 7.8 0 0 0-1.7 1l-2.3-.9-2 3.4 2 1.5a6.6 6.6 0 0 0 0 2l-2 1.5 2 3.4 2.3-.9c.5.4 1.1.8 1.7 1l.4 2.4h4l.4-2.4c.6-.2 1.2-.6 1.7-1l2.3.9 2-3.4-2-1.5c.1-.3.1-.7.1-1Z",
};

function Icon({ name }: { name: string }) {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d={iconPaths[name] ?? iconPaths.today} />
    </svg>
  );
}

function App() {
  const [active, setActive] = useState("today");
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [schema, setSchema] = useState<SchemaCheck | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!runningInTauri) {
      return;
    }

    let cancelled = false;
    Promise.all([getAppInfo(), runSchemaSelfCheck()])
      .then(([appInfo, check]) => {
        if (!cancelled) {
          setInfo(appInfo);
          setSchema(check);
        }
      })
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const activeLabel =
    navItems.find((item) => item.id === active)?.label ?? "Hoy";

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <img src="/omega.svg" alt="" className="brand-mark" />
          <span className="brand-name">Omega</span>
          <span className="phase-chip">Fase 0</span>
        </div>
        <div className="topbar-meta">
          <span className="quiet">Local · sin cuentas · sin telemetría</span>
          <span className="version">
            {info ? `v${info.version}` : "v0.1.0"}
          </span>
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
          <section className="hero">
            <p className="eyebrow">Base del proyecto · {activeLabel}</p>
            <h1>Tu año académico, en una bóveda local.</h1>
            <p className="hero-copy">
              Horarios, exámenes, tareas, notas y estudio. Sin nube, sin cuentas
              y sin que tus datos salgan del equipo. Esta es la Fase 0: el
              contrato técnico sobre el que se construye el resto.
            </p>
            <div className="hero-actions">
              <button type="button" className="button filled">
                Ver checklist
              </button>
              <button type="button" className="button tonal">
                Abrir bóveda
              </button>
            </div>
          </section>

          <section className="cards" aria-label="Principios del proyecto">
            <article className="card">
              <span className="card-kicker">Datos</span>
              <h2>Bóveda local</h2>
              <p>
                SQLite en <code>/omega</code>, adjuntos y copias verificadas en{" "}
                <code>/loglife</code>.
              </p>
            </article>
            <article className="card">
              <span className="card-kicker">Portabilidad</span>
              <h2>Exportación total</h2>
              <p>
                Un único <code>.txt</code> canónico, con checksum y round-trip
                byte a byte.
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
                Modo navegador: el IPC de Tauri no está disponible. Abrí la app
                con <code>npm run tauri dev</code> para ejecutar el
                autodiagnóstico.
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
                    {schema
                      ? `${schema.foreignKeyViolations} violaciones`
                      : "…"}
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
            <h2 id="next-title">Fase 0 en curso</h2>
            <ul className="checklist">
              <li className="done">Scaffold Tauri 2 + React + TypeScript</li>
              <li className="done">
                Esquema SQLite v1 y migración con checksum
              </li>
              <li className="done">Permisos mínimos y CSP base</li>
              <li className="done">
                Checklist persistente en docs/CHECKLIST.md
              </li>
              <li>Contrato de exportación v1</li>
              <li>Revisión de licencias y documento de arquitectura</li>
            </ul>
          </section>
        </main>
      </div>

      <footer className="footer">
        <span>Omega · Fase 0</span>
        <span>{runningInTauri ? "IPC conectado" : "Render en navegador"}</span>
      </footer>
    </div>
  );
}

export default App;
