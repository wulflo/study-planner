import { type FormEvent, useEffect, useState } from "react";
import {
  createGradeCategory,
  createGradeEntry,
  createGradeScale,
  deleteGradeCategory,
  deleteGradeEntry,
  deleteGradeScale,
  type GradeCategory,
  type GradeEntry,
  type GradeScale,
  getSubjectGrade,
  listAcademicYears,
  listGradeCategories,
  listGradeEntries,
  listGradeScales,
  listSubjects,
  listTerms,
  type Subject,
  type SubjectGrade,
  type VaultStatus,
} from "../lib/ipc";

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

function cpToValue(value: number | null, precision: number): string {
  if (value === null) {
    return "—";
  }
  return (value / 100).toFixed(precision);
}

function bpToPercent(value: number | null): string {
  if (value === null) {
    return "—";
  }
  return `${(value / 100).toFixed(1)} %`;
}

const emptyScale = {
  name: "Escala 0–100",
  min: "0",
  max: "100",
  pass: "60",
  precision: "2",
  isDefault: true,
};

const emptyCategory = {
  scaleId: "",
  name: "",
  weight: "50",
  dropLowest: "0",
};

const emptyEntry = {
  categoryId: "",
  title: "",
  score: "",
  max: "100",
  weight: "",
  date: "",
};

