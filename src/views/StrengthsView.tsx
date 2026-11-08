import { format } from "date-fns";
import { type FormEvent, useEffect, useMemo, useState } from "react";
import {
  clearStrengthOverride,
  createSelfAssessment,
  listAcademicYears,
  listSubjects,
  listTerms,
  listTopicStudies,
  reviewTopic,
  type StrengthClassification,
  type Subject,
  setReviewDue,
  setStrengthOverride,
  type TopicStudy,
  type VaultStatus,
} from "../lib/ipc";

const classificationLabels: Record<StrengthClassification, string> = {
  weak: "Débil",
  neutral: "Neutral",
  strength: "Fortaleza",
};

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function todayIso(): string {
  return format(new Date(), "yyyy-MM-dd");
}

function scoreText(score: number): string {
  return `${score} / 5`;
}

export function StrengthsView({
  vault,
  onGoVault,
  onGoSubjects,
  onGoExams,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  onGoSubjects: () => void;
  onGoExams: () => void;
}) {
  const ready = vault?.ready === true;
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [subjectId, setSubjectId] = useState("");
  const [studies, setStudies] = useState<TopicStudy[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [score, setScore] = useState("3");
  const [note, setNote] = useState("");
  const [overrideClass, setOverrideClass] =
    useState<StrengthClassification>("neutral");
  const [overrideReason, setOverrideReason] = useState("");
  const [quality, setQuality] = useState("4");
  const [dueDate, setDueDate] = useState("");

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
      setStudies([]);
      return;
    }
    let cancelled = false;

    async function loadStudies() {
      try {
        const loaded = await listTopicStudies(subjectId, todayIso());
        if (!cancelled) {
          setStudies(loaded);
          setSelectedId((current) =>
            current && loaded.some((item) => item.topicId === current)
              ? current
              : (loaded[0]?.topicId ?? null),
          );
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadStudies();
    return () => {
      cancelled = true;
    };
  }, [ready, subjectId]);

  const selected = useMemo(
    () => studies.find((item) => item.topicId === selectedId) ?? null,
    [studies, selectedId],
  );

  const counts = useMemo(() => {
    const base = { weak: 0, neutral: 0, strength: 0, due: 0 };
    for (const study of studies) {
      base[study.classification] += 1;
      if (study.reviewItem?.dueDate && study.reviewItem.dueDate <= todayIso()) {
        base.due += 1;
      }
    }
    return base;
  }, [studies]);

  async function reload() {
    if (!subjectId) {
      return;
    }
    const loaded = await listTopicStudies(subjectId, todayIso());
    setStudies(loaded);
  }

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await action();
    } catch (reason) {
      setError(messageOf(reason));
    } finally {
      setBusy(false);
    }
  }

  function selectTopic(topic: TopicStudy) {
    setSelectedId(topic.topicId);
    setScore(String(topic.latestAssessment?.score ?? 3));
    setNote("");
    setOverrideClass(topic.overrideClassification ?? "neutral");
    setOverrideReason(topic.overrideReason ?? "");
    setQuality("4");
    setDueDate(topic.reviewItem?.dueDate ?? "");
    setNotice(null);
    setError(null);
  }

  async function handleAssessment(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected) {
      return;
    }
    await run(async () => {
      await createSelfAssessment(
        selected.topicId,
        Number(score),
        note.trim() || null,
      );
      await reload();
      setNote("");
      setNotice("Autoevaluación guardada.");
    });
  }

  async function handleOverride(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected) {
      return;
    }
    await run(async () => {
      await setStrengthOverride(
        selected.topicId,
        overrideClass,
        overrideReason.trim() || null,
      );
      await reload();
      setNotice("Clasificación manual aplicada.");
    });
  }

  async function handleClearOverride() {
    if (!selected) {
      return;
    }
    await run(async () => {
      await clearStrengthOverride(selected.topicId);
      await reload();
      setOverrideReason("");
      setNotice("Override eliminado; vuelve la clasificación automática.");
    });
  }

  async function handleReview() {
    if (!selected) {
      return;
    }
    await run(async () => {
      const item = await reviewTopic(
        selected.topicId,
        Number(quality),
        todayIso(),
      );
      await reload();
      setDueDate(item.dueDate ?? "");
      setNotice(
        `Repaso registrado. Próximo vencimiento: ${item.dueDate ?? "sin fecha"}.`,
      );
    });
  }

  async function handleScheduleReview() {
    if (!selected || !dueDate) {
      setError("Elegí una fecha para el repaso.");
      return;
    }
    await run(async () => {
      await setReviewDue(selected.topicId, dueDate);
      await reload();
      setNotice("Fecha de repaso actualizada.");
    });
  }

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Fortalezas · autoevaluación y repaso</p>
        <h1>Fortalezas</h1>
        <p className="hero-copy">
          Autoevaluá cada tema del 1 al 5, ajustá la clasificación con un
          override justificado y programá el repaso espaciado. Todo queda en tu
          bóveda local.
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
            Las autoevaluaciones y el repaso viven en tu bóveda local. Elegí una
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
                Necesitás al menos una materia con temas para autoevaluar y
                programar repasos.
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
                <label className="field">
                  <span>Materia</span>
                  <select
                    className="select-field"
                    value={subjectId}
                    onChange={(event) => {
                      setSubjectId(event.target.value);
                      setSelectedId(null);
                      setNotice(null);
                      setError(null);
                    }}
                  >
                    {subjects.map((subject) => (
                      <option key={subject.id} value={subject.id}>
                        {subject.name}
                      </option>
                    ))}
                  </select>
                </label>
                <div className="study-summary">
                  <span className="status-pill is-low">
                    {counts.weak} débiles
                  </span>
                  <span className="status-pill">
                    {counts.neutral} neutrales
                  </span>
                  <span className="status-pill is-ok">
                    {counts.strength} fortalezas
                  </span>
                  <span className="status-pill">
                    {counts.due} repasos vencidos
                  </span>
                </div>
              </section>

              {studies.length === 0 ? (
                <section className="next-card">
                  <p className="eyebrow">Temas requeridos</p>
                  <h2>Esta materia todavía no tiene temas</h2>
                  <p>
                    Creá los temas en la sección Exámenes; después volvé para
                    autoevaluarlos y programar repasos.
                  </p>
                  <div className="hero-actions">
                    <button
                      type="button"
                      className="button filled"
                      onClick={onGoExams}
                    >
                      Ir a Exámenes
                    </button>
                  </div>
                </section>
              ) : (
                <div className="study-layout">
                  <ul className="mini-list" aria-label="Temas de la materia">
                    {studies.map((study) => (
                      <li key={study.topicId}>
                        <button
                          type="button"
                          className={`mini-item study-topic${
                            selectedId === study.topicId ? " is-active" : ""
                          }`}
                          onClick={() => selectTopic(study)}
                        >
                          <span
                            className={`strength-chip is-${study.classification}`}
                          >
                            {classificationLabels[study.classification]}
                          </span>
                          <strong>{study.name}</strong>
                          <span className="quiet">
                            {study.latestAssessment
                              ? `Última: ${scoreText(study.latestAssessment.score)}`
                              : "Sin evaluar"}
                          </span>
                          {study.reviewItem?.dueDate ? (
                            <span className="quiet">
                              Repaso: {study.reviewItem.dueDate}
                            </span>
                          ) : null}
                        </button>
                      </li>
                    ))}
                  </ul>

                  {selected ? (
                    <section className="status-card study-detail">
                      <div>
                        <p className="eyebrow">Tema seleccionado</p>
                        <h2>{selected.name}</h2>
                        <p className="quiet">
                          Clasificación automática:{" "}
                          {classificationLabels[selected.derivedClassification]}
                          {selected.overrideClassification
                            ? ` · override: ${classificationLabels[selected.overrideClassification]}`
                            : ""}
                        </p>
                        {selected.description ? (
                          <p>{selected.description}</p>
                        ) : null}
                        {selected.nextExamDate ? (
                          <p className="quiet">
                            Próximo examen: {selected.nextExamDate}
                          </p>
                        ) : null}
                      </div>

                      <div className="study-detail-grid">
                        <form
                          className="inline-form"
                          onSubmit={handleAssessment}
                        >
                          <p className="eyebrow">Autoevaluación</p>
                          <label className="field narrow">
                            <span>Puntaje (1–5)</span>
                            <select
                              className="select-field"
                              aria-label="Autoevaluación"
                              value={score}
                              onChange={(event) => setScore(event.target.value)}
                            >
                              {[1, 2, 3, 4, 5].map((value) => (
                                <option key={value} value={value}>
                                  {value}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label className="field">
                            <span>Nota (opcional)</span>
                            <input
                              className="text-field"
                              aria-label="Nota de autoevaluación"
                              value={note}
                              maxLength={120}
                              onChange={(event) => setNote(event.target.value)}
                            />
                          </label>
                          <button
                            type="submit"
                            className="button filled"
                            disabled={busy}
                          >
                            Guardar autoevaluación
                          </button>
                        </form>

                        <form className="inline-form" onSubmit={handleOverride}>
                          <p className="eyebrow">Override manual</p>
                          <label className="field narrow">
                            <span>Clasificación</span>
                            <select
                              className="select-field"
                              aria-label="Clasificación manual"
                              value={overrideClass}
                              onChange={(event) =>
                                setOverrideClass(
                                  event.target.value as StrengthClassification,
                                )
                              }
                            >
                              <option value="weak">Débil</option>
                              <option value="neutral">Neutral</option>
                              <option value="strength">Fortaleza</option>
                            </select>
                          </label>
                          <label className="field">
                            <span>Motivo (opcional)</span>
                            <input
                              className="text-field"
                              aria-label="Motivo del override"
                              value={overrideReason}
                              maxLength={120}
                              onChange={(event) =>
                                setOverrideReason(event.target.value)
                              }
                            />
                          </label>
                          <div className="form-actions">
                            <button
                              type="submit"
                              className="button tonal"
                              disabled={busy}
                            >
                              Aplicar override
                            </button>
                            {selected.overrideClassification ? (
                              <button
                                type="button"
                                className="mini-action danger"
                                disabled={busy}
                                onClick={() => void handleClearOverride()}
                              >
                                Quitar override
                              </button>
                            ) : null}
                          </div>
                        </form>
                      </div>

                      <div className="study-detail-grid">
                        <div className="inline-form">
                          <p className="eyebrow">Repaso espaciado</p>
                          {selected.reviewItem ? (
                            <ul className="stats-list">
                              <li>
                                <span>Facilidad</span>
                                <strong>
                                  {(
                                    selected.reviewItem.easeFactorCp / 100
                                  ).toFixed(2)}
                                </strong>
                              </li>
                              <li>
                                <span>Intervalo</span>
                                <strong>
                                  {selected.reviewItem.intervalDays} día(s)
                                </strong>
                              </li>
                              <li>
                                <span>Repeticiones</span>
                                <strong>
                                  {selected.reviewItem.repetitions}
                                </strong>
                              </li>
                              <li>
                                <span>Lapsos</span>
                                <strong>{selected.reviewItem.lapses}</strong>
                              </li>
                              <li>
                                <span>Próximo repaso</span>
                                <strong>
                                  {selected.reviewItem.dueDate ?? "sin fecha"}
                                </strong>
                              </li>
                            </ul>
                          ) : (
                            <p className="quiet">
                              Este tema todavía no tiene repaso programado.
                            </p>
                          )}

                          <label className="field narrow">
                            <span>Calidad del repaso</span>
                            <select
                              className="select-field"
                              aria-label="Calidad del repaso"
                              value={quality}
                              onChange={(event) =>
                                setQuality(event.target.value)
                              }
                            >
                              {[1, 2, 3, 4, 5].map((value) => (
                                <option key={value} value={value}>
                                  {value}
                                </option>
                              ))}
                            </select>
                          </label>
                          <button
                            type="button"
                            className="button filled"
                            disabled={busy}
                            onClick={() => void handleReview()}
                          >
                            Registrar repaso
                          </button>
                        </div>

                        <div className="inline-form">
                          <p className="eyebrow">Programar repaso</p>
                          <label className="field">
                            <span>Fecha</span>
                            <input
                              type="date"
                              className="text-field"
                              aria-label="Fecha de repaso"
                              value={dueDate}
                              onChange={(event) =>
                                setDueDate(event.target.value)
                              }
                            />
                          </label>
                          <button
                            type="button"
                            className="button tonal"
                            disabled={busy}
                            onClick={() => void handleScheduleReview()}
                          >
                            Guardar fecha
                          </button>
                        </div>
                      </div>

                      {selected.latestAssessment ? (
                        <p className="quiet">
                          Última autoevaluación:{" "}
                          {scoreText(selected.latestAssessment.score)} · origen{" "}
                          {selected.latestAssessment.source} ·{" "}
                          {selected.latestAssessment.assessedAt.slice(0, 10)}
                        </p>
                      ) : null}
                    </section>
                  ) : null}
                </div>
              )}
            </>
          )}
        </>
      )}
    </>
  );
}
