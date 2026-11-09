import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { runningInTauri } from "../lib/ipc";

type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

const handles: ReadonlyArray<{
  className: string;
  direction: ResizeDirection;
}> = [
  { className: "resize-n", direction: "North" },
  { className: "resize-s", direction: "South" },
  { className: "resize-e", direction: "East" },
  { className: "resize-w", direction: "West" },
  { className: "resize-ne", direction: "NorthEast" },
  { className: "resize-nw", direction: "NorthWest" },
  { className: "resize-se", direction: "SouthEast" },
  { className: "resize-sw", direction: "SouthWest" },
];

/**
 * Con `decorations: false` la ventana pierde el borde nativo de
 * redimensionado. Estos bordes invisibles delegan el arrastre a Tauri.
 *
 * En una ventana maximizada no hay borde que arrastrar y los handles pueden
 * dejar artefactos de repintado en algunos compositores, así que se ocultan.
 */
export function ResizeHandles() {
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

  if (!runningInTauri || maximized) {
    return null;
  }

  return (
    <>
      {handles.map(({ className, direction }) => (
        <button
          key={direction}
          type="button"
          tabIndex={-1}
          aria-hidden="true"
          className={`resize-handle ${className}`}
          data-tauri-drag-region="false"
          onMouseDown={(event) => {
            if (event.button !== 0) {
              return;
            }
            event.preventDefault();
            getCurrentWindow()
              .startResizeDragging(direction)
              .catch(() => undefined);
          }}
        />
      ))}
    </>
  );
}
