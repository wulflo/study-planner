import {
  addMonths,
  eachDayOfInterval,
  endOfMonth,
  endOfWeek,
  format,
  isSameMonth,
  startOfMonth,
  startOfWeek,
} from "date-fns";
import { es } from "date-fns/locale";
import { useEffect, useMemo, useState } from "react";
import {
  type Exam,
  listAcademicYears,
  listExams,
  listSubjects,
  listTerms,
  type Subject,
  searchTasks,
  type Task,
  type Term,
  type VaultStatus,
} from "../lib/ipc";

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function isoDay(date: Date): string {
  return format(date, "yyyy-MM-dd");
}

export function CalendarView({
  vault,
  onGoVault,
  onGoSubjects,
  onBack,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  onGoSubjects: () => void;
  onBack: () => void;
}) {
  const ready = vault?.ready === true;
  const [month, setMonth] = useState(() => startOfMonth(new Date()));
  const [terms, setTerms] = useState<Term[]>([]);
  const [termId, setTermId] = useState("");
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [exams, setExams] = useState<Exam[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function loadStructure() {
      try {
        const years = await listAcademicYears();
        const loadedTerms = years[0] ? await listTerms(years[0].id) : [];
        if (!cancelled) {
          setTerms(loadedTerms);
          setTermId((current) => current || (loadedTerms[0]?.id ?? ""));
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
    if (!ready || !termId) {
      return;
    }
    let cancelled = false;

    async function loadExams() {
      try {
        const [loadedExams, loadedSubjects] = await Promise.all([
          listExams(termId, null),
          listSubjects(termId),
        ]);
        if (!cancelled) {
          setExams(loadedExams);
          setSubjects(loadedSubjects);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadExams();
    return () => {
      cancelled = true;
    };
  }, [ready, termId]);

  const gridStart = startOfWeek(startOfMonth(month), { weekStartsOn: 1 });
  const gridEnd = endOfWeek(endOfMonth(month), { weekStartsOn: 1 });
  const days = useMemo(
    () => eachDayOfInterval({ start: gridStart, end: gridEnd }),
    [gridStart, gridEnd],
  );
  const rangeStart = isoDay(gridStart);
  const rangeEnd = isoDay(gridEnd);

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function loadTasks() {
      try {
        const loaded = await searchTasks({
          dueFrom: rangeStart,
          dueTo: rangeEnd,
        });
        if (!cancelled) {
          setTasks(loaded);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadTasks();
    return () => {
      cancelled = true;
    };
  }, [ready, rangeStart, rangeEnd]);

  const subjectNames = useMemo(() => {
    const map = new Map<string, string>();
    for (const subject of subjects) {
      map.set(subject.id, subject.name);
    }
    return map;
  }, [subjects]);

  const eventsByDay = useMemo(() => {
    const map = new Map<
      string,
      { id: string; label: string; kind: string }[]
    >();
    for (const exam of exams) {
      const day = exam.date;
      const list = map.get(day) ?? [];
      list.push({
        id: `exam-${exam.id}`,
        label: `Examen · ${exam.title}${
          subjectNames.has(exam.subjectId)
            ? ` (${subjectNames.get(exam.subjectId)})`
            : ""
        }`,
        kind: "exam",
      });
      map.set(day, list);
    }
    for (const task of tasks) {
      if (!task.dueDate) {
        continue;
      }
      const list = map.get(task.dueDate) ?? [];
      list.push({
        id: `task-${task.id}`,
        label: `Tarea · ${task.title}`,
        kind: "task",
      });
      map.set(task.dueDate, list);
    }
    return map;
  }, [exams, tasks, subjectNames]);

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Calendario · exámenes y vencimientos</p>
        <h1>Calendario</h1>
        <p className="hero-copy">
          Una vista mensual local con exámenes y tareas por fecha. Sin
          sincronización externa.
        </p>
        <div className="hero-actions">
          <button type="button" className="button tonal" onClick={onBack}>
            Volver a Hoy
          </button>
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
            El calendario lee exámenes y tareas de tu bóveda local. Elegí una
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

          {terms.length === 0 ? (
            <section className="next-card">
              <p className="eyebrow">Materias requeridas</p>
              <h2>Creá una materia primero</h2>
              <p>
                Para ubicar exámenes en el calendario necesitás al menos un
                término con materias.
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
          ) : (
            <>
              <section className="academic-panel">
                <div className="week-tools">
                  <label className="field">
                    <span>Término</span>
                    <select
                      className="select-field"
                      value={termId}
                      onChange={(event) => setTermId(event.target.value)}
                    >
                      {terms.map((term) => (
                        <option key={term.id} value={term.id}>
                          {term.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="week-nav">
                    <button
                      type="button"
                      className="button tonal"
                      onClick={() => setMonth(addMonths(month, -1))}
                    >
                      ← Anterior
                    </button>
                    <h2>{format(month, "MMMM yyyy", { locale: es })}</h2>
                    <button
                      type="button"
                      className="button tonal"
                      onClick={() => setMonth(addMonths(month, 1))}
                    >
                      Siguiente →
                    </button>
                  </div>
                </div>
              </section>

              <section
                className="calendar-grid"
                aria-label="Calendario mensual"
              >
                {["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"].map(
                  (label) => (
                    <div key={label} className="calendar-weekday">
                      {label}
                    </div>
                  ),
                )}
                {days.map((day) => {
                  const key = isoDay(day);
                  const events = eventsByDay.get(key) ?? [];
                  return (
                    <div
                      key={key}
                      className={`calendar-day${
                        isSameMonth(day, month) ? "" : " is-outside"
                      }${key === isoDay(new Date()) ? " is-today" : ""}`}
                    >
                      <span className="calendar-day-number">
                        {format(day, "d")}
                      </span>
                      {events.map((event) => (
                        <span
                          key={event.id}
                          className={`calendar-event is-${event.kind}`}
                        >
                          {event.label}
                        </span>
                      ))}
                    </div>
                  );
                })}
              </section>
            </>
          )}
        </>
      )}
    </>
  );
}
