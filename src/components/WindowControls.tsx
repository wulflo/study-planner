import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { runningInTauri } from "../lib/ipc";

type WindowAction = "minimize" | "toggleMaximize" | "close";

type WindowIconName = "minimize" | "maximize" | "restore" | "close";

function runWindowAction(action: WindowAction) {
  const win = getCurrentWindow();
  win[action]().catch((error: unknown) => {
    console.error(`No se pudo ejecutar «${action}» en la ventana`, error);
  });
}

function WindowIcon({ name }: { name: WindowIconName }) {
  return (
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      {name === "minimize" ? <path d="M3 8h10" /> : null}
      {name === "maximize" ? <path d="M3.5 3.5h9v9h-9z" /> : null}
      {name === "restore" ? (
        <>
          <path d="M5.5 3.5h7v7" />
          <path d="M3.5 5.5h7v7h-7z" />
        </>
      ) : null}
      {name === "close" ? <path d="M3 3l10 10M13 3L3 13" /> : null}
    </svg>
  );
}

/**
 * Controles de ventana integrados en la barra de título propia.
 * Solo se muestran cuando la app corre dentro de Tauri.
 */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!runningInTauri) {
      return;
    }

    const win = getCurrentWindow();
    let active = true;
    let unlisten: (() => void) | undefined;

    const sync = () => {
      win
        .isMaximized()
        .then((value) => {
          if (active) {
            setMaximized(value);
          }
        })
        .catch(() => undefined);
    };

    sync();
    win
      .onResized(sync)
      .then((stop) => {
        if (active) {
          unlisten = stop;
        } else {
          stop();
        }
      })
      .catch(() => undefined);

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  if (!runningInTauri) {
    return null;
  }

  return (
    <div className="window-controls" role="toolbar" aria-label="Ventana">
      <button
        type="button"
        className="window-control"
        aria-label="Minimizar"
        title="Minimizar"
        onClick={() => runWindowAction("minimize")}
      >
        <WindowIcon name="minimize" />
      </button>
      <button
        type="button"
        className="window-control"
        aria-label={maximized ? "Restaurar" : "Maximizar"}
        title={maximized ? "Restaurar" : "Maximizar"}
        aria-pressed={maximized}
        onClick={() => runWindowAction("toggleMaximize")}
      >
        <WindowIcon name={maximized ? "restore" : "maximize"} />
      </button>
      <button
        type="button"
        className="window-control close"
        aria-label="Cerrar"
        title="Cerrar"
        onClick={() => runWindowAction("close")}
      >
        <WindowIcon name="close" />
      </button>
    </div>
  );
}
