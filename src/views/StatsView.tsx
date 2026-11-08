import { format, subDays } from "date-fns";
import { useEffect, useState } from "react";
import { getStudyStats, type StudyStats, type VaultStatus } from "../lib/ipc";

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function todayIso(): string {
  return format(new Date(), "yyyy-MM-dd");
}

function percentText(valueBp: number | null): string {
  return valueBp === null ? "sin datos" : `${(valueBp / 100).toFixed(0)} %`;
}

function minutesText(minutes: number): string {
  if (minutes < 60) {
    return `${minutes} min`;
  }
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${hours} h` : `${hours} h ${rest} min`;
}

type Period = "7" | "30" | "all";

export function StatsView({
  vault,
  onGoVault,
  onGoStrengths,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  onGoStrengths: () => void;
}) {
  const ready = vault?.ready === true;
  const [period, setPeriod] = useState<Period>("30");
  const [stats, setStats] = useState<StudyStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  const from =
    period === "all"
      ? null
      : format(subDays(new Date(), Number(period) - 1), "yyyy-MM-dd");

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function load() {
      try {
        const loaded = await getStudyStats(from, todayIso());
        if (!cancelled) {
          setStats(loaded);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, [ready, from]);

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Estadísticas · tiempo y constancia</p>
        <h1>Estadísticas</h1>
        <p className="hero-copy">
          Tiempo de estudio, rachas, materias débiles y próximos repasos,
          calculados localmente a partir de tus sesiones y autoevaluaciones.
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
            Las estadísticas se calculan sobre tu bóveda local. Elegí una
            carpeta en la sección Bóveda.
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

          <section className="academic-panel">
            <label className="field narrow">
              <span>Período</span>
              <select
                className="select-field"
                aria-label="Período de estadísticas"
                value={period}
                onChange={(event) => setPeriod(event.target.value as Period)}
              >
                <option value="7">Últimos 7 días</option>
                <option value="30">Últimos 30 días</option>
                <option value="all">Todo el historial</option>
              </select>
            </label>
            <div className="study-summary">
              <span className="status-pill">
                {stats?.totals.sessionCount ?? 0} sesiones
              </span>
              <span className="status-pill">
                {stats?.totals.currentStreakDays ?? 0} días de racha
              </span>
              <span className="status-pill">
                {stats?.dueReviews ?? 0} repasos vencidos
              </span>
            </div>
          </section>

          {!stats || stats.totals.sessionCount === 0 ? (
            <section className="next-card">
              <p className="eyebrow">Sin sesiones</p>
              <h2>Todavía no hay tiempo registrado</h2>
              <p>
                Arrancá un Pomodoro en la sección Estudio. Las estadísticas se
                actualizan solas al cerrar la sesión.
              </p>
            </section>
          ) : (
            <>
              <section className="study-cards" aria-label="Totales de estudio">
                <article className="card">
                  <span className="card-kicker">Tiempo</span>
                  <h2>{minutesText(stats.totals.totalMinutes)}</h2>
                  <p>{stats.totals.sessionCount} sesiones registradas.</p>
                </article>
                <article className="card">
                  <span className="card-kicker">Constancia</span>
                  <h2>{stats.totals.currentStreakDays} días</h2>
                  <p>
                    Racha actual. Máxima histórica:{" "}
                    {stats.totals.longestStreakDays} días.
                  </p>
                </article>
                <article className="card">
                  <span className="card-kicker">Repasos</span>
                  <h2>{stats.dueReviews}</h2>
                  <p>Repasos vencidos hoy o antes.</p>
                </article>
              </section>

              <section className="status-card">
                <div>
                  <p className="eyebrow">Tiempo por materia</p>
                  <h2>Dónde invertiste las horas</h2>
                </div>
                <ul className="bar-list" aria-label="Minutos por materia">
                  {stats.minutesBySubject.map((item) => {
                    const max = Math.max(
                      ...stats.minutesBySubject.map((entry) => entry.minutes),
                      1,
                    );
                    const width = Math.round((item.minutes / max) * 100);
                    return (
                      <li key={item.subjectId ?? "sin-materia"}>
                        <div className="bar-head">
                          <strong>{item.subjectName ?? "Sin materia"}</strong>
                          <span className="quiet">
                            {minutesText(item.minutes)} · {item.sessions}{" "}
                            sesión(es)
                          </span>
                        </div>
                        <div className="bar-track" aria-hidden="true">
                          <span style={{ width: `${width}%` }} />
                        </div>
                      </li>
                    );
                  })}
                </ul>
              </section>
            </>
          )}

          <div className="study-layout">
            <section className="status-card">
              <div>
                <p className="eyebrow">Materias débiles</p>
                <h2>Temas que necesitan atención</h2>
                <p className="quiet">
                  Se calculan con la última autoevaluación y el override manual.
                </p>
                <button
                  type="button"
                  className="button tonal"
                  onClick={onGoStrengths}
                >
                  Ir a Fortalezas
                </button>
              </div>
              {!stats || stats.weakSubjects.length === 0 ? (
                <p className="quiet">No hay materias con temas débiles.</p>
              ) : (
                <ul className="mini-list" aria-label="Materias débiles">
                  {stats.weakSubjects.map((item) => (
                    <li key={item.subjectId} className="mini-item">
                      <div>
                        <strong>{item.subjectName}</strong>
                        <span className="quiet">
                          {item.weakTopics} de {item.topicCount} temas débiles ·
                          promedio {percentText(item.averageScoreBp)}
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </section>

            <section className="status-card">
              <div>
                <p className="eyebrow">Próximos repasos</p>
                <h2>Agenda de repaso</h2>
                <p className="quiet">
                  Vencidos primero, luego los próximos 30 días.
                </p>
              </div>
              {!stats || stats.upcomingReviews.length === 0 ? (
                <p className="quiet">
                  No hay repasos programados en el horizonte.
                </p>
              ) : (
                <ul className="mini-list" aria-label="Próximos repasos">
                  {stats.upcomingReviews.map((review) => (
                    <li key={review.topicId} className="mini-item">
                      <div>
                        <strong>{review.topicName}</strong>
                        <span className="quiet">
                          {review.subjectName} · {review.dueDate} ·{" "}
                          {review.daysUntil < 0
                            ? `vencido hace ${Math.abs(review.daysUntil)} día(s)`
                            : review.daysUntil === 0
                              ? "hoy"
                              : `en ${review.daysUntil} día(s)`}
                        </span>
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
