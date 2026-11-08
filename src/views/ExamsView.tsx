import { type FormEvent, useEffect, useState } from "react";
import {
  createExam,
  createTopic,
  deleteExam,
  deleteTopic,
  type Exam,
  type ExamInput,
  type ExamResultStatus,
  type ExamTopic,
  linkExamTopic,
  listAcademicYears,
  listExams,
  listExamTopics,
  listSubjects,
  listTerms,
  listTopics,
  type Subject,
  type Topic,
  type TopicInput,
  unlinkExamTopic,
  updateExam,
  type VaultStatus,
} from "../lib/ipc";

const statusLabels: Record<ExamResultStatus, string> = {
  pending: "Pendiente",
  passed: "Aprobado",
  failed: "Desaprobado",
  absent: "Ausente",
};

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function optionalText(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length === 0 ? null : trimmed;
}

function optionalNumber(value: string): number | null {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return null;
  }
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
}

function cpToValue(value: number | null): string {
  return value === null ? "—" : (value / 100).toFixed(2);
}

const emptyExam = {
  id: null as string | null,
  title: "",
  date: "",
  startTime: "",
  endTime: "",
  room: "",
  weight: "",
  resultScore: "",
  resultMax: "",
  resultStatus: "pending" as ExamResultStatus,
  notes: "",
};

const emptyTopic = {
  id: null as string | null,
  name: "",
  parentTopicId: "",
  description: "",
  sortOrder: "0",
};

type ExamForm = typeof emptyExam;

function examInputFromForm(form: ExamForm): ExamInput {
  return {
    title: form.title,
    date: form.date,
    startTime: optionalText(form.startTime),
    endTime: optionalText(form.endTime),
    room: optionalText(form.room),
    weightBp:
      optionalNumber(form.weight) === null
        ? null
        : Math.round((optionalNumber(form.weight) ?? 0) * 100),
    resultScoreCp:
      optionalNumber(form.resultScore) === null
        ? null
        : Math.round((optionalNumber(form.resultScore) ?? 0) * 100),
    resultMaxCp:
      optionalNumber(form.resultMax) === null
        ? null
        : Math.round((optionalNumber(form.resultMax) ?? 0) * 100),
    resultStatus: form.resultStatus,
    notes: optionalText(form.notes),
  };
}

function formFromExam(exam: Exam): ExamForm {
  return {
    id: exam.id,
    title: exam.title,
    date: exam.date,
    startTime: exam.startTime ?? "",
    endTime: exam.endTime ?? "",
    room: exam.room ?? "",
    weight: exam.weightBp === null ? "" : String(exam.weightBp / 100),
    resultScore:
      exam.resultScoreCp === null ? "" : String(exam.resultScoreCp / 100),
    resultMax: exam.resultMaxCp === null ? "" : String(exam.resultMaxCp / 100),
    resultStatus: exam.resultStatus,
    notes: exam.notes ?? "",
  };
}

