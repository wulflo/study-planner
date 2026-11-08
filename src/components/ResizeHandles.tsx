import { getCurrentWindow } from "@tauri-apps/api/window";
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
 */
export function ResizeHandles() {
  if (!runningInTauri) {
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
