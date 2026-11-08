import { format, parseISO, startOfWeek } from "date-fns";
import { es } from "date-fns/locale";
import { useEffect, useState } from "react";
import { AttendanceButtons } from "../components/AttendanceButtons";
import {
  type AttendanceStatus,
  clearAttendance,
  getNextClass,
  getWeekSchedule,
  listTerms,
  type ScheduleOccurrence,
  setAttendance,
  type VaultStatus,
  type WeekSchedule,
} from "../lib/ipc";

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function isoDate(date: Date): string {
  return format(date, "yyyy-MM-dd");
}

function mondayOf(value: string): string {
  return isoDate(startOfWeek(parseISO(value), { weekStartsOn: 1 }));
}

function formatPercent(value: number | null): string {
  return value === null ? "—" : `${(value / 100).toFixed(1)}%`;
}

export function TodaySchedule({
  vault,
  onGoSubjects,
}: {
  vault: VaultStatus | null;
  onGoSubjects: () => void;
}) {
  const ready = vault?.ready === true;
  const [today] = useState(() => isoDate(new Date()));
  const [now] = useState(() => format(new Date(), "HH:mm"));
  const [termId, setTermId] = useState<string | null>(null);
  const [nextClass, setNextClass] = useState<ScheduleOccurrence | null>(null);
  const [week, setWeek] = useState<WeekSchedule | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!ready) {
      return;
    }

    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const terms = await listTerms(null);
        if (cancelled) {
          return;
        }
        const nextTerm = terms[0]?.id ?? null;
        setTermId(nextTerm);
        if (!nextTerm) {
          setNextClass(null);
          setWeek(null);
          return;
        }

        const [loadedNext, loadedWeek] = await Promise.all([
          getNextClass(nextTerm, today, now),
          getWeekSchedule(nextTerm, mondayOf(today)),
        ]);
        if (!cancelled) {
          setNextClass(loadedNext);
          setWeek(loadedWeek);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load();

    return () => {
      cancelled = true;
    };
  }, [ready, today, now]);

  async function markAttendance(
    occurrence: ScheduleOccurrence,
    status: AttendanceStatus | null,
  ) {
    if (!termId) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      if (status === null) {
        await clearAttendance(occurrence.slotId, occurrence.date);
      } else {
        await setAttendance(occurrence.slotId, occurrence.date, status, null);
      }
      setWeek(await getWeekSchedule(termId, mondayOf(today)));
      setNotice("Asistencia actualizada.");
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  if (!ready) {
    return null;
  }

  const todayOccurrences =
    week?.occurrences.filter((occurrence) => occurrence.date === today) ?? [];
  const todaySubjects = new Set(
    todayOccurrences.map((occurrence) => occurrence.subjectId),
  );
  const todayAttendance =
    week?.attendance.filter((summary) =>
      todaySubjects.has(summary.subjectId),
    ) ?? [];

  return (
    <>
      {error ? (
        <p className="notice error" role="alert">
          {error}
        </p>
      ) : notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      <section className="status-card" aria-labelledby="next-class-title">
        <div>
          <p className="eyebrow">Horario · {today}</p>
          <h2 id="next-class-title">Próxima clase</h2>
          {loading && !week ? (
            <p className="quiet">Cargando horario…</p>
          ) : !termId ? (
            <p>
              Todavía no hay materias en un término académico. Creá el año, el
              término y tus materias para ver el horario acá.
            </p>
          ) : nextClass ? (
            <p>
              <strong>{nextClass.subjectName}</strong>
              <br />
              {format(parseISO(nextClass.date), "EEEE d MMM", { locale: es })} ·{" "}
              {nextClass.startTime}–{nextClass.endTime}
              {nextClass.room ? ` · ${nextClass.room}` : ""}
            </p>
          ) : (
            <p className="quiet">No hay próximas clases en este término.</p>
          )}
          {!termId ? (
            <button
              type="button"
              className="button tonal"
              onClick={onGoSubjects}
            >
              Ir a Materias
            </button>
          ) : null}
        </div>
        {week?.rotationLabel ? (
          <span className="status-pill is-ok">
            Rotación {week.rotationLabel}
          </span>
        ) : null}
      </section>

      {termId ? (
        <section className="status-card" aria-labelledby="today-classes-title">
          <div className="task-board-head">
            <div>
              <p className="eyebrow">Hoy</p>
              <h2 id="today-classes-title">Clases de hoy</h2>
            </div>
            <span className="count-chip">{todayOccurrences.length}</span>
          </div>
          {todayOccurrences.length === 0 ? (
            <p className="empty-state">No hay clases hoy.</p>
          ) : (
            <ul className="occurrence-list">
              {todayOccurrences.map((occurrence) => (
                <li
                  className={`occurrence kind-${occurrence.kind}`}
                  key={`${occurrence.slotId}-${occurrence.date}-${occurrence.kind}`}
                >
                  <div className="occurrence-head">
                    <span
                      className="subject-dot"
                      style={{ background: occurrence.subjectColor }}
                      aria-hidden="true"
                    />
                    <strong>{occurrence.subjectName}</strong>
                    <span className="quiet">
                      {occurrence.startTime}–{occurrence.endTime}
                    </span>
                  </div>
                  <div className="occurrence-meta">
                    {occurrence.room ? (
                      <span className="quiet">{occurrence.room}</span>
                    ) : null}
                    {occurrence.kind !== "scheduled" ? (
                      <span className={`kind-chip ${occurrence.kind}`}>
                        {occurrence.kind === "cancelled"
                          ? "Cancelada"
                          : occurrence.kind === "moved"
                            ? "Movida"
                            : "Extra"}
                      </span>
                    ) : null}
                  </div>
                  {occurrence.kind !== "cancelled" ? (
                    <AttendanceButtons
                      occurrence={occurrence}
                      busy={busy}
                      onChange={(status) =>
                        void markAttendance(occurrence, status)
                      }
                    />
                  ) : null}
                </li>
              ))}
            </ul>
          )}

          {todayAttendance.length > 0 ? (
            <ul className="mini-list" aria-label="Asistencia de hoy">
              {todayAttendance.map((summary) => (
                <li className="attendance-summary" key={summary.subjectId}>
                  <span
                    className="subject-dot"
                    style={{ background: summary.subjectColor }}
                    aria-hidden="true"
                  />
                  <span className="attendance-name">{summary.subjectName}</span>
                  <span
                    className={`attendance-value${summary.belowMin ? " is-low" : ""}`}
                  >
                    {formatPercent(summary.percentBp)}
                    {summary.minBp !== null
                      ? ` / mín. ${formatPercent(summary.minBp)}`
                      : ""}
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      ) : null}
    </>
  );
}
