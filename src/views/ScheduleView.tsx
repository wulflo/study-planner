import { addDays, format, parseISO, startOfWeek } from "date-fns";
import { es } from "date-fns/locale";
import {
  type FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { AttendanceButtons } from "../components/AttendanceButtons";
import {
  type AttendanceStatus,
  type ClassException,
  type ClassSlot,
  type ClassSlotInput,
  clearAttendance,
  createClassException,
  createClassSlot,
  createHoliday,
  deleteClassException,
  deleteClassSlot,
  deleteHoliday,
  type ExceptionKind,
  getRotation,
  getWeekSchedule,
  type Holiday,
  listClassExceptions,
  listClassSlots,
  listHolidays,
  listSubjects,
  listTerms,
  type Recurrence,
  type RotationConfig,
  type RotationMode,
  type ScheduleOccurrence,
  type Subject,
  saveRotation,
  setAttendance,
  type Term,
  updateClassSlot,
  type VaultStatus,
  type WeekSchedule,
} from "../lib/ipc";

const weekdayLabels = [
  "Lunes",
  "Martes",
  "Miércoles",
  "Jueves",
  "Viernes",
  "Sábado",
  "Domingo",
];
const weekdayShort = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];
const kindLabels: Record<ExceptionKind, string> = {
  cancelled: "Cancelada",
  moved: "Movida",
  extra: "Extra",
};
const occurrenceLabels: Record<ScheduleOccurrence["kind"], string> = {
  scheduled: "Clase",
  cancelled: "Cancelada",
  moved: "Movida",
  extra: "Extra",
};
const rotationModeLabels: Record<RotationMode, string> = {
  weekly_ab: "Semanal A/B",
  parity: "Paridad de semanas",
  day_cycle: "Ciclo de días",
};

const emptySlotForm = {
  id: null as string | null,
  subjectId: "",
  weekday: 1,
  startTime: "08:00",
  endTime: "10:00",
  recurrence: "weekly" as Recurrence,
  rotationKey: "",
  validFrom: "",
  validTo: "",
  roomOverride: "",
};

const emptyHolidayForm = { date: "", name: "" };

const emptyExceptionForm = {
  slotId: "",
  date: "",
  kind: "cancelled" as ExceptionKind,
  newStartTime: "",
  newEndTime: "",
  newRoom: "",
};

const emptyRotationForm = {
  mode: "weekly_ab" as RotationMode,
  anchorDate: "",
  cycleLength: 2,
  labels: "A, B",
  dayCycle: "1, 2, 3",
};

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function isoDate(date: Date): string {
  return format(date, "yyyy-MM-dd");
}

function mondayOf(value: string): string {
  return isoDate(startOfWeek(parseISO(value), { weekStartsOn: 1 }));
}

function todayIso(): string {
  return isoDate(new Date());
}

function optionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length === 0 ? null : trimmed;
}

function splitLabels(value: string): string[] {
  return value
    .split(",")
    .map((label) => label.trim())
    .filter((label) => label.length > 0);
}

function formatPercent(value: number | null): string {
  return value === null ? "—" : `${(value / 100).toFixed(1)}%`;
}

function slotInput(form: typeof emptySlotForm): ClassSlotInput {
  return {
    weekday: form.weekday,
    startTime: form.startTime,
    endTime: form.endTime,
    recurrence: form.recurrence,
    rotationKey: optionalText(form.rotationKey),
    validFrom: optionalText(form.validFrom),
    validTo: optionalText(form.validTo),
    roomOverride: optionalText(form.roomOverride),
  };
}

function rotationFormFrom(
  config: RotationConfig | null,
): typeof emptyRotationForm {
  if (!config) {
    return emptyRotationForm;
  }
  return {
    mode: config.mode,
    anchorDate: config.anchorDate,
    cycleLength: config.cycleLength,
    labels: config.labels.join(", "),
    dayCycle: config.dayCycle.join(", "),
  };
}

