import { useEffect, useState } from "react";

export type ThemeMode = "system" | "light" | "dark";

export type AccentId =
  | "violet"
  | "blue"
  | "teal"
  | "green"
  | "amber"
  | "orange"
  | "pink"
  | "red";

export const THEME_MODES: ReadonlyArray<{
  id: ThemeMode;
  label: string;
}> = [
  { id: "system", label: "Sistema" },
  { id: "light", label: "Claro" },
  { id: "dark", label: "Oscuro" },
];

export const ACCENTS: ReadonlyArray<{
  id: AccentId;
  label: string;
  swatch: string;
}> = [
  { id: "violet", label: "Violeta", swatch: "#6750a4" },
  { id: "blue", label: "Azul", swatch: "#2f6fed" },
  { id: "teal", label: "Verde azulado", swatch: "#00796b" },
  { id: "green", label: "Verde", swatch: "#2e7d32" },
  { id: "amber", label: "Ámbar", swatch: "#b26a00" },
  { id: "orange", label: "Naranja", swatch: "#c2410c" },
  { id: "pink", label: "Rosa", swatch: "#b0306b" },
  { id: "red", label: "Rojo", swatch: "#b3261e" },
];

const MODE_KEY = "study-planner.theme";
const ACCENT_KEY = "study-planner.accent";
const DEFAULT_MODE: ThemeMode = "system";
const DEFAULT_ACCENT: AccentId = "violet";

function isThemeMode(value: string | null): value is ThemeMode {
  return value === "system" || value === "light" || value === "dark";
}

function isAccentId(value: string | null): value is AccentId {
  return ACCENTS.some((accent) => accent.id === value);
}

function readStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // El tema sigue funcionando aunque el navegador bloquee el almacenamiento.
  }
}

export function readThemeMode(): ThemeMode {
  const stored = readStorage(MODE_KEY);
  return isThemeMode(stored) ? stored : DEFAULT_MODE;
}

export function readAccent(): AccentId {
  const stored = readStorage(ACCENT_KEY);
  return isAccentId(stored) ? stored : DEFAULT_ACCENT;
}

function systemPrefersDark(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );
}

export function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode === "system") {
    return systemPrefersDark() ? "dark" : "light";
  }
  return mode;
}

export function applyTheme(mode: ThemeMode, accent: AccentId) {
  const root = document.documentElement;
  const resolved = resolveTheme(mode);
  root.dataset.theme = resolved;
  root.dataset.accent = accent;
  root.style.colorScheme = resolved;
}

export function applyStoredTheme() {
  applyTheme(readThemeMode(), readAccent());
}

export function useTheme() {
  const [mode, setMode] = useState<ThemeMode>(readThemeMode);
  const [accent, setAccent] = useState<AccentId>(readAccent);

  useEffect(() => {
    applyTheme(mode, accent);
    writeStorage(MODE_KEY, mode);
    writeStorage(ACCENT_KEY, accent);

    if (mode !== "system" || typeof window.matchMedia !== "function") {
      return;
    }

    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const sync = () => applyTheme(mode, accent);
    query.addEventListener("change", sync);
    return () => query.removeEventListener("change", sync);
  }, [mode, accent]);

  return { mode, accent, setMode, setAccent };
}
