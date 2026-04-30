import * as React from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Import } from "lucide-react";

type DragDropState = {
  active: boolean;
  paths: string[];
  position: { x: number; y: number } | null;
};

type DragDropContextValue = {
  state: DragDropState;
};

const DragDropCtx = React.createContext<DragDropContextValue>({
  state: { active: false, paths: [], position: null },
});

/**
 * Check whether the current drag cursor is within a given element.
 * Returns `isOver` plus the drop-zone label (for the import modal to
 * know whether the drop landed on a tree directory or the flat pane).
 */
export function useDropZone(ref: React.RefObject<HTMLElement | null>): {
  isOver: boolean;
  fileCount: number;
} {
  const { state } = React.useContext(DragDropCtx);

  const isOver = React.useMemo(() => {
    if (!state.active || !state.position || !ref.current) return false;
    const rect = ref.current.getBoundingClientRect();
    return (
      state.position.x >= rect.left &&
      state.position.x <= rect.right &&
      state.position.y >= rect.top &&
      state.position.y <= rect.bottom
    );
  }, [state.active, state.position, ref]);

  return { isOver, fileCount: state.paths.length };
}

/**
 * Expose the raw drag state for components that need fine-grained
 * hit-testing (e.g. TreeView folder highlight).
 */
export function useDragActive(): DragDropState {
  return React.useContext(DragDropCtx).state;
}

/**
 * Full-window provider that listens to Tauri's native drag-drop events.
 * Individual panes use `useDropZone(ref)` to show per-pane overlays.
 *
 * When the user drops, `onDrop` fires with the absolute file paths
 * and the drop position (to determine which pane received the drop).
 */
export function DragDropProvider(props: {
  children: React.ReactNode;
  onDrop: (paths: string[], position: { x: number; y: number }) => void;
}) {
  const [state, setState] = React.useState<DragDropState>({
    active: false,
    paths: [],
    position: null,
  });

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      const appWindow = getCurrentWebviewWindow();
      unlisten = await appWindow.onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter") {
          setState({
            active: true,
            paths: p.paths,
            position: p.position,
          });
        } else if (p.type === "over") {
          setState((prev) => ({
            ...prev,
            position: p.position,
          }));
        } else if (p.type === "drop") {
          setState((prev) => {
            const paths = prev.paths.length > 0 ? prev.paths : p.paths;
            props.onDrop(paths, p.position);
            return { active: false, paths: [], position: null };
          });
        } else {
          setState({ active: false, paths: [], position: null });
        }
      });
    };

    void setup();

    return () => {
      unlisten?.();
    };
  }, [props.onDrop]);

  const ctxValue = React.useMemo(() => ({ state }), [state]);

  return <DragDropCtx.Provider value={ctxValue}>{props.children}</DragDropCtx.Provider>;
}

/**
 * Per-pane drop overlay — render as a sibling of the pane content
 * inside a `relative` container.
 */
export function DropOverlay(props: { isOver: boolean; fileCount: number; label?: string }) {
  if (!props.isOver) return null;
  return (
    <div className="absolute inset-0 z-40 flex items-center justify-center bg-background/60 backdrop-blur-sm rounded-md border-2 border-dashed border-primary/50 transition-all">
      <div className="flex flex-col items-center gap-2">
        <Import className="size-8 text-primary animate-pulse" />
        <div className="text-xs font-medium">
          Drop to import {props.fileCount} file{props.fileCount === 1 ? "" : "s"}
        </div>
        {props.label ? (
          <div className="text-[0.625rem] text-muted-foreground">{props.label}</div>
        ) : null}
      </div>
    </div>
  );
}
