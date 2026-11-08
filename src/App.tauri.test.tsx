import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const api = vi.hoisted(() => ({
  getAppInfo: vi.fn(),
  runSchemaSelfCheck: vi.fn(),
  getVaultStatus: vi.fn(),
  chooseVault: vi.fn(),
  createBackup: vi.fn(),
  exportVault: vi.fn(),
  importVault: vi.fn(),
  restoreMainBackup: vi.fn(),
  listTasks: vi.fn(),
  createTask: vi.fn(),
  setTaskDone: vi.fn(),
  deleteTask: vi.fn(),
  createTaskFull: vi.fn(),
  updateTask: vi.fn(),
  setTaskStatus: vi.fn(),
  searchTasks: vi.fn(),
  listTags: vi.fn(),
  createTag: vi.fn(),
  updateTag: vi.fn(),
  deleteTag: vi.fn(),
  setTaskTags: vi.fn(),
  listSavedFilters: vi.fn(),
  createSavedFilter: vi.fn(),
  updateSavedFilter: vi.fn(),
  deleteSavedFilter: vi.fn(),
  listUndoEntries: vi.fn(),
  undoEntry: vi.fn(),
  purgeExpiredUndo: vi.fn(),
  listExams: vi.fn(),
  createExam: vi.fn(),
  updateExam: vi.fn(),
  deleteExam: vi.fn(),
  listTopics: vi.fn(),
  createTopic: vi.fn(),
  updateTopic: vi.fn(),
  deleteTopic: vi.fn(),
  listExamTopics: vi.fn(),
  linkExamTopic: vi.fn(),
  unlinkExamTopic: vi.fn(),
  listGradeScales: vi.fn(),
  createGradeScale: vi.fn(),
  updateGradeScale: vi.fn(),
  deleteGradeScale: vi.fn(),
  listGradeCategories: vi.fn(),
  createGradeCategory: vi.fn(),
  updateGradeCategory: vi.fn(),
  deleteGradeCategory: vi.fn(),
  listGradeEntries: vi.fn(),
  createGradeEntry: vi.fn(),
  updateGradeEntry: vi.fn(),
  deleteGradeEntry: vi.fn(),
  getSubjectGrade: vi.fn(),
  listAcademicYears: vi.fn(),
  createAcademicYear: vi.fn(),
  updateAcademicYear: vi.fn(),
  deleteAcademicYear: vi.fn(),
  listTerms: vi.fn(),
  createTerm: vi.fn(),
  updateTerm: vi.fn(),
  deleteTerm: vi.fn(),
  listSubjects: vi.fn(),
  createSubject: vi.fn(),
  updateSubject: vi.fn(),
  deleteSubject: vi.fn(),
  listClassSlots: vi.fn(),
  createClassSlot: vi.fn(),
  updateClassSlot: vi.fn(),
  deleteClassSlot: vi.fn(),
  getRotation: vi.fn(),
  saveRotation: vi.fn(),
  deleteRotation: vi.fn(),
  listHolidays: vi.fn(),
  createHoliday: vi.fn(),
  deleteHoliday: vi.fn(),
  listClassExceptions: vi.fn(),
  createClassException: vi.fn(),
  deleteClassException: vi.fn(),
  listAttendance: vi.fn(),
  setAttendance: vi.fn(),
  clearAttendance: vi.fn(),
  getWeekSchedule: vi.fn(),
  getNextClass: vi.fn(),
}));

vi.mock("./lib/ipc", () => ({
  runningInTauri: true,
  ...api,
}));

vi.mock("./components/WindowControls", () => ({
  WindowControls: () => null,
}));

vi.mock("./components/ResizeHandles", () => ({
  ResizeHandles: () => null,
}));

