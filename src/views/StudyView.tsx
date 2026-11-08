import { format } from "date-fns";
import { useEffect, useMemo, useState } from "react";
import {
  deleteStudySession,
  finishStudySession,
  listAcademicYears,
  listStudyPriorities,
  listStudySessions,
  listSubjects,
  listTerms,
  listTopics,
  type StudyPriorityView,
  type StudySession,
  type StudySessionMode,
  type Subject,
  startStudySession,
  type Topic,
  type VaultStatus,
} from "../lib/ipc";

const modeLabels: Record<StudySessionMode, string> = {
  pomodoro: "Pomodoro",
  free: "Libre",
  review: "Repaso",
};

const classificationLabels = {
  weak: "Débil",
  neutral: "Neutral",
  strength: "Fortaleza",
} as const;

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function todayIso(): string {
  return format(new Date(), "yyyy-MM-dd");
}

function formatClock(totalSeconds: number): string {
  const safe = Math.max(0, totalSeconds);
  const minutes = Math.floor(safe / 60);
  const seconds = safe % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function formatDateTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value.slice(0, 16).replace("T", " ")
    : new Intl.DateTimeFormat("es", {
        day: "numeric",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date);
}

export function StudyView({
  vault,
  onGoVault,
  onGoSubjects,
  onGoStrengths,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  onGoSubjects: () => void;
  onGoStrengths: () => void;
}) {
  const ready = vault?.ready === true;
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [priorities, setPriorities] = useState<StudyPriorityView[]>([]);
  const [sessions, setSessions] = useState<StudySession[]>([]);
  const [subjectId, setSubjectId] = useState("");
  const [topics, setTopics] = useState<Topic[]>([]);
  const [topicId, setTopicId] = useState("");
  const [mode, setMode] = useState<StudySessionMode>("pomodoro");
  const [planned, setPlanned] = useState("25");
  const [activeSession, setActiveSession] = useState<StudySession | null>(null);
  const [plannedMinutes, setPlannedMinutes] = useState(25);
  const [remainingSeconds, setRemainingSeconds] = useState(0);
  const [running, setRunning] = useState(false);
  const [interruptions, setInterruptions] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function loadStructure() {
      try {
        const years = await listAcademicYears();
        const terms = years[0] ? await listTerms(years[0].id) : [];
        const loadedSubjects = terms[0] ? await listSubjects(terms[0].id) : [];
        if (!cancelled) {
          setSubjects(loadedSubjects);
          setSubjectId((current) => current || (loadedSubjects[0]?.id ?? ""));
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadStructure();
    return () => {
      cancelled = true;
    };
  }, [ready]);

  useEffect(() => {
    if (!ready || !subjectId) {
      setTopics([]);
      return;
    }
    let cancelled = false;

    async function loadTopics() {
      try {
        const loaded = await listTopics(subjectId);
        if (!cancelled) {
          setTopics(loaded);
          setTopicId((current) =>
            current && loaded.some((item) => item.id === current)
              ? current
              : "",
          );
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadTopics();
    return () => {
      cancelled = true;
    };
  }, [ready, subjectId]);

  async function reload() {
    const [loadedPriorities, loadedSessions] = await Promise.all([
      listStudyPriorities(todayIso()),
      listStudySessions(),
    ]);
    setPriorities(loadedPriorities);
    setSessions(loadedSessions);
  }

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function load() {
      try {
        const [loadedPriorities, loadedSessions] = await Promise.all([
          listStudyPriorities(todayIso()),
          listStudySessions(),
        ]);
        if (!cancelled) {
          setPriorities(loadedPriorities);
          setSessions(loadedSessions);
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
  }, [ready]);

  useEffect(() => {
    if (!activeSession || !running) {
      return;
    }
    const timer = window.setInterval(() => {
      setRemainingSeconds((value) => Math.max(0, value - 1));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [activeSession, running]);

  useEffect(() => {
    if (activeSession && running && remainingSeconds === 0) {
      setRunning(false);
      setNotice(
        "Tiempo cumplido. Podés cerrar la sesión o seguir en modo libre.",
      );
    }
  }, [activeSession, running, remainingSeconds]);

  const subjectNames = useMemo(() => {
    const map = new Map<string, string>();
    for (const subject of subjects) {
      map.set(subject.id, subject.name);
    }
    return map;
  }, [subjects]);

  const elapsedSeconds = activeSession
    ? Math.max(0, plannedMinutes * 60 - remainingSeconds)
    : 0;
  const progress = activeSession
    ? Math.min(100, Math.round((elapsedSeconds / (plannedMinutes * 60)) * 100))
    : 0;

  async function reloadSafe() {
    try {
      await reload();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleStart() {
    const minutes = Number(planned);
    if (
      !Number.isFinite(minutes) ||
      !Number.isInteger(minutes) ||
      minutes < 1 ||
      minutes > 240
    ) {
      setError("Los minutos planificados deben ser un entero entre 1 y 240.");
      return;
    }

    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const session = await startStudySession(
        subjectId || null,
        topicId || null,
        mode,
        minutes,
      );
      setActiveSession(session);
      setPlannedMinutes(minutes);
      setRemainingSeconds(minutes * 60);
      setInterruptions(0);
      setRunning(true);
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  function toggleRunning() {
    if (!activeSession) {
      return;
    }
    if (running) {
      setRunning(false);
      setInterruptions((value) => value + 1);
    } else {
      setRunning(true);
    }
  }

  async function handleFinish() {
    if (!activeSession) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const actual = Math.max(0, Math.round(elapsedSeconds / 60));
      await finishStudySession(activeSession.id, actual, interruptions, null);
      setActiveSession(null);
      setRunning(false);
      setRemainingSeconds(0);
      setInterruptions(0);
      setNotice(`Sesión cerrada con ${actual} minuto(s).`);
      await reloadSafe();
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  async function handleDiscard() {
    if (!activeSession) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await deleteStudySession(activeSession.id);
      setActiveSession(null);
      setRunning(false);
      setRemainingSeconds(0);
      setInterruptions(0);
      setNotice("Sesión descartada.");
      await reloadSafe();
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  async function handleDeleteSession(id: string) {
    setBusy(true);
    setError(null);
    try {
      await deleteStudySession(id);
      await reloadSafe();
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Estudio · Pomodoro y prioridad</p>
        <h1>Estudio</h1>
        <p className="hero-copy">
          La cola de prioridad explica por qué un tema va primero. Arrancá un
          Pomodoro, registrá las pausas y guardá la sesión en tu bóveda.
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
            Las sesiones de estudio se guardan en tu bóveda local. Elegí una
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
          {notice ? <p className="notice">{notice}</p> : null}

          {subjects.length === 0 ? (
            <section className="next-card">
              <p className="eyebrow">Materias requeridas</p>
              <h2>Creá una materia primero</h2>
              <p>
                Podés estudiar sin materia, pero la cola de prioridad necesita
                temas. Creá la estructura académica y volvé.
              </p>
              <div className="hero-actions">
                <button
                  type="button"
                  className="button filled"
                  onClick={onGoSubjects}
                >
                  Ir a Materias
                </button>
              </div>
            </section>
          ) : null}

          <div className="study-layout">
            <section className="status-card">
              <div>
                <p className="eyebrow">Pomodoro</p>
                <h2>
                  {activeSession
                    ? formatClock(remainingSeconds)
                    : `${plannedMinutes} min planificados`}
                </h2>
                <p className="quiet">
                  {activeSession
                    ? `${modeLabels[activeSession.mode]} · ${interruptions} interrupción(es)`
                    : "El temporizador se apoya en la hora local del equipo."}
                </p>
              </div>

              {activeSession ? (
                <div className="pomodoro">
                  <div className="pomodoro-progress" aria-hidden="true">
                    <span style={{ width: `${progress}%` }} />
                  </div>
                  <div className="hero-actions">
                    <button
                      type="button"
                      className="button filled"
                      disabled={busy}
                      onClick={toggleRunning}
                    >
                      {running ? "Pausar" : "Reanudar"}
                    </button>
                    <button
                      type="button"
                      className="button tonal"
                      disabled={busy}
                      onClick={() => void handleFinish()}
                    >
                      Terminar sesión
                    </button>
                    <button
                      type="button"
                      className="mini-action danger"
                      disabled={busy}
                      onClick={() => void handleDiscard()}
                    >
                      Descartar
                    </button>
                  </div>
                </div>
              ) : (
                <div className="inline-form">
                  <label className="field narrow">
                    <span>Modo</span>
                    <select
                      className="select-field"
                      aria-label="Modo de estudio"
                      value={mode}
                      onChange={(event) =>
                        setMode(event.target.value as StudySessionMode)
                      }
                    >
                      <option value="pomodoro">Pomodoro</option>
                      <option value="free">Libre</option>
                      <option value="review">Repaso</option>
                    </select>
                  </label>
                  <label className="field narrow">
                    <span>Minutos</span>
                    <input
                      className="text-field"
                      type="number"
                      min={1}
                      max={240}
                      aria-label="Minutos planificados"
                      value={planned}
                      onChange={(event) => setPlanned(event.target.value)}
                    />
                  </label>
                  <label className="field">
                    <span>Materia</span>
                    <select
                      className="select-field"
                      aria-label="Materia de la sesión"
                      value={subjectId}
                      onChange={(event) => setSubjectId(event.target.value)}
                    >
                      <option value="">Sin materia</option>
                      {subjects.map((subject) => (
                        <option key={subject.id} value={subject.id}>
                          {subject.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="field">
                    <span>Tema (opcional)</span>
                    <select
                      className="select-field"
                      aria-label="Tema de la sesión"
                      value={topicId}
                      onChange={(event) => setTopicId(event.target.value)}
                    >
                      <option value="">Sin tema</option>
                      {topics.map((topic) => (
                        <option key={topic.id} value={topic.id}>
                          {topic.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    type="button"
                    className="button filled"
                    disabled={busy}
                    onClick={() => void handleStart()}
                  >
                    Empezar
                  </button>
                </div>
              )}
            </section>

            <section className="status-card">
              <div>
                <p className="eyebrow">Prioridad explicable</p>
                <h2>Qué estudiar primero</h2>
                <p className="quiet">
                  Puntaje determinista: fortaleza, repaso y examen próximo. Cada
                  fila muestra sus motivos.
                </p>
                <button
                  type="button"
                  className="button tonal"
                  onClick={onGoStrengths}
                >
                  Ajustar fortalezas
                </button>
              </div>
              {priorities.length === 0 ? (
                <p className="quiet">
                  Todavía no hay temas evaluados. Creá temas y autoevalualos en
                  Fortalezas.
                </p>
              ) : (
                <ol className="priority-list" aria-label="Cola de prioridad">
                  {priorities.slice(0, 10).map((item, index) => (
                    <li key={item.topicId} className="priority-row">
                      <span className="priority-rank">{index + 1}</span>
                      <div>
                        <strong>{item.topicName}</strong>{" "}
                        <span className="quiet">{item.subjectName}</span>{" "}
                        <span
                          className={`strength-chip is-${item.classification}`}
                        >
                          {classificationLabels[item.classification]}
                        </span>
                        <ul className="priority-reasons">
                          {item.reasons.map((reason) => (
                            <li key={reason.code}>
                              {reason.label}{" "}
                              <span className="quiet">+{reason.points}</span>
                            </li>
                          ))}
                        </ul>
                      </div>
                      <span className="priority-score">{item.score}</span>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          </div>

          <section className="status-card">
            <div>
              <p className="eyebrow">Historial</p>
              <h2>Sesiones recientes</h2>
              <p className="quiet">
                Se guardan con minutos reales, modo e interrupciones.
              </p>
            </div>
            {sessions.length === 0 ? (
              <p className="quiet">Todavía no registraste sesiones.</p>
            ) : (
              <ul className="mini-list" aria-label="Sesiones de estudio">
                {sessions.slice(0, 12).map((session) => (
                  <li key={session.id} className="mini-item">
                    <div>
                      <strong>
                        {session.subjectId
                          ? (subjectNames.get(session.subjectId) ??
                            "Materia eliminada")
                          : "Sin materia"}
                      </strong>
                      <span className="quiet">
                        {modeLabels[session.mode]} ·{" "}
                        {session.actualMinutes ?? 0} min ·{" "}
                        {session.interruptions} interrupción(es) ·{" "}
                        {formatDateTime(session.startedAt)}
                      </span>
                    </div>
                    <button
                      type="button"
                      className="mini-action danger"
                      aria-label={`Eliminar sesión de ${formatDateTime(session.startedAt)}`}
                      disabled={busy}
                      onClick={() => void handleDeleteSession(session.id)}
                    >
                      ×
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </>
      )}
    </>
  );
}