export function ScheduleView({
  vault,
  onGoVault,
  onGoSubjects,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  onGoSubjects: () => void;
}) {
  const ready = vault?.ready === true;
  const [terms, setTerms] = useState<Term[]>([]);
  const [termId, setTermId] = useState<string | null>(null);
  const [weekStart, setWeekStart] = useState(() => mondayOf(todayIso()));
  const [week, setWeek] = useState<WeekSchedule | null>(null);
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [slots, setSlots] = useState<ClassSlot[]>([]);
  const [holidays, setHolidays] = useState<Holiday[]>([]);
  const [exceptions, setExceptions] = useState<ClassException[]>([]);
  const [rotation, setRotation] = useState<RotationConfig | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [slotForm, setSlotForm] = useState(emptySlotForm);
  const [holidayForm, setHolidayForm] = useState(emptyHolidayForm);
  const [exceptionForm, setExceptionForm] = useState(emptyExceptionForm);
  const [rotationForm, setRotationForm] = useState(emptyRotationForm);

  const loadTerm = useCallback(async (nextTerm: string, nextWeek: string) => {
    const [
      loadedWeek,
      loadedSlots,
      loadedHolidays,
      loadedExceptions,
      loadedRotation,
      loadedSubjects,
    ] = await Promise.all([
      getWeekSchedule(nextTerm, nextWeek),
      listClassSlots(nextTerm),
      listHolidays(nextTerm),
      listClassExceptions(nextTerm),
      getRotation(nextTerm),
      listSubjects(nextTerm),
    ]);

    setWeek(loadedWeek);
    setSlots(loadedSlots);
    setHolidays(loadedHolidays);
    setExceptions(loadedExceptions);
    setRotation(loadedRotation);
    setRotationForm(rotationFormFrom(loadedRotation));
    setSubjects(loadedSubjects);
    if (loadedSubjects.length > 0) {
      setSlotForm((current) =>
        current.id === null && current.subjectId === ""
          ? { ...current, subjectId: loadedSubjects[0].id }
          : current,
      );
    }
    if (loadedSlots.length > 0) {
      setExceptionForm((current) =>
        current.slotId === ""
          ? { ...current, slotId: loadedSlots[0].id }
          : current,
      );
    }
  }, []);

  useEffect(() => {
    if (!ready) {
      return;
    }

    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const loadedTerms = await listTerms(null);
        if (cancelled) {
          return;
        }
        setTerms(loadedTerms);
        const nextTerm = loadedTerms[0]?.id ?? null;
        setTermId(nextTerm);
        if (nextTerm) {
          await loadTerm(nextTerm, mondayOf(todayIso()));
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
  }, [ready, loadTerm]);

  const weekDays = useMemo(() => {
    const anchor = week?.weekStart ?? weekStart;
    return Array.from({ length: 7 }, (_, index) =>
      isoDate(addDays(parseISO(anchor), index)),
    );
  }, [week?.weekStart, weekStart]);

  async function run(operation: () => Promise<void>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await operation();
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  async function selectTerm(nextTerm: string) {
    setTermId(nextTerm);
    setError(null);
    setWeek(null);
    setSlotForm(emptySlotForm);
    setExceptionForm(emptyExceptionForm);
    try {
      setLoading(true);
      await loadTerm(nextTerm, weekStart);
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setLoading(false);
    }
  }

  async function moveWeek(delta: number) {
    if (!termId) {
      return;
    }
    const nextWeek = isoDate(addDays(parseISO(weekStart), delta * 7));
    setWeekStart(nextWeek);
    setError(null);
    try {
      setWeek(await getWeekSchedule(termId, nextWeek));
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function goCurrentWeek() {
    if (!termId) {
      return;
    }
    const nextWeek = mondayOf(todayIso());
    setWeekStart(nextWeek);
    setError(null);
    try {
      setWeek(await getWeekSchedule(termId, nextWeek));
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function refreshAll(preferredTerm: string | null = termId) {
    const loadedTerms = await listTerms(null);
    setTerms(loadedTerms);
    const nextTerm =
      loadedTerms.find((term) => term.id === preferredTerm)?.id ??
      loadedTerms[0]?.id ??
      null;
    setTermId(nextTerm);
    if (nextTerm) {
      await loadTerm(nextTerm, weekStart);
    } else {
      setWeek(null);
      setSlots([]);
      setHolidays([]);
      setExceptions([]);
      setRotation(null);
      setSubjects([]);
    }
  }

  async function submitSlot(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await run(async () => {
      if (slotForm.id) {
        await updateClassSlot(slotForm.id, slotInput(slotForm));
        setNotice("Franja actualizada.");
        setSlotForm(emptySlotForm);
      } else {
        if (!slotForm.subjectId) {
          throw new Error("Elegí una materia para la franja.");
        }
        await createClassSlot(slotForm.subjectId, slotInput(slotForm));
        setNotice("Franja agregada al horario.");
      }
      await refreshAll(termId);
    });
  }

  async function submitHoliday(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!termId) {
      return;
    }
    await run(async () => {
      await createHoliday(termId, holidayForm.date, holidayForm.name);
      setHolidayForm(emptyHolidayForm);
      setNotice("Feriado agregado.");
      await refreshAll(termId);
    });
  }

  async function submitException(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await run(async () => {
      await createClassException(
        exceptionForm.slotId,
        exceptionForm.date,
        exceptionForm.kind,
        optionalText(exceptionForm.newStartTime),
        optionalText(exceptionForm.newEndTime),
        optionalText(exceptionForm.newRoom),
      );
      setExceptionForm(emptyExceptionForm);
      setNotice("Excepción registrada.");
      await refreshAll(termId);
    });
  }

  async function submitRotation(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!termId) {
      return;
    }
    await run(async () => {
      const dayCycle = splitLabels(rotationForm.dayCycle);
      const labels = splitLabels(rotationForm.labels);
      await saveRotation(termId, {
        mode: rotationForm.mode,
        anchorDate: rotationForm.anchorDate,
        cycleLength:
          rotationForm.mode === "day_cycle"
            ? dayCycle.length
            : rotationForm.cycleLength,
        labels: rotationForm.mode === "day_cycle" ? [] : labels,
        dayCycle: rotationForm.mode === "day_cycle" ? dayCycle : [],
      });
      setNotice("Rotación guardada.");
      await refreshAll(termId);
    });
  }

  async function markAttendance(
    occurrence: ScheduleOccurrence,
    status: AttendanceStatus | null,
  ) {
    await run(async () => {
      if (status === null) {
        await clearAttendance(occurrence.slotId, occurrence.date);
      } else {
        await setAttendance(occurrence.slotId, occurrence.date, status, null);
      }
      if (termId) {
        setWeek(await getWeekSchedule(termId, weekStart));
      }
    });
  }

  async function removeSlot(slot: ClassSlot) {
    await run(async () => {
      await deleteClassSlot(slot.id);
      if (slotForm.id === slot.id) {
        setSlotForm(emptySlotForm);
      }
      setNotice("Franja eliminada.");
      await refreshAll(termId);
    });
  }

  async function removeHoliday(holiday: Holiday) {
    await run(async () => {
      await deleteHoliday(holiday.id);
      setNotice("Feriado eliminado.");
      await refreshAll(termId);
    });
  }

  async function removeException(exception: ClassException) {
    await run(async () => {
      await deleteClassException(exception.id);
      setNotice("Excepción eliminada.");
      await refreshAll(termId);
    });
  }

  function slotLabel(slotId: string): string {
    const slot = slots.find((item) => item.id === slotId);
    if (!slot) {
      return "Franja eliminada";
    }
    const subject = subjects.find((item) => item.id === slot.subjectId);
    return `${subject?.name ?? "Materia"} · ${weekdayShort[slot.weekday - 1]} ${slot.startTime}`;
  }

  if (!ready) {
    return (
      <>
        <section className="hero section-hero">
          <p className="eyebrow">Horario · Fase 2</p>
          <h1>Horario semanal</h1>
          <p className="hero-copy">
            El horario se calcula en local a partir de tus materias.
          </p>
        </section>
        <section className="next-card">
          <p className="eyebrow">Bóveda requerida</p>
          <h2>Primero elegí dónde guardar tus datos</h2>
          <p>
            {vault?.message ??
              "Elegí una carpeta en Bóveda y volvé para cargar el horario."}
          </p>
          <button type="button" className="button filled" onClick={onGoVault}>
            Ir a Bóveda
          </button>
        </section>
      </>
    );
  }

  const dayLabels = weekDays.map((date) =>
    format(parseISO(date), "EEE d MMM", { locale: es }),
  );

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Horario · Fase 2</p>
        <h1>Horario semanal</h1>
        <p className="hero-copy">
          Rotaciones, feriados y excepciones se aplican sobre la semana. La
          asistencia se registra por clase y alimenta el umbral de cada materia.
        </p>
      </section>

      {error ? (
        <p className="notice error" role="alert">
          {error}
        </p>
      ) : notice ? (
        <p className="notice" role="status">
          {notice}
        </p>
      ) : null}

      {terms.length === 0 ? (
        <section className="next-card">
          <p className="eyebrow">Sin términos</p>
          <h2>Primero cargá un año y un término</h2>
          <p>
            El horario necesita materias dentro de un término académico. Podés
            crearlos en la sección Materias.
          </p>
          <button
            type="button"
            className="button filled"
            onClick={onGoSubjects}
          >
            Ir a Materias
          </button>
        </section>
      ) : (
        <>
          <section className="status-card" aria-labelledby="week-title">
            <div className="week-head">
              <div>
                <p className="eyebrow">Semana</p>
                <h2 id="week-title">
                  {week
                    ? `${week.weekStart} → ${week.weekEnd}`
                    : "Cargando semana…"}
                </h2>
                <p>
                  {week?.rotationLabel
                    ? `Rotación ${week.rotationLabel}`
                    : "Sin rotación configurada"}
                </p>
              </div>
              <div className="week-tools">
                <label className="field">
                  <span>Término</span>
                  <select
                    className="select-field"
                    value={termId ?? ""}
                    onChange={(event) => void selectTerm(event.target.value)}
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
                    onClick={() => void moveWeek(-1)}
                  >
                    ← Anterior
                  </button>
                  <button
                    type="button"
                    className="button"
                    onClick={() => void goCurrentWeek()}
                  >
                    Hoy
                  </button>
                  <button
                    type="button"
                    className="button tonal"
                    onClick={() => void moveWeek(1)}
                  >
                    Siguiente →
                  </button>
                </div>
              </div>
            </div>

            {loading ? (
              <p className="notice">Cargando horario…</p>
            ) : (
              <div className="week-grid">
                {weekDays.map((date, index) => {
                  const holiday = week?.holidays.find(
                    (item) => item.date === date,
                  );
                  const occurrences =
                    week?.occurrences.filter((item) => item.date === date) ??
                    [];
                  return (
                    <div className="week-day" key={date}>
                      <div className="week-day-head">
                        <span>{dayLabels[index]}</span>
                        {holiday ? (
                          <span className="kind-chip holiday">Feriado</span>
                        ) : null}
                      </div>
                      {holiday ? (
                        <p className="week-holiday">{holiday.name}</p>
                      ) : null}
                      {occurrences.length === 0 ? (
                        <p className="quiet week-empty">Sin clases</p>
                      ) : (
                        <ul className="occurrence-list">
                          {occurrences.map((occurrence) => (
                            <li
                              className={`occurrence kind-${occurrence.kind}`}
                              key={`${occurrence.slotId}-${occurrence.date}-${occurrence.kind}`}
                            >
                              <div className="occurrence-head">
                                <span
                                  className="subject-dot"
                                  style={{
                                    background: occurrence.subjectColor,
                                  }}
                                  aria-hidden="true"
                                />
                                <strong>{occurrence.subjectName}</strong>
                                <span className="quiet">
                                  {occurrence.startTime}–{occurrence.endTime}
                                </span>
                              </div>
                              <div className="occurrence-meta">
                                {occurrence.kind !== "scheduled" ? (
                                  <span
                                    className={`kind-chip ${occurrence.kind}`}
                                  >
                                    {occurrenceLabels[occurrence.kind]}
                                  </span>
                                ) : null}
                                {occurrence.room ? (
                                  <span className="quiet">
                                    {occurrence.room}
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
                    </div>
                  );
                })}
              </div>
            )}
          </section>

          <div className="schedule-grid">
            <section
              className="academic-panel"
              aria-labelledby="overlaps-title"
            >
              <div className="task-board-head">
                <h2 id="overlaps-title">Solapamientos</h2>
                <span className="count-chip">{week?.overlaps.length ?? 0}</span>
              </div>
              {!week || week.overlaps.length === 0 ? (
                <p className="empty-state">No hay clases superpuestas.</p>
              ) : (
                <ul className="mini-list">
                  {week.overlaps.map((overlap) => (
                    <li
                      className="overlap-row"
                      key={`${overlap.firstSlotId}-${overlap.secondSlotId}`}
                    >
                      <span>
                        {weekdayLabels[overlap.weekday - 1]} ·{" "}
                        {overlap.startTime}–{overlap.endTime}
                      </span>
                      <span className="quiet">
                        {overlap.firstSubjectName} / {overlap.secondSubjectName}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </section>

            <section
              className="academic-panel"
              aria-labelledby="attendance-summary-title"
            >
              <div className="task-board-head">
                <h2 id="attendance-summary-title">Asistencia</h2>
              </div>
              {!week || week.attendance.length === 0 ? (
                <p className="empty-state">Todavía no hay materias cargadas.</p>
              ) : (
                <ul className="mini-list">
                  {week.attendance.map((summary) => (
                    <li className="attendance-summary" key={summary.subjectId}>
                      <span
                        className="subject-dot"
                        style={{ background: summary.subjectColor }}
                        aria-hidden="true"
                      />
                      <span className="attendance-name">
                        {summary.subjectName}
                      </span>
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
              )}
              {week?.attendance.some((summary) => summary.belowMin) ? (
                <p className="notice error">
                  Hay materias por debajo del umbral de asistencia.
                </p>
              ) : null}
            </section>
          </div>

          <div className="schedule-grid">
            <section className="academic-panel" aria-labelledby="slots-title">
              <div className="task-board-head">
                <h2 id="slots-title">Franjas horarias</h2>
                <span className="count-chip">{slots.length}</span>
              </div>
              {slots.length === 0 ? (
                <p className="empty-state">
                  Agregá la primera franja con materia, día y horario.
                </p>
              ) : (
                <ul className="mini-list">
                  {slots.map((slot) => (
                    <li key={slot.id}>
                      <span className="mini-item">
                        <span>{slotLabel(slot.id)}</span>
                        <span className="quiet">
                          {slot.recurrence === "biweekly"
                            ? "Bisemanal"
                            : "Semanal"}
                          {slot.rotationKey
                            ? ` · rotación ${slot.rotationKey}`
                            : ""}
                          {slot.validFrom ? ` · desde ${slot.validFrom}` : ""}
                          {slot.validTo ? ` · hasta ${slot.validTo}` : ""}
                        </span>
                      </span>
                      <button
                        type="button"
                        className="mini-action"
                        onClick={() =>
                          setSlotForm({
                            id: slot.id,
                            subjectId: slot.subjectId,
                            weekday: slot.weekday,
                            startTime: slot.startTime,
                            endTime: slot.endTime,
                            recurrence: slot.recurrence,
                            rotationKey: slot.rotationKey ?? "",
                            validFrom: slot.validFrom ?? "",
                            validTo: slot.validTo ?? "",
                            roomOverride: slot.roomOverride ?? "",
                          })
                        }
                      >
                        Editar
                      </button>
                      <button
                        type="button"
                        className="mini-action danger"
                        onClick={() => void removeSlot(slot)}
                        disabled={busy}
                        aria-label={`Eliminar franja de ${slotLabel(slot.id)}`}
                      >
                        ×
                      </button>
                    </li>
                  ))}
                </ul>
              )}

              <form className="inline-form" onSubmit={submitSlot}>
                <label className="field">
                  <span>Materia</span>
                  <select
                    className="select-field"
                    value={slotForm.subjectId}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        subjectId: event.target.value,
                      })
                    }
                    disabled={slotForm.id !== null}
                    required
                  >
                    <option value="">Elegir…</option>
                    {subjects.map((subject) => (
                      <option key={subject.id} value={subject.id}>
                        {subject.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  <span>Día</span>
                  <select
                    className="select-field"
                    value={slotForm.weekday}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        weekday: Number(event.target.value),
                      })
                    }
                  >
                    {weekdayLabels.map((label, index) => (
                      <option key={label} value={index + 1}>
                        {label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field narrow">
                  <span>Inicio</span>
                  <input
                    className="text-field"
                    type="time"
                    value={slotForm.startTime}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        startTime: event.target.value,
                      })
                    }
                    required
                  />
                </label>
                <label className="field narrow">
                  <span>Fin</span>
                  <input
                    className="text-field"
                    type="time"
                    value={slotForm.endTime}
                    onChange={(event) =>
                      setSlotForm({ ...slotForm, endTime: event.target.value })
                    }
                    required
                  />
                </label>
                <label className="field">
                  <span>Recurrencia</span>
                  <select
                    className="select-field"
                    value={slotForm.recurrence}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        recurrence: event.target.value as Recurrence,
                      })
                    }
                  >
                    <option value="weekly">Semanal</option>
                    <option value="biweekly">Bisemanal</option>
                  </select>
                </label>
                <label className="field narrow">
                  <span>Rotación</span>
                  <input
                    className="text-field"
                    value={slotForm.rotationKey}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        rotationKey: event.target.value,
                      })
                    }
                    maxLength={40}
                    placeholder="A"
                  />
                </label>
                <label className="field narrow">
                  <span>Desde</span>
                  <input
                    className="text-field"
                    type="date"
                    value={slotForm.validFrom}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        validFrom: event.target.value,
                      })
                    }
                  />
                </label>
                <label className="field narrow">
                  <span>Hasta</span>
                  <input
                    className="text-field"
                    type="date"
                    value={slotForm.validTo}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        validTo: event.target.value,
                      })
                    }
                  />
                </label>
                <label className="field">
                  <span>Aula</span>
                  <input
                    className="text-field"
                    value={slotForm.roomOverride}
                    onChange={(event) =>
                      setSlotForm({
                        ...slotForm,
                        roomOverride: event.target.value,
                      })
                    }
                    maxLength={120}
                  />
                </label>
                <div className="form-actions">
                  <button
                    type="submit"
                    className="button filled"
                    disabled={busy}
                  >
                    {slotForm.id ? "Guardar franja" : "Agregar franja"}
                  </button>
                  {slotForm.id ? (
                    <button
                      type="button"
                      className="button"
                      onClick={() => setSlotForm(emptySlotForm)}
                    >
                      Cancelar
                    </button>
                  ) : null}
                </div>
              </form>
            </section>

            <section
              className="academic-panel"
              aria-labelledby="rotation-title"
            >
              <div className="task-board-head">
                <h2 id="rotation-title">Rotación</h2>
                <span className="count-chip">
                  {rotation
                    ? rotationModeLabels[rotation.mode]
                    : "Sin configurar"}
                </span>
              </div>
              <form className="inline-form" onSubmit={submitRotation}>
                <label className="field">
                  <span>Modo</span>
                  <select
                    className="select-field"
                    value={rotationForm.mode}
                    onChange={(event) =>
                      setRotationForm({
                        ...rotationForm,
                        mode: event.target.value as RotationMode,
                      })
                    }
                  >
                    <option value="weekly_ab">Semanal A/B</option>
                    <option value="parity">Paridad de semanas</option>
                    <option value="day_cycle">Ciclo de días</option>
                  </select>
                </label>
                <label className="field narrow">
                  <span>Ancla</span>
                  <input
                    className="text-field"
                    type="date"
                    value={rotationForm.anchorDate}
                    onChange={(event) =>
                      setRotationForm({
                        ...rotationForm,
                        anchorDate: event.target.value,
                      })
                    }
                    required
                  />
                </label>
                {rotationForm.mode === "day_cycle" ? (
                  <label className="field">
                    <span>Ciclo (separado por comas)</span>
                    <input
                      className="text-field"
                      value={rotationForm.dayCycle}
                      onChange={(event) =>
                        setRotationForm({
                          ...rotationForm,
                          dayCycle: event.target.value,
                        })
                      }
                      placeholder="1, 2, 3, 4"
                      required
                    />
                  </label>
                ) : (
                  <>
                    <label className="field narrow">
                      <span>Ciclo</span>
                      <input
                        className="text-field"
                        type="number"
                        min={1}
                        max={12}
                        value={rotationForm.cycleLength}
                        onChange={(event) =>
                          setRotationForm({
                            ...rotationForm,
                            cycleLength: Number(event.target.value),
                          })
                        }
                      />
                    </label>
                    <label className="field">
                      <span>Etiquetas</span>
                      <input
                        className="text-field"
                        value={rotationForm.labels}
                        onChange={(event) =>
                          setRotationForm({
                            ...rotationForm,
                            labels: event.target.value,
                          })
                        }
                        placeholder="A, B"
                      />
                    </label>
                  </>
                )}
                <div className="form-actions">
                  <button
                    type="submit"
                    className="button filled"
                    disabled={busy}
                  >
                    Guardar rotación
                  </button>
                </div>
              </form>
            </section>
          </div>

          <div className="schedule-grid">
            <section
              className="academic-panel"
              aria-labelledby="holidays-title"
            >
              <div className="task-board-head">
                <h2 id="holidays-title">Feriados</h2>
                <span className="count-chip">{holidays.length}</span>
              </div>
              {holidays.length === 0 ? (
                <p className="empty-state">No hay feriados cargados.</p>
              ) : (
                <ul className="mini-list">
                  {holidays.map((holiday) => (
                    <li key={holiday.id}>
                      <span className="mini-item">
                        <span>{holiday.name}</span>
                        <span className="quiet">{holiday.date}</span>
                      </span>
                      <button
                        type="button"
                        className="mini-action danger"
                        onClick={() => void removeHoliday(holiday)}
                        disabled={busy}
                        aria-label={`Eliminar feriado «${holiday.name}»`}
                      >
                        ×
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              <form className="inline-form" onSubmit={submitHoliday}>
                <label className="field narrow">
                  <span>Fecha</span>
                  <input
                    className="text-field"
                    type="date"
                    value={holidayForm.date}
                    onChange={(event) =>
                      setHolidayForm({
                        ...holidayForm,
                        date: event.target.value,
                      })
                    }
                    required
                  />
                </label>
                <label className="field">
                  <span>Nombre</span>
                  <input
                    className="text-field"
                    value={holidayForm.name}
                    onChange={(event) =>
                      setHolidayForm({
                        ...holidayForm,
                        name: event.target.value,
                      })
                    }
                    maxLength={120}
                    required
                  />
                </label>
                <div className="form-actions">
                  <button
                    type="submit"
                    className="button filled"
                    disabled={busy}
                  >
                    Agregar feriado
                  </button>
                </div>
              </form>
            </section>

            <section
              className="academic-panel"
              aria-labelledby="exceptions-title"
            >
              <div className="task-board-head">
                <h2 id="exceptions-title">Excepciones</h2>
                <span className="count-chip">{exceptions.length}</span>
              </div>
              {exceptions.length === 0 ? (
                <p className="empty-state">
                  No hay clases canceladas, movidas ni extra.
                </p>
              ) : (
                <ul className="mini-list">
                  {exceptions.map((exception) => (
                    <li key={exception.id}>
                      <span className="mini-item">
                        <span>
                          {kindLabels[exception.kind]} · {exception.date}
                        </span>
                        <span className="quiet">
                          {slotLabel(exception.classSlotId)}
                          {exception.newStartTime
                            ? ` · ${exception.newStartTime}–${exception.newEndTime ?? ""}`
                            : ""}
                          {exception.newRoom ? ` · ${exception.newRoom}` : ""}
                        </span>
                      </span>
                      <button
                        type="button"
                        className="mini-action danger"
                        onClick={() => void removeException(exception)}
                        disabled={busy}
                        aria-label={`Eliminar excepción del ${exception.date}`}
                      >
                        ×
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              <form className="inline-form" onSubmit={submitException}>
                <label className="field">
                  <span>Franja</span>
                  <select
                    className="select-field"
                    value={exceptionForm.slotId}
                    onChange={(event) =>
                      setExceptionForm({
                        ...exceptionForm,
                        slotId: event.target.value,
                      })
                    }
                    required
                  >
                    <option value="">Elegir…</option>
                    {slots.map((slot) => (
                      <option key={slot.id} value={slot.id}>
                        {slotLabel(slot.id)}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field narrow">
                  <span>Fecha</span>
                  <input
                    className="text-field"
                    type="date"
                    value={exceptionForm.date}
                    onChange={(event) =>
                      setExceptionForm({
                        ...exceptionForm,
                        date: event.target.value,
                      })
                    }
                    required
                  />
                </label>
                <label className="field">
                  <span>Tipo</span>
                  <select
                    className="select-field"
                    value={exceptionForm.kind}
                    onChange={(event) =>
                      setExceptionForm({
                        ...exceptionForm,
                        kind: event.target.value as ExceptionKind,
                      })
                    }
                  >
                    {Object.entries(kindLabels).map(([value, label]) => (
                      <option key={value} value={value}>
                        {label}
                      </option>
                    ))}
                  </select>
                </label>
                {exceptionForm.kind !== "cancelled" ? (
                  <>
                    <label className="field narrow">
                      <span>Nuevo inicio</span>
                      <input
                        className="text-field"
                        type="time"
                        value={exceptionForm.newStartTime}
                        onChange={(event) =>
                          setExceptionForm({
                            ...exceptionForm,
                            newStartTime: event.target.value,
                          })
                        }
                      />
                    </label>
                    <label className="field narrow">
                      <span>Nuevo fin</span>
                      <input
                        className="text-field"
                        type="time"
                        value={exceptionForm.newEndTime}
                        onChange={(event) =>
                          setExceptionForm({
                            ...exceptionForm,
                            newEndTime: event.target.value,
                          })
                        }
                      />
                    </label>
                    <label className="field">
                      <span>Nueva aula</span>
                      <input
                        className="text-field"
                        value={exceptionForm.newRoom}
                        onChange={(event) =>
                          setExceptionForm({
                            ...exceptionForm,
                            newRoom: event.target.value,
                          })
                        }
                        maxLength={120}
                      />
                    </label>
                  </>
                ) : null}
                <div className="form-actions">
                  <button
                    type="submit"
                    className="button filled"
                    disabled={busy}
                  >
                    Agregar excepción
                  </button>
                </div>
              </form>
            </section>
          </div>
        </>
      )}
    </>
  );
}