export function GradesView({
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
  const [scales, setScales] = useState<GradeScale[]>([]);
  const [categories, setCategories] = useState<GradeCategory[]>([]);
  const [entries, setEntries] = useState<GradeEntry[]>([]);
  const [grade, setGrade] = useState<SubjectGrade | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [scaleForm, setScaleForm] = useState(emptyScale);
  const [categoryForm, setCategoryForm] = useState(emptyCategory);
  const [entryForm, setEntryForm] = useState(emptyEntry);

  useEffect(() => {
    if (!ready) {
      return;
    }
    let cancelled = false;

    async function loadSubjects() {
      setLoading(true);
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
      } finally {
        if (!cancelled) {
          setLoading(false);
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
      setLoading(true);
      setError(null);
      try {
        const [loadedScales, loadedCategories, loadedEntries, loadedGrade] =
          await Promise.all([
            listGradeScales(subjectId),
            listGradeCategories(subjectId),
            listGradeEntries(subjectId),
            getSubjectGrade(subjectId),
          ]);
        if (!cancelled) {
          setScales(loadedScales);
          setCategories(loadedCategories);
          setEntries(loadedEntries);
          setGrade(loadedGrade);
          const defaultScale =
            loadedScales.find((scale) => scale.isDefault) ?? loadedScales[0];
          setCategoryForm((current) => ({
            ...current,
            scaleId: current.scaleId || (defaultScale?.id ?? ""),
          }));
          setEntryForm((current) => ({
            ...current,
            categoryId: current.categoryId || (loadedCategories[0]?.id ?? ""),
          }));
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

    void loadData();
    return () => {
      cancelled = true;
    };
  }, [ready, subjectId]);

  async function refresh() {
    if (!subjectId) {
      return;
    }
    const [loadedScales, loadedCategories, loadedEntries, loadedGrade] =
      await Promise.all([
        listGradeScales(subjectId),
        listGradeCategories(subjectId),
        listGradeEntries(subjectId),
        getSubjectGrade(subjectId),
      ]);
    setScales(loadedScales);
    setCategories(loadedCategories);
    setEntries(loadedEntries);
    setGrade(loadedGrade);
  }

  async function handleCreateScale(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!subjectId) {
      return;
    }
    setError(null);
    try {
      await createGradeScale({
        subjectId,
        name: scaleForm.name,
        minCp: Math.round((optionalNumber(scaleForm.min) ?? 0) * 100),
        maxCp: Math.round((optionalNumber(scaleForm.max) ?? 100) * 100),
        passCp: Math.round((optionalNumber(scaleForm.pass) ?? 60) * 100),
        precision: Math.round(optionalNumber(scaleForm.precision) ?? 2),
        isDefault: scaleForm.isDefault,
      });
      setNotice("Escala creada.");
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleCreateDefaultScale() {
    if (!subjectId) {
      return;
    }
    setError(null);
    try {
      await createGradeScale({
        subjectId,
        name: "Escala 0–100",
        minCp: 0,
        maxCp: 10_000,
        passCp: 6_000,
        precision: 2,
        isDefault: true,
      });
      setNotice("Escala 0–100 creada y marcada como predeterminada.");
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleCreateCategory(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!subjectId) {
      return;
    }
    setError(null);
    try {
      await createGradeCategory(subjectId, {
        scaleId: categoryForm.scaleId,
        name: categoryForm.name,
        weightBp: Math.round((optionalNumber(categoryForm.weight) ?? 0) * 100),
        dropLowest: Math.round(optionalNumber(categoryForm.dropLowest) ?? 0),
      });
      setCategoryForm((current) => ({ ...current, name: "" }));
      setNotice("Categoría creada.");
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleCreateEntry(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!subjectId) {
      return;
    }
    setError(null);
    try {
      await createGradeEntry(subjectId, {
        categoryId: entryForm.categoryId,
        examId: null,
        title: entryForm.title,
        scoreCp: Math.round((optionalNumber(entryForm.score) ?? 0) * 100),
        maxCp: Math.round((optionalNumber(entryForm.max) ?? 100) * 100),
        weightBp:
          optionalNumber(entryForm.weight) === null
            ? null
            : Math.round((optionalNumber(entryForm.weight) ?? 0) * 100),
        date: optionalText(entryForm.date),
      });
      setEntryForm((current) => ({ ...current, title: "", score: "" }));
      setNotice("Nota registrada.");
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleDeleteScale(scale: GradeScale) {
    setError(null);
    try {
      await deleteGradeScale(scale.id);
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleDeleteCategory(category: GradeCategory) {
    setError(null);
    try {
      await deleteGradeCategory(category.id);
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  async function handleDeleteEntry(entry: GradeEntry) {
    setError(null);
    try {
      await deleteGradeEntry(entry.id);
      await refresh();
    } catch (reason) {
      setError(messageOf(reason));
    }
  }

  const subject = subjects.find((item) => item.id === subjectId) ?? null;
  const activeScale =
    scales.find((scale) => scale.id === grade?.scaleId) ??
    scales.find((scale) => scale.isDefault) ??
    scales[0] ??
    null;
  const precision = activeScale?.precision ?? 2;

  return (
    <>
      <section className="hero section-hero">
        <p className="eyebrow">Notas · punto fijo</p>
        <h1>Notas</h1>
        <p className="hero-copy">
          Escalas, categorías y entradas con promedio ponderado y descarte del
          menor. Los cálculos usan enteros: centipuntos y basis points.
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
            Las notas se guardan en <b>omega</b>. Elegí una carpeta en la
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
          <p>
            Las escalas, categorías y notas se asocian a una materia de tu
            estructura académica.
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
            <div className="inline-form">
              <label className="field">
                <span>Materia</span>
                <select
                  className="select-field"
                  value={subjectId}
                  onChange={(event) => {
                    setSubjectId(event.target.value);
                    setCategoryForm((current) => ({
                      ...current,
                      scaleId: "",
                    }));
                    setEntryForm((current) => ({ ...current, categoryId: "" }));
                  }}
                >
                  {subjects.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </label>
              <div className="grade-summary">
                <span className="eyebrow">Promedio final</span>
                <strong>
                  {grade
                    ? cpToValue(grade.finalCp, precision)
                    : loading
                      ? "…"
                      : "—"}
                </strong>
                <span className="quiet">
                  {grade?.finalBp !== null && grade?.finalBp !== undefined
                    ? `${(grade.finalBp / 100).toFixed(1)} %`
                    : "Sin notas"}
                  {grade?.passed === true
                    ? " · aprobada"
                    : grade?.passed === false
                      ? " · no aprobada"
                      : ""}
                </span>
              </div>
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

          {grade && grade.categories.length > 0 ? (
            <section className="status-card" aria-labelledby="breakdown-title">
              <div>
                <p className="eyebrow">Desglose</p>
                <h2 id="breakdown-title">{subject?.name ?? "Materia"}</h2>
                <p>
                  Cada categoría aporta según su peso. El descarte del menor es
                  determinista.
                </p>
              </div>
              <dl className="status-grid">
                {grade.categories.map((category) => (
                  <div key={category.categoryId}>
                    <dt>
                      {category.categoryName} · {bpToPercent(category.weightBp)}
                    </dt>
                    <dd>
                      {category.percentBp === null
                        ? "Sin notas"
                        : `${(category.percentBp / 100).toFixed(1)} %`}
                      {category.dropped > 0
                        ? ` · descarta ${category.dropped}`
                        : ""}
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ) : null}

          <section className="academic-panel" aria-labelledby="scales-title">
            <div className="task-board-head">
              <h2 id="scales-title">Escalas</h2>
              <span className="count-chip">{scales.length}</span>
            </div>
            {scales.length === 0 ? (
              <div className="hero-actions">
                <button
                  type="button"
                  className="button tonal"
                  onClick={() => void handleCreateDefaultScale()}
                >
                  Crear escala 0–100
                </button>
              </div>
            ) : (
              <ul className="mini-list">
                {scales.map((scale) => (
                  <li key={scale.id} className="mini-item">
                    <span>
                      {scale.name}
                      <span className="quiet">
                        {" "}
                        · {cpToValue(scale.minCp, scale.precision)} a{" "}
                        {cpToValue(scale.maxCp, scale.precision)} · aprueba{" "}
                        {cpToValue(scale.passCp, scale.precision)}
                        {scale.isDefault ? " · predeterminada" : ""}
                      </span>
                    </span>
                    <button
                      type="button"
                      className="mini-action danger"
                      aria-label={`Eliminar escala ${scale.name}`}
                      onClick={() => void handleDeleteScale(scale)}
                    >
                      ×
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <form className="inline-form" onSubmit={handleCreateScale}>
              <label className="field">
                <span>Nombre</span>
                <input
                  className="text-field"
                  value={scaleForm.name}
                  onChange={(event) =>
                    setScaleForm({ ...scaleForm, name: event.target.value })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Mínimo</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={scaleForm.min}
                  onChange={(event) =>
                    setScaleForm({ ...scaleForm, min: event.target.value })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Máximo</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={scaleForm.max}
                  onChange={(event) =>
                    setScaleForm({ ...scaleForm, max: event.target.value })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Aprueba</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={scaleForm.pass}
                  onChange={(event) =>
                    setScaleForm({ ...scaleForm, pass: event.target.value })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Decimales</span>
                <input
                  className="text-field"
                  type="number"
                  min={0}
                  max={4}
                  value={scaleForm.precision}
                  onChange={(event) =>
                    setScaleForm({
                      ...scaleForm,
                      precision: event.target.value,
                    })
                  }
                />
              </label>
              <label className="field">
                <span>Predeterminada</span>
                <input
                  type="checkbox"
                  checked={scaleForm.isDefault}
                  onChange={(event) =>
                    setScaleForm({
                      ...scaleForm,
                      isDefault: event.target.checked,
                    })
                  }
                />
              </label>
              <button type="submit" className="button tonal">
                Crear escala
              </button>
            </form>
          </section>

          <section
            className="academic-panel"
            aria-labelledby="categories-title"
          >
            <div className="task-board-head">
              <h2 id="categories-title">Categorías</h2>
              <span className="count-chip">{categories.length}</span>
            </div>
            <ul className="mini-list">
              {categories.map((category) => (
                <li key={category.id} className="mini-item">
                  <span>
                    {category.name}
                    <span className="quiet">
                      {" "}
                      · {bpToPercent(category.weightBp)}
                      {category.dropLowest > 0
                        ? ` · descarta ${category.dropLowest}`
                        : ""}
                    </span>
                  </span>
                  <button
                    type="button"
                    className="mini-action danger"
                    aria-label={`Eliminar categoría ${category.name}`}
                    onClick={() => void handleDeleteCategory(category)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
            <form className="inline-form" onSubmit={handleCreateCategory}>
              <label className="field">
                <span>Nombre</span>
                <input
                  className="text-field"
                  value={categoryForm.name}
                  onChange={(event) =>
                    setCategoryForm({
                      ...categoryForm,
                      name: event.target.value,
                    })
                  }
                  placeholder="Parciales, trabajos…"
                />
              </label>
              <label className="field">
                <span>Escala</span>
                <select
                  className="select-field"
                  value={categoryForm.scaleId}
                  onChange={(event) =>
                    setCategoryForm({
                      ...categoryForm,
                      scaleId: event.target.value,
                    })
                  }
                >
                  <option value="">Elegí una escala</option>
                  {scales.map((scale) => (
                    <option key={scale.id} value={scale.id}>
                      {scale.name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field narrow">
                <span>Peso (%)</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={categoryForm.weight}
                  onChange={(event) =>
                    setCategoryForm({
                      ...categoryForm,
                      weight: event.target.value,
                    })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Descarta</span>
                <input
                  className="text-field"
                  type="number"
                  min={0}
                  value={categoryForm.dropLowest}
                  onChange={(event) =>
                    setCategoryForm({
                      ...categoryForm,
                      dropLowest: event.target.value,
                    })
                  }
                />
              </label>
              <button
                type="submit"
                className="button tonal"
                disabled={
                  !categoryForm.scaleId || categoryForm.name.trim().length === 0
                }
              >
                Crear categoría
              </button>
            </form>
          </section>

          <section className="academic-panel" aria-labelledby="entries-title">
            <div className="task-board-head">
              <h2 id="entries-title">Entradas de nota</h2>
              <span className="count-chip">{entries.length}</span>
            </div>
            <ul className="mini-list">
              {entries.map((entry) => {
                const category = categories.find(
                  (item) => item.id === entry.categoryId,
                );
                return (
                  <li key={entry.id} className="mini-item">
                    <span>
                      {entry.title}
                      <span className="quiet">
                        {" "}
                        · {category?.name ?? "Sin categoría"} ·{" "}
                        {cpToValue(entry.scoreCp, precision)} /{" "}
                        {cpToValue(entry.maxCp, precision)}
                        {entry.date ? ` · ${entry.date}` : ""}
                      </span>
                    </span>
                    <button
                      type="button"
                      className="mini-action danger"
                      aria-label={`Eliminar nota ${entry.title}`}
                      onClick={() => void handleDeleteEntry(entry)}
                    >
                      ×
                    </button>
                  </li>
                );
              })}
            </ul>
            <form className="inline-form" onSubmit={handleCreateEntry}>
              <label className="field">
                <span>Título</span>
                <input
                  className="text-field"
                  value={entryForm.title}
                  onChange={(event) =>
                    setEntryForm({ ...entryForm, title: event.target.value })
                  }
                  placeholder="Parcial 1"
                />
              </label>
              <label className="field">
                <span>Categoría</span>
                <select
                  className="select-field"
                  value={entryForm.categoryId}
                  onChange={(event) =>
                    setEntryForm({
                      ...entryForm,
                      categoryId: event.target.value,
                    })
                  }
                >
                  <option value="">Elegí una categoría</option>
                  {categories.map((category) => (
                    <option key={category.id} value={category.id}>
                      {category.name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field narrow">
                <span>Nota</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={entryForm.score}
                  onChange={(event) =>
                    setEntryForm({ ...entryForm, score: event.target.value })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Máximo</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={entryForm.max}
                  onChange={(event) =>
                    setEntryForm({ ...entryForm, max: event.target.value })
                  }
                />
              </label>
              <label className="field narrow">
                <span>Peso (%)</span>
                <input
                  className="text-field"
                  type="number"
                  step="0.01"
                  value={entryForm.weight}
                  onChange={(event) =>
                    setEntryForm({ ...entryForm, weight: event.target.value })
                  }
                  placeholder="opcional"
                />
              </label>
              <label className="field">
                <span>Fecha</span>
                <input
                  className="text-field"
                  type="date"
                  value={entryForm.date}
                  onChange={(event) =>
                    setEntryForm({ ...entryForm, date: event.target.value })
                  }
                />
              </label>
              <button
                type="submit"
                className="button tonal"
                disabled={
                  !entryForm.categoryId ||
                  entryForm.title.trim().length === 0 ||
                  entryForm.score.trim().length === 0
                }
              >
                Registrar nota
              </button>
            </form>
          </section>
        </>
      )}
    </>
  );
}
