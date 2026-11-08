import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

afterEach(cleanup);

describe("App en navegador", () => {
  it("muestra la identidad, la navegación y el primer paso", () => {
    render(<App />);
    expect(screen.getByText("Study Planner")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /empezá por elegir tu bóveda/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("navigation")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /horario/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /tareas/i })).toBeInTheDocument();
  });

  it("explica el modo navegador cuando no hay IPC de Tauri", () => {
    render(<App />);
    expect(screen.getByText(/modo navegador/i)).toBeInTheDocument();
  });

  it("cambia de vista al seleccionar una sección", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Horario" }));

    expect(
      screen.getByRole("heading", { name: "Horario semanal", level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByText(/bóveda requerida/i)).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /empezá por elegir/i }),
    ).not.toBeInTheDocument();
  });

  it("vuelve a Hoy desde una sección provisional", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Calendario" }));
    await user.click(screen.getByRole("button", { name: "Volver a Hoy" }));

    expect(
      screen.getByRole("heading", { name: /empezá por elegir/i }),
    ).toBeInTheDocument();
  });

  it("lleva a Bóveda y deshabilita el selector fuera de Tauri", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Configurar bóveda" }));

    expect(
      screen.getByRole("heading", { name: "Bóveda local", level: 1 }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Elegir carpeta" }),
    ).toBeDisabled();
    expect(screen.getByText(/modo navegador/i)).toBeInTheDocument();
  });

  it("pide configurar la bóveda antes de mostrar tareas", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Tareas" }));

    expect(
      screen.getByRole("heading", { name: "Tareas", level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByText(/bóveda requerida/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Ir a Bóveda" }),
    ).toBeInTheDocument();
  });

  it("muestra Ajustes y pide configurar la bóveda", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Ajustes" }));

    expect(
      screen.getByRole("heading", { name: "Ajustes", level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByText(/bóveda requerida/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Ir a Bóveda" }),
    ).toBeInTheDocument();
  });

  it("abre y cierra el checklist del proyecto", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Ver checklist" }));
    expect(
      screen.getByRole("dialog", { name: "Checklist persistente" }),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Cerrar checklist" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