const vaultReady = {
  configured: true,
  ready: true,
  name: "Bóveda de prueba",
  device: "equipo",
  schemaVersion: 1,
  integrity: "ok",
  foreignKeyViolations: 0,
  vaultId: "vault-1",
  hasData: true,
  backup: {
    fileName: "study-planner.db",
    sha256: "a".repeat(64),
    sizeBytes: 4096,
    schemaVersion: 1,
    integrity: "ok",
    foreignKeyViolations: 0,
    modifiedAtMillis: 1762600000000,
  },
  snapshots: [
    {
      date: "2026-11-08",
      fileName: "study-planner.db",
      sha256: "b".repeat(64),
      sizeBytes: 4096,
      schemaVersion: 1,
      integrity: "ok",
      foreignKeyViolations: 0,
      createdAt: "2026-11-08T10:00:00.000Z",
    },
  ],
  message: null,
};

const vaultMissing = {
  configured: false,
  ready: false,
  name: null,
  device: null,
  schemaVersion: null,
  integrity: null,
  foreignKeyViolations: null,
  vaultId: null,
  hasData: false,
  backup: null,
  snapshots: [],
  message: null,
};

const vaultEmpty = {
  ...vaultReady,
  name: "Bóveda vacía",
  vaultId: "vault-empty",
  hasData: false,
  backup: null,
  snapshots: [],
};

