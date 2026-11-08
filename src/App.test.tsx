import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

afterEach(cleanup);

describe("App", () => {
  it("muestra la identidad de Omega y la navegación principal", () => {
    render(<App />);
    expect(screen.getByText("Omega")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /tu año académico/i }),
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
});