export function ExamsView({
  vault,
  onGoVault,
  onGoSubjects,
}: {
  vault: VaultStatus | null;
  onGoVault: () => void;
  onGoSubjects: () => void;
}) {
  const ready = vault?.ready === true;
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [subjectId, setSubjectId] = useState("");
  const [exams, setExams] = useState<Exam[]>([]);
  const [topics, setTopics] = useState<Topic[]>([]);
  const [links, setLinks] = useState<Record<string, string[]>>({});
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [examForm, setExamForm] = useState<ExamForm>(emptyExam);
  const [topicForm, setTopicForm] = useState(emptyTopic);

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function loadSubjects() {
      try {
        const years = await listAcademicYears();
        const terms = years[0] ? await listTerms(years[0].id) : [];
        const loaded = terms[0] ? await listSubjects(terms[0].id) : [];
        if (!cancelled) {
          setSubjects(loaded);
          setSubjectId((current) => current || (loaded[0]?.id ?? ""));
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadSubjects();
    return () => {
      cancelled = true;
    };
  }, [ready]);

  useEffect(() => {
    if (!ready || !subjectId) {
      return;
    }
    let cancelled = false;

    async function loadData() {
      try {
        const [loadedExams, loadedTopics] = await Promise.all([
          listExams(null, subjectId),
          listTopics(subjectId),
        ]);
        if (cancelled) {
          return;
        }
        setExams(loadedExams);
        setTopics(loadedTopics);
        const entries = await Promise.all(
          loadedExams.map(async (exam) => {
            const examTopics = await listExamTopics(exam.id);
            return [exam.id, examTopics] as const;
          }),
        );
        if (!cancelled) {
          const map: Record<string, string[]> = {};
          for (const [examId, examTopics] of entries) {
            map[examId] = examTopics.map((link: ExamTopic) => link.topicId);
          }
          setLinks(map);
        }
      } catch (reason) {
        if (!cancelled) {
          setError(messageOf(reason));
        }
      }
    }

    void loadData();
    return () => {
      cancelled = true;
    };
  }, [ready, subjectId]);

  async function refresh() {
    if (!subjectId) {
      return;
    }
    const [loadedExams, loadedTopics] = await Promise.all([
      listExams(null, subjectId),
      listTopics(subjectId),
    ]);
    setExams(loadedExams);
    setTopics(loadedTopics);
    const entries = await Promise.all(
      loadedExams.map(async (exam) => {
        const examTopics = await listExamTopics(exam.id);
        return [exam.id, examTopics] as const;
      }),
    );
    const map: Record<string, string[]> = {};
    for (const [examId, examTopics] of entries) {
      map[examId] = examTopics.map((link) => link.topicId);
    }
    setLinks(map);
  }

  async function submitExam(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!subjectId) {
      return;
    }
    const subject = subjects.find((item) => item.id === subjectId);
    if (!subject) {
      return;
    }
    setError(null);
    try {
      if (examForm.id) {
        await updateExam(examForm.id, examInputFromForm(examForm));
        setNotice("Examen actualizado.");
      } else {
        await createExam(
          subjectId,
          subject.termId,
          examInputFromForm(examForm),
        );
        setNotice("Examen creado.");
      }
      setExamForm(emptyExam);
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function submitTopic(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!subjectId) {
      return;
    }
    setError(null);
    const input: TopicInput = {
      parentTopicId: optionalText(topicForm.parentTopicId),
      name: topicForm.name,
      description: optionalText(topicForm.description),
      sortOrder: Math.round(optionalNumber(topicForm.sortOrder) ?? 0),
    };
    try {
      await createTopic(subjectId, input);
      setTopicForm(emptyTopic);
      setNotice("Tema creado.");
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleDeleteExam(exam: Exam) {
    setError(null);
    try {
      await deleteExam(exam.id);
      if (examForm.id === exam.id) {
        setExamForm(emptyExam);
      }
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleDeleteTopic(topic: Topic) {
    setError(null);
    try {
      await deleteTopic(topic.id);
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function toggleLink(examId: string, topicId: string, checked: boolean) {
    setError(null);
    try {
      if (checked) {
        await linkExamTopic(examId, topicId);
      } else {
        await unlinkExamTopic(examId, topicId);
      }
      setLinks((current) => {
        const existing = new Set(current[examId] ?? []);
        if (checked) {
          existing.add(topicId);
        } else {
          existing.delete(topicId);
        }
        return { ...current, [examId]: [...existing] };
      });
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Exámenes · temas y pesos</p>
        <h1>Exámenes</h1>
        <p className="hero-copy">
          Fechas, pesos, resultados y temas vinculados. Todo local y sin
          dependencias de red.
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
            Los exámenes se guardan en <b>omega</b>. Elegí una carpeta en la
            sección Bóveda y volvé acá.
          </p>
          <div className="hero-actions">
            <button type="button" className="button filled" onClick={onGoVault}>
              Ir a Bóveda
            </button>
          </div>
        </section>
      ) : subjects.length === 0 ? (
        <section className="next-card">
          <p className="eyebrow">Materias requeridas</p>
          <h2>Creá una materia primero</h2>
          <p>Los exámenes se asocian a una materia y un término.</p>
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
            <div className="inline-form">
              <label className="field">
                <span>Materia</span>
                <select
                  className="select-field"
                  value={subjectId}
                  onChange={(event) => {
                    setSubjectId(event.target.value);
                    setExamForm(emptyExam);
                    setTopicForm(emptyTopic);
                  }}
                >
                  {subjects.map((subject) => (
                    <option key={subject.id} value={subject.id}>
                      {subject.name}
                    </option>
                  ))}
                </select>
              </label>
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

          <section className="academic-panel" aria-labelledby="exam-list-title">
            <div className="task-board-head">
              <h2 id="exam-list-title">Exámenes</h2>
              <span className="count-chip">{exams.length}</span>
            </div>
            {exams.length === 0 ? (
              <p className="empty-state">Todavía no hay exámenes.</p>
            ) : (
              <ul className="mini-list">
                {exams.map((exam) => (
                  <li key={exam.id} className="mini-item exam-item">
                    <div>
                      <strong>{exam.title}</strong>
                      <span className="quiet">
                        {" "}
                        · {exam.date}
                        {exam.startTime ? ` ${exam.startTime}` : ""}
                        {exam.endTime ? `–${exam.endTime}` : ""}
                        {exam.weightBp !== null
                          ? ` · peso ${(exam.weightBp / 100).toFixed(1)} %`
                          : ""}
                        {exam.resultStatus !== "pending"
                          ? ` · ${statusLabels[exam.resultStatus]}`
                          : ""}
                        {exam.resultScoreCp !== null &&
                        exam.resultMaxCp !== null
                          ? ` · ${cpToValue(exam.resultScoreCp)} / ${cpToValue(exam.resultMaxCp)}`
                          : ""}
                      </span>
                      {topics.length > 0 ? (
                        <div className="tag-list">
                          {topics.map((topic) => (
                            <label key={topic.id} className="tag-chip">
                              <input
                                type="checkbox"
                                checked={(links[exam.id] ?? []).includes(
                                  topic.id,
                                )}
                                onChange={(event) =>
                                  void toggleLink(
                                    exam.id,
                                    topic.id,
                                    event.target.checked,
                                  )
                                }
                              />
                              <span>{topic.name}</span>
                            </label>
                          ))}
                        </div>
                      ) : null}
                    </div>
                    <div className="task-board-head">
                      <button
                        type="button"
                        className="mini-action"
                        onClick={() => setExamForm(formFromExam(exam))}
                      >
                        Editar
                      </button>
                      <button
                        type="button"
                        className="mini-action danger"
                        aria-label={`Eliminar examen ${exam.title}`}
                        onClick={() => void handleDeleteExam(exam)}
                      >
                        ×
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className="academic-panel" aria-labelledby="exam-form-title">
            <div className="task-board-head">
              <h2 id="exam-form-title">
                {examForm.id ? "Editar examen" : "Nuevo examen"}
              </h2>
            </div>
            <form onSubmit={submitExam}>
              <div className="inline-form">
                <label className="field">
                  <span>Título</span>
                  <input
                    className="text-field"
                    value={examForm.title}
                    onChange={(event) =>
                      setExamForm({ ...examForm, title: event.target.value })
                    }
                    required
                  />
                </label>
                <label className="field">
                  <span>Fecha</span>
                  <input
                    className="text-field"
                    type="date"
                    value={examForm.date}
                    onChange={(event) =>
                      setExamForm({ ...examForm, date: event.target.value })
                    }
                    required
                  />
                </label>
                <label className="field narrow">
                  <span>Inicio</span>
                  <input
                    className="text-field"
                    type="time"
                    value={examForm.startTime}
                    onChange={(event) =>
                      setExamForm({
                        ...examForm,
                        startTime: event.target.value,
                      })
                    }
                  />
                </label>
                <label className="field narrow">
                  <span>Fin</span>
                  <input
                    className="text-field"
                    type="time"
                    value={examForm.endTime}
                    onChange={(event) =>
                      setExamForm({ ...examForm, endTime: event.target.value })
                    }
                  />
                </label>
                <label className="field">
                  <span>Aula</span>
                  <input
                    className="text-field"
                    value={examForm.room}
                    onChange={(event) =>
                      setExamForm({ ...examForm, room: event.target.value })
                    }
                  />
                </label>
                <label className="field narrow">
                  <span>Peso (%)</span>
                  <input
                    className="text-field"
                    type="number"
                    step="0.01"
                    value={examForm.weight}
                    onChange={(event) =>
                      setExamForm({ ...examForm, weight: event.target.value })
                    }
                  />
                </label>
              </div>
              <div className="inline-form">
                <label className="field narrow">
                  <span>Nota</span>
                  <input
                    className="text-field"
                    type="number"
                    step="0.01"
                    value={examForm.resultScore}
                    onChange={(event) =>
                      setExamForm({
                        ...examForm,
                        resultScore: event.target.value,
                      })
                    }
                  />
                </label>
                <label className="field narrow">
                  <span>Máximo</span>
                  <input
                    className="text-field"
                    type="number"
                    step="0.01"
                    value={examForm.resultMax}
                    onChange={(event) =>
                      setExamForm({
                        ...examForm,
                        resultMax: event.target.value,
                      })
                    }
                  />
                </label>
                <label className="field">
                  <span>Resultado</span>
                  <select
                    className="select-field"
                    value={examForm.resultStatus}
                    onChange={(event) =>
                      setExamForm({
                        ...examForm,
                        resultStatus: event.target.value as ExamResultStatus,
                      })
                    }
                  >
                    {Object.entries(statusLabels).map(([value, label]) => (
                      <option key={value} value={value}>
                        {label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  <span>Notas</span>
                  <input
                    className="text-field"
                    value={examForm.notes}
                    onChange={(event) =>
                      setExamForm({ ...examForm, notes: event.target.value })
                    }
                  />
                </label>
              </div>
              <div className="form-actions">
                {examForm.id ? (
                  <button
                    type="button"
                    className="button"
                    onClick={() => setExamForm(emptyExam)}
                  >
                    Cancelar
                  </button>
                ) : null}
                <button
                  type="submit"
                  className="button filled"
                  disabled={
                    examForm.title.trim().length === 0 || !examForm.date
                  }
                >
                  {examForm.id ? "Guardar cambios" : "Crear examen"}
                </button>
              </div>
            </form>
          </section>

          <section className="academic-panel" aria-labelledby="topics-title">
            <div className="task-board-head">
              <h2 id="topics-title">Temas</h2>
              <span className="count-chip">{topics.length}</span>
            </div>
            <ul className="mini-list">
              {topics.map((topic) => (
                <li key={topic.id} className="mini-item">
                  <span>
                    {topic.name}
                    {topic.parentTopicId ? (
                      <span className="quiet">
                        {" "}
                        · subtema de{" "}
                        {topics.find((item) => item.id === topic.parentTopicId)
                          ?.name ?? "otro tema"}
                      </span>
                    ) : null}
                  </span>
                  <button
                    type="button"
                    className="mini-action danger"
                    aria-label={`Eliminar tema ${topic.name}`}
                    onClick={() => void handleDeleteTopic(topic)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
            <form className="inline-form" onSubmit={submitTopic}>
              <label className="field">
                <span>Nombre</span>
                <input
                  className="text-field"
                  value={topicForm.name}
                  onChange={(event) =>
                    setTopicForm({ ...topicForm, name: event.target.value })
                  }
                  required
                />
              </label>
              <label className="field">
                <span>Padre</span>
                <select
                  className="select-field"
                  value={topicForm.parentTopicId}
                  onChange={(event) =>
                    setTopicForm({
                      ...topicForm,
                      parentTopicId: event.target.value,
                    })
                  }
                >
                  <option value="">Sin padre</option>
                  {topics.map((topic) => (
                    <option key={topic.id} value={topic.id}>
                      {topic.name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>Descripción</span>
                <input
                  className="text-field"
                  value={topicForm.description}
                  onChange={(event) =>
                    setTopicForm({
                      ...topicForm,
                      description: event.target.value,
                    })
                  }
                />
              </label>
              <button
                type="submit"
                className="button tonal"
                disabled={topicForm.name.trim().length === 0}
              >
                Crear tema
              </button>
            </form>
          </section>
        </>
      )}
    </>
  );
}