const pendingTask = {
  id: "t1",
  subjectId: null,
  parentTaskId: null,
  taskType: "task",
  title: "Repasar matrices",
  description: null,
  startDate: null,
  dueDate: null,
  dueTime: null,
  priority: "medium",
  important: false,
  estimateMinutes: null,
  status: "pending",
  completedAt: null,
  recurrence: null,
  tags: [],
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const doneTask = {
  ...pendingTask,
  id: "t2",
  title: "Entregar práctico",
  status: "done",
  completedAt: "2026-11-08T09:00:00.000Z",
  createdAt: "2026-11-07T10:00:00.000Z",
  updatedAt: "2026-11-08T09:00:00.000Z",
};

function todayIso(): string {
  const date = new Date();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

const academicYear = {
  id: "year-1",
  name: "2026",
  startDate: "2026-03-01",
  endDate: "2026-12-20",
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const academicTerm = {
  id: "term-1",
  yearId: "year-1",
  name: "Primer semestre",
  kind: "semester",
  startDate: "2026-03-02",
  endDate: "2026-07-10",
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const algebra = {
  id: "subject-1",
  termId: "term-1",
  name: "Álgebra lineal",
  code: "MAT-101",
  color: "#6750A4",
  teacher: "Docente",
  room: "Aula 3",
  credits: 6,
  goalScoreCp: null,
  attendanceMinBp: 7500,
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const examTag = {
  id: "tag-1",
  name: "examen",
  color: null,
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const gradeScale = {
  id: "scale-1",
  subjectId: "subject-1",
  name: "Escala 0–100",
  minCp: 0,
  maxCp: 10000,
  passCp: 6000,
  precision: 2,
  isDefault: true,
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const gradeCategories = [
  {
    id: "cat-1",
    subjectId: "subject-1",
    scaleId: "scale-1",
    name: "Parciales",
    weightBp: 6000,
    dropLowest: 1,
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  },
  {
    id: "cat-2",
    subjectId: "subject-1",
    scaleId: "scale-1",
    name: "Trabajos",
    weightBp: 4000,
    dropLowest: 0,
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  },
];

const gradeEntries = [
  {
    id: "entry-1",
    subjectId: "subject-1",
    categoryId: "cat-1",
    examId: null,
    title: "Parcial 1",
    scoreCp: 9000,
    maxCp: 10000,
    weightBp: null,
    date: "2026-04-01",
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  },
  {
    id: "entry-2",
    subjectId: "subject-1",
    categoryId: "cat-1",
    examId: null,
    title: "Parcial 2",
    scoreCp: 10000,
    maxCp: 10000,
    weightBp: null,
    date: "2026-05-01",
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  },
  {
    id: "entry-3",
    subjectId: "subject-1",
    categoryId: "cat-2",
    examId: null,
    title: "Trabajo 1",
    scoreCp: 8000,
    maxCp: 10000,
    weightBp: null,
    date: "2026-05-15",
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  },
];

const subjectGrade = {
  subjectId: "subject-1",
  scaleId: "scale-1",
  finalBp: 9200,
  finalCp: 9200,
  passed: true,
  categories: [
    {
      categoryId: "cat-1",
      categoryName: "Parciales",
      weightBp: 6000,
      entries: 2,
      dropped: 1,
      percentBp: 10000,
      averageCp: 10000,
    },
    {
      categoryId: "cat-2",
      categoryName: "Trabajos",
      weightBp: 4000,
      entries: 1,
      dropped: 0,
      percentBp: 8000,
      averageCp: 8000,
    },
  ],
};

const classSlot = {
  id: "slot-1",
  subjectId: "subject-1",
  weekday: 1,
  startTime: "08:00",
  endTime: "10:00",
  recurrence: "weekly",
  rotationKey: null,
  validFrom: null,
  validTo: null,
  roomOverride: null,
  createdAt: "2026-11-08T10:00:00.000Z",
  updatedAt: "2026-11-08T10:00:00.000Z",
};

const weekSchedule = {
  termId: "term-1",
  weekStart: "2026-03-02",
  weekEnd: "2026-03-08",
  rotationLabel: "A",
  holidays: [],
  occurrences: [
    {
      date: "2026-03-02",
      weekday: 1,
      slotId: "slot-1",
      subjectId: "subject-1",
      subjectName: "Álgebra lineal",
      subjectColor: "#6750A4",
      startTime: "08:00",
      endTime: "10:00",
      room: "Aula 3",
      kind: "scheduled",
      attendanceStatus: null,
    },
  ],
  overlaps: [],
  attendance: [
    {
      subjectId: "subject-1",
      subjectName: "Álgebra lineal",
      subjectColor: "#6750A4",
      present: 0,
      absent: 0,
      late: 0,
      justified: 0,
      counted: 0,
      attended: 0,
      percentBp: null,
      minBp: 7500,
      belowMin: false,
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  api.getAppInfo.mockResolvedValue({
    name: "Study Planner",
    version: "0.1.0",
    schemaVersion: 1,
    exportFormatVersion: 1,
  });
  api.runSchemaSelfCheck.mockResolvedValue({
    schemaVersion: 1,
    integrity: "ok",
    foreignKeyViolations: 0,
    vaultId: "self-check",
  });
  api.getVaultStatus.mockResolvedValue(vaultReady);
  api.listTasks.mockResolvedValue([pendingTask, doneTask]);
  api.createTask.mockResolvedValue({
    ...pendingTask,
    id: "t3",
    title: "Nueva tarea",
  });
  api.setTaskDone.mockResolvedValue({
    ...pendingTask,
    status: "done",
    completedAt: "2026-11-08T11:00:00.000Z",
  });
  api.deleteTask.mockResolvedValue(undefined);
  api.createTaskFull.mockResolvedValue(pendingTask);
  api.updateTask.mockResolvedValue(pendingTask);
  api.setTaskStatus.mockResolvedValue(pendingTask);
  api.searchTasks.mockResolvedValue([pendingTask, doneTask]);
  api.listTags.mockResolvedValue([]);
  api.createTag.mockResolvedValue({
    id: "tag-1",
    name: "mate",
    color: null,
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.updateTag.mockResolvedValue({
    id: "tag-1",
    name: "mate",
    color: null,
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.deleteTag.mockResolvedValue(undefined);
  api.setTaskTags.mockResolvedValue(undefined);
  api.listSavedFilters.mockResolvedValue([]);
  api.createSavedFilter.mockResolvedValue({
    id: "filter-1",
    screen: "tasks",
    name: "Urgentes",
    queryJson: "{}",
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.updateSavedFilter.mockResolvedValue({
    id: "filter-1",
    screen: "tasks",
    name: "Urgentes",
    queryJson: "{}",
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.deleteSavedFilter.mockResolvedValue(undefined);
  api.listUndoEntries.mockResolvedValue([]);
  api.undoEntry.mockResolvedValue(pendingTask);
  api.purgeExpiredUndo.mockResolvedValue(0);
  api.listExams.mockResolvedValue([]);
  api.createExam.mockResolvedValue(null);
  api.updateExam.mockResolvedValue(null);
  api.deleteExam.mockResolvedValue(undefined);
  api.listTopics.mockResolvedValue([]);
  api.createTopic.mockResolvedValue(null);
  api.updateTopic.mockResolvedValue(null);
  api.deleteTopic.mockResolvedValue(undefined);
  api.listExamTopics.mockResolvedValue([]);
  api.linkExamTopic.mockResolvedValue(null);
  api.unlinkExamTopic.mockResolvedValue(undefined);
  api.listGradeScales.mockResolvedValue([]);
  api.createGradeScale.mockResolvedValue(null);
  api.updateGradeScale.mockResolvedValue(null);
  api.deleteGradeScale.mockResolvedValue(undefined);
  api.listGradeCategories.mockResolvedValue([]);
  api.createGradeCategory.mockResolvedValue(null);
  api.updateGradeCategory.mockResolvedValue(null);
  api.deleteGradeCategory.mockResolvedValue(undefined);
  api.listGradeEntries.mockResolvedValue([]);
  api.createGradeEntry.mockResolvedValue(null);
  api.updateGradeEntry.mockResolvedValue(null);
  api.deleteGradeEntry.mockResolvedValue(undefined);
  api.getSubjectGrade.mockResolvedValue({
    subjectId: "subject-1",
    scaleId: null,
    finalBp: null,
    finalCp: null,
    passed: null,
    categories: [],
  });
  api.chooseVault.mockResolvedValue(vaultReady);
  api.createBackup.mockResolvedValue(vaultReady);
  api.exportVault.mockResolvedValue({
    fileName: "boveda.txt",
    sha256: "c".repeat(64),
    sizeBytes: 1234,
    rows: 7,
    snapshotAt: "2026-11-08T10:00:00.000Z",
  });
  api.importVault.mockResolvedValue({
    status: vaultReady,
    fileName: "boveda.txt",
    rows: 7,
    attachments: 1,
    vaultId: "vault-1",
    snapshotAt: "2026-11-08T10:00:00.000Z",
  });
  api.restoreMainBackup.mockResolvedValue(vaultReady);
  api.listAcademicYears.mockResolvedValue([academicYear]);
  api.listTerms.mockResolvedValue([academicTerm]);
  api.listSubjects.mockResolvedValue([algebra]);
  api.createAcademicYear.mockResolvedValue(academicYear);
  api.updateAcademicYear.mockResolvedValue(academicYear);
  api.deleteAcademicYear.mockResolvedValue(undefined);
  api.createTerm.mockResolvedValue(academicTerm);
  api.updateTerm.mockResolvedValue(academicTerm);
  api.deleteTerm.mockResolvedValue(undefined);
  api.createSubject.mockResolvedValue(algebra);
  api.updateSubject.mockResolvedValue(algebra);
  api.deleteSubject.mockResolvedValue(undefined);
  api.listClassSlots.mockResolvedValue([]);
  api.createClassSlot.mockResolvedValue(classSlot);
  api.updateClassSlot.mockResolvedValue(classSlot);
  api.deleteClassSlot.mockResolvedValue(undefined);
  api.getRotation.mockResolvedValue(null);
  api.saveRotation.mockResolvedValue({
    id: "rotation-1",
    termId: "term-1",
    mode: "weekly_ab",
    anchorDate: "2026-03-02",
    cycleLength: 2,
    labels: ["A", "B"],
    dayCycle: [],
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.deleteRotation.mockResolvedValue(undefined);
  api.listHolidays.mockResolvedValue([]);
  api.createHoliday.mockResolvedValue({
    id: "holiday-1",
    termId: "term-1",
    date: "2026-04-02",
    name: "Feriado",
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.deleteHoliday.mockResolvedValue(undefined);
  api.listClassExceptions.mockResolvedValue([]);
  api.createClassException.mockResolvedValue({
    id: "exception-1",
    classSlotId: "slot-1",
    date: "2026-03-09",
    kind: "cancelled",
    newStartTime: null,
    newEndTime: null,
    newRoom: null,
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.deleteClassException.mockResolvedValue(undefined);
  api.listAttendance.mockResolvedValue([]);
  api.setAttendance.mockResolvedValue({
    id: "attendance-1",
    classSlotId: "slot-1",
    date: "2026-03-02",
    status: "present",
    note: null,
    createdAt: "2026-11-08T10:00:00.000Z",
    updatedAt: "2026-11-08T10:00:00.000Z",
  });
  api.clearAttendance.mockResolvedValue(undefined);
  api.getWeekSchedule.mockResolvedValue(weekSchedule);
  api.getNextClass.mockResolvedValue(null);
});

afterEach(cleanup);

describe("App con IPC de Tauri", () => {
  it("carga la bóveda y muestra los pendientes al abrir", async () => {
    render(<App />);

    expect(
      await screen.findByText(/bóveda local · bóveda de prueba/i),
    ).toBeInTheDocument();
    expect(
      await screen.findByRole("checkbox", {
        name: "Completar «Repasar matrices»",
      }),
    ).toBeInTheDocument();
    expect(api.getVaultStatus).toHaveBeenCalled();
    expect(api.listTasks).toHaveBeenCalled();
  });

  it("crea una tarea desde la sección Tareas", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Tareas" }));
    const input = await screen.findByLabelText("Nueva tarea");
    await user.type(input, "Preparar final");

    const created = {
      ...pendingTask,
      id: "t3",
      title: "Preparar final",
    };
    api.listTasks.mockResolvedValue([created, pendingTask, doneTask]);

    await user.click(screen.getByRole("button", { name: "Agregar" }));

    await waitFor(() =>
      expect(api.createTask).toHaveBeenCalledWith("Preparar final"),
    );
    expect(await screen.findByText("Preparar final")).toBeInTheDocument();
    expect(input).toHaveValue("");
  });

  it("marca una tarea como hecha y la mueve a Hechas", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Tareas" }));
    const checkbox = await screen.findByRole("checkbox", {
      name: "Completar «Repasar matrices»",
    });

    const completed = {
      ...pendingTask,
      status: "done",
      completedAt: "2026-11-08T11:00:00.000Z",
    };
    api.listTasks.mockResolvedValue([completed, doneTask]);

    await user.click(checkbox);

    await waitFor(() =>
      expect(api.setTaskDone).toHaveBeenCalledWith("t1", true),
    );
    expect(
      await screen.findByRole("checkbox", {
        name: "Reabrir «Repasar matrices»",
      }),
    ).toBeChecked();
    expect(screen.getByRole("heading", { name: "Hechas" })).toBeInTheDocument();
  });

  it("elimina una tarea", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Tareas" }));
    const remove = await screen.findByRole("button", {
      name: "Eliminar «Repasar matrices»",
    });
    api.listTasks.mockResolvedValue([doneTask]);

    await user.click(remove);

    await waitFor(() => expect(api.deleteTask).toHaveBeenCalledWith("t1"));
    await waitFor(() =>
      expect(screen.queryByText("Repasar matrices")).not.toBeInTheDocument(),
    );
  });

  it("abre el selector y muestra la bóveda elegida", async () => {
    const user = userEvent.setup();
    api.getVaultStatus.mockResolvedValue(vaultMissing);
    api.listTasks.mockResolvedValue([]);
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Configurar bóveda" }));
    const choose = await screen.findByRole("button", {
      name: "Elegir carpeta",
    });
    expect(choose).toBeEnabled();

    await user.click(choose);

    await waitFor(() => expect(api.chooseVault).toHaveBeenCalled());
    expect(await screen.findByText("Bóveda de prueba")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Cambiar carpeta" }),
    ).toBeInTheDocument();
  });

  it("crea un backup desde la vista Bóveda", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Bóveda" }));

    expect(await screen.findByText("Backup principal")).toBeInTheDocument();
    expect(screen.getByText(/2026-11-08/)).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Crear backup ahora" }),
    );

    await waitFor(() => expect(api.createBackup).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText(/backup creado y verificado/i),
    ).toBeInTheDocument();
  });

  it("exporta la bóveda desde la vista Bóveda", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Bóveda" }));
    await user.click(
      await screen.findByRole("button", { name: "Exportar bóveda" }),
    );

    await waitFor(() => expect(api.exportVault).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText(/exportación lista: boveda\.txt/i),
    ).toBeInTheDocument();
  });

  it("solo permite importar en una bóveda vacía", async () => {
    const user = userEvent.setup();
    api.getVaultStatus.mockResolvedValue(vaultEmpty);
    api.listTasks.mockResolvedValue([]);
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Bóveda" }));
    const importButton = await screen.findByRole("button", {
      name: "Importar en bóveda vacía",
    });
    expect(importButton).toBeEnabled();

    await user.click(importButton);

    await waitFor(() => expect(api.importVault).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText(/importación completada desde boveda\.txt/i),
    ).toBeInTheDocument();
  });

  it("bloquea la importación cuando la bóveda ya tiene datos", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Bóveda" }));

    expect(
      await screen.findByRole("button", {
        name: "Importar en bóveda vacía",
      }),
    ).toBeDisabled();
    expect(screen.getByText(/esta bóveda ya tiene datos/i)).toBeInTheDocument();
  });

  it("pide confirmación y restaura el backup principal", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Bóveda" }));
    await user.click(
      await screen.findByRole("button", {
        name: "Restaurar backup principal",
      }),
    );

    expect(await screen.findByRole("alertdialog")).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Sí, restaurar backup" }),
    );

    await waitFor(() => expect(api.restoreMainBackup).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText(/backup principal restaurado/i),
    ).toBeInTheDocument();
  });

  it("carga la estructura académica y crea una materia", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Materias" }));

    expect(
      await screen.findByRole("heading", {
        name: "Estructura académica",
        level: 1,
      }),
    ).toBeInTheDocument();
    expect(await screen.findByText("2026")).toBeInTheDocument();
    expect(await screen.findByText("Primer semestre")).toBeInTheDocument();
    expect(await screen.findByText("Álgebra lineal")).toBeInTheDocument();

    const input = screen.getByLabelText("Materia");
    await user.type(input, "Física I");

    const physics = {
      ...algebra,
      id: "subject-2",
      name: "Física I",
      code: null,
      teacher: null,
      room: null,
      credits: null,
      attendanceMinBp: null,
    };
    api.createSubject.mockResolvedValue(physics);
    api.listSubjects.mockResolvedValue([algebra, physics]);

    await user.click(screen.getByRole("button", { name: "Agregar materia" }));

    await waitFor(() =>
      expect(api.createSubject).toHaveBeenCalledWith(
        "term-1",
        expect.objectContaining({ name: "Física I" }),
      ),
    );
    expect(await screen.findByText("Física I")).toBeInTheDocument();
  });

  it("carga la semana y registra asistencia desde Horario", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Horario" }));

    expect(
      await screen.findByRole("heading", {
        name: "Horario semanal",
        level: 1,
      }),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("2026-03-02 → 2026-03-08"),
    ).toBeInTheDocument();
    expect(await screen.findByText("Rotación A")).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Marcar presente «Álgebra lineal» 2026-03-02",
      }),
    ).toBeInTheDocument();

    api.getWeekSchedule.mockResolvedValue({
      ...weekSchedule,
      occurrences: [
        {
          ...weekSchedule.occurrences[0],
          attendanceStatus: "present",
        },
      ],
      attendance: [
        {
          ...weekSchedule.attendance[0],
          present: 1,
          counted: 1,
          attended: 1,
          percentBp: 10000,
        },
      ],
    });

    await user.click(
      screen.getByRole("button", {
        name: "Marcar presente «Álgebra lineal» 2026-03-02",
      }),
    );

    await waitFor(() =>
      expect(api.setAttendance).toHaveBeenCalledWith(
        "slot-1",
        "2026-03-02",
        "present",
        null,
      ),
    );
    expect(
      await screen.findByRole("button", {
        name: "Marcar presente «Álgebra lineal» 2026-03-02",
        pressed: true,
      }),
    ).toBeInTheDocument();
    expect(await screen.findByText("100.0% / mín. 75.0%")).toBeInTheDocument();
  });

  it("agrega una franja horaria a la materia", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Horario" }));
    await screen.findByRole("heading", { name: "Franjas horarias" });

    await user.click(screen.getByRole("button", { name: "Agregar franja" }));

    await waitFor(() =>
      expect(api.createClassSlot).toHaveBeenCalledWith(
        "subject-1",
        expect.objectContaining({
          weekday: 1,
          startTime: "08:00",
          endTime: "10:00",
          recurrence: "weekly",
        }),
      ),
    );
  });

  it("muestra la próxima clase y registra la asistencia del día", async () => {
    const user = userEvent.setup();
    const today = todayIso();
    const occurrence = {
      ...weekSchedule.occurrences[0],
      date: today,
    };
    const todayWeek = {
      ...weekSchedule,
      occurrences: [occurrence],
    };
    api.getWeekSchedule.mockResolvedValue(todayWeek);
    api.getNextClass.mockResolvedValue(occurrence);

    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Próxima clase" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("Clases de hoy")).toBeInTheDocument();
    await waitFor(() =>
      expect(api.getNextClass).toHaveBeenCalledWith(
        "term-1",
        today,
        expect.any(String),
      ),
    );

    const mark = await screen.findByRole("button", {
      name: `Marcar presente «Álgebra lineal» ${today}`,
    });

    api.getWeekSchedule.mockResolvedValue({
      ...todayWeek,
      occurrences: [{ ...occurrence, attendanceStatus: "present" }],
      attendance: [
        {
          ...weekSchedule.attendance[0],
          present: 1,
          counted: 1,
          attended: 1,
          percentBp: 10000,
        },
      ],
    });

    await user.click(mark);

    await waitFor(() =>
      expect(api.setAttendance).toHaveBeenCalledWith(
        "slot-1",
        today,
        "present",
        null,
      ),
    );
    expect(await screen.findByText("100.0% / mín. 75.0%")).toBeInTheDocument();
  });

  it("muestra el promedio ponderado y el descarte del menor", async () => {
    const user = userEvent.setup();
    api.listGradeScales.mockResolvedValue([gradeScale]);
    api.listGradeCategories.mockResolvedValue(gradeCategories);
    api.listGradeEntries.mockResolvedValue(gradeEntries);
    api.getSubjectGrade.mockResolvedValue(subjectGrade);
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Notas" }));

    expect(
      await screen.findByRole("heading", { name: "Notas", level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByText("92.00")).toBeInTheDocument();
    expect(screen.getByText("92.0 % · aprobada")).toBeInTheDocument();
    expect(screen.getByText("Parciales · 60.0 %")).toBeInTheDocument();
    expect(screen.getByText("100.0 % · descarta 1")).toBeInTheDocument();
    expect(screen.getByText("Trabajos · 40.0 %")).toBeInTheDocument();
    expect(screen.getByText("80.0 %")).toBeInTheDocument();
    expect(api.getSubjectGrade).toHaveBeenCalledWith("subject-1");
  });

  it("busca tareas por texto en la bóveda", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Tareas" }));
    const search = await screen.findByPlaceholderText(
      "Buscar en título, descripción o etiquetas",
    );
    api.searchTasks.mockResolvedValue([pendingTask]);

    await user.type(search, "matrices");

    await waitFor(() =>
      expect(api.searchTasks).toHaveBeenLastCalledWith({ text: "matrices" }),
    );
    expect(await screen.findByText("Repasar matrices")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.queryByText("Entregar práctico")).not.toBeInTheDocument(),
    );
  });

  it("filtra tareas por etiqueta", async () => {
    const user = userEvent.setup();
    api.listTags.mockResolvedValue([examTag]);
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Tareas" }));
    const tagFilter = await screen.findByLabelText("Etiqueta");
    api.searchTasks.mockResolvedValue([
      {
        ...pendingTask,
        id: "t3",
        title: "Leer apunte de examen",
        tags: [examTag],
      },
    ]);

    await user.selectOptions(tagFilter, "tag-1");

    await waitFor(() =>
      expect(api.searchTasks).toHaveBeenLastCalledWith({ tagId: "tag-1" }),
    );
    expect(
      await screen.findByText("Leer apunte de examen"),
    ).toBeInTheDocument();
  });

  it("abre la vista de exámenes y temas", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Exámenes" }));

    expect(
      await screen.findByRole("heading", { name: "Exámenes", level: 1 }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Nuevo examen" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Crear tema" })).toBeDisabled();
  });
});
