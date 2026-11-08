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
  title: "Repasar matrices",
  status: "pending",
  createdAt: "2026-11-08T10:00:00.000Z",
  completedAt: null,
};

const doneTask = {
  id: "t2",
  title: "Entregar práctico",
  status: "done",
  createdAt: "2026-11-07T10:00:00.000Z",
  completedAt: "2026-11-08T09:00:00.000Z",
};

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
    expect(screen.getByText("Hechas")).toBeInTheDocument();
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
});
