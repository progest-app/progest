import * as React from "react";
import { FolderOpen, FolderPlus, Sparkles } from "lucide-react";

import { CommandPalette } from "@/components/command-palette";
import { DirectoryInspector } from "@/components/directory-inspector";
import { DragDropProvider, DropOverlay, useDropZone } from "@/components/drag-drop-overlay";
import { FileInspector } from "@/components/file-inspector";
import { FlatView } from "@/components/flat-view";
import { ImportModal } from "@/components/import-modal";
import { InitProjectDialog } from "@/components/init-project-dialog";
import { StatusBar } from "@/components/status-bar";
import { TreeView } from "@/components/tree-view";
import {
  ALL_PANELS_VISIBLE,
  TitleBar,
  type PanelKey,
  type PanelVisibility,
} from "@/components/title-bar";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { TooltipProvider } from "@/components/ui/tooltip";
import { FlatViewSummaryProvider } from "@/lib/flat-view-context";
import { ProjectProvider, useProject } from "@/lib/project-context";
import { ThemeProvider } from "next-themes";
import type { DirEntry, RichSearchHit } from "@/lib/ipc";

import "./App.css";

const PANEL_VISIBILITY_KEY = "progest:panel-visibility";

/**
 * Either a directory the user is inspecting (TreeView click on a dir)
 * or a single file (TreeView / FlatView / palette click on a file).
 * Mutually exclusive — picking one clears the other so the inspector
 * pane never tries to render both at once.
 */
type Selection = { kind: "dir"; path: string } | { kind: "file"; hit: RichSearchHit } | null;

function loadPanelVisibility(): PanelVisibility {
  try {
    const raw = localStorage.getItem(PANEL_VISIBILITY_KEY);
    if (!raw) return ALL_PANELS_VISIBLE;
    const parsed = JSON.parse(raw) as Partial<PanelVisibility>;
    return {
      tree: parsed.tree ?? true,
      flat: parsed.flat ?? true,
      inspector: parsed.inspector ?? true,
    };
  } catch {
    return ALL_PANELS_VISIBLE;
  }
}

export function App() {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      storageKey="progest:theme"
      disableTransitionOnChange
    >
      <TooltipProvider delayDuration={150}>
        <ProjectProvider>
          <FlatViewSummaryProvider>
            <Shell />
          </FlatViewSummaryProvider>
        </ProjectProvider>
      </TooltipProvider>
    </ThemeProvider>
  );
}

function Shell() {
  const { project } = useProject();
  const [selection, setSelection] = React.useState<Selection>(null);
  // Panel visibility lives at the shell level so the titlebar toggles
  // can drive the Resizable layout. Persisted to localStorage so user
  // preferences survive a reload.
  const [panels, setPanels] = React.useState<PanelVisibility>(() => loadPanelVisibility());
  React.useEffect(() => {
    localStorage.setItem(PANEL_VISIBILITY_KEY, JSON.stringify(panels));
  }, [panels]);
  const togglePanel = React.useCallback((key: PanelKey) => {
    setPanels((p) => {
      // Don't let the user hide every panel — there'd be nothing left
      // to interact with except the titlebar itself.
      const next = { ...p, [key]: !p[key] };
      if (!next.tree && !next.flat && !next.inspector) return p;
      return next;
    });
  }, []);

  // Reset selection when the user swaps projects — otherwise the
  // inspector keeps trying to read state for a path that may not
  // exist in the new project.
  React.useEffect(() => {
    setSelection(null);
  }, [project?.root]);

  const onPickFlatHit = React.useCallback((hit: RichSearchHit) => {
    setSelection({ kind: "file", hit });
  }, []);

  const onPickTreeFile = React.useCallback((entry: DirEntry) => {
    const hit = treeEntryToHit(entry);
    if (hit) setSelection({ kind: "file", hit });
  }, []);

  const onSelectDir = React.useCallback((path: string) => {
    setSelection({ kind: "dir", path });
  }, []);

  const selectedDir = selection?.kind === "dir" ? selection.path : "";

  // --- import via drag & drop -----------------------------------------------
  const [importSources, setImportSources] = React.useState<string[]>([]);
  const [importDest, setImportDest] = React.useState<string | undefined>();
  const [importOpen, setImportOpen] = React.useState(false);

  const treeRef = React.useRef<HTMLElement>(null);
  const dragHoverDirRef = React.useRef<string | null>(null);

  const handleDrop = React.useCallback(
    (paths: string[], position: { x: number; y: number }) => {
      if (!project || paths.length === 0) return;

      // If the drop lands on the tree pane and a specific folder is
      // highlighted, use that folder as the destination.
      let dest: string | undefined;
      if (treeRef.current) {
        const rect = treeRef.current.getBoundingClientRect();
        if (
          position.x >= rect.left &&
          position.x <= rect.right &&
          position.y >= rect.top &&
          position.y <= rect.bottom &&
          dragHoverDirRef.current != null
        ) {
          dest = dragHoverDirRef.current;
        }
      }
      dragHoverDirRef.current = null;

      setImportSources(paths);
      setImportDest(dest);
      setImportOpen(true);
    },
    [project],
  );

  return (
    <DragDropProvider onDrop={handleDrop}>
      <div className="grid h-screen grid-rows-[auto_1fr_auto] bg-background">
        <TitleBar panels={panels} onTogglePanel={togglePanel} />
        {project ? (
          <MainShell
            onPickHit={onPickFlatHit}
            onPickTreeFile={onPickTreeFile}
            selection={selection}
            selectedDir={selectedDir}
            onSelectDir={onSelectDir}
            panels={panels}
            treeRef={treeRef}
            dragHoverDirRef={dragHoverDirRef}
          />
        ) : (
          <Welcome />
        )}
        <StatusBar />
      </div>
      <CommandPalette onPickHit={onPickFlatHit} />
      <InitProjectDialog />
      <ImportModal
        open={importOpen}
        onOpenChange={setImportOpen}
        sources={importSources}
        initialDest={importDest}
      />
    </DragDropProvider>
  );
}

function MainShell(props: {
  onPickHit: (hit: RichSearchHit) => void;
  onPickTreeFile: (entry: DirEntry) => void;
  selection: Selection;
  selectedDir: string;
  onSelectDir: (path: string) => void;
  panels: PanelVisibility;
  treeRef: React.RefObject<HTMLElement | null>;
  dragHoverDirRef: React.MutableRefObject<string | null>;
}) {
  const flatRef = React.useRef<HTMLElement>(null);
  const flatDrop = useDropZone(flatRef);

  const panes: { key: PanelKey; node: React.ReactNode }[] = [];
  if (props.panels.tree) {
    panes.push({
      key: "tree",
      node: (
        <ResizablePanel id="tree" key="tree" defaultSize={22} minSize={12}>
          <aside ref={props.treeRef} className="h-full overflow-hidden">
            <TreeView
              onPickFile={props.onPickTreeFile}
              selectedDir={props.selectedDir}
              onSelectDir={props.onSelectDir}
              dragHoverDirRef={props.dragHoverDirRef}
            />
          </aside>
        </ResizablePanel>
      ),
    });
  }
  if (props.panels.flat) {
    panes.push({
      key: "flat",
      node: (
        <ResizablePanel id="flat" key="flat" defaultSize={40} minSize={20}>
          <main ref={flatRef} className="relative h-full overflow-hidden">
            <FlatView onPickHit={props.onPickHit} />
            <DropOverlay
              isOver={flatDrop.isOver}
              fileCount={flatDrop.fileCount}
              label="Auto-suggest destination"
            />
          </main>
        </ResizablePanel>
      ),
    });
  }
  if (props.panels.inspector) {
    panes.push({
      key: "inspector",
      node: (
        <ResizablePanel id="inspector" key="inspector" defaultSize={38} minSize={20}>
          <aside className="h-full overflow-hidden">
            <InspectorPane selection={props.selection} />
          </aside>
        </ResizablePanel>
      ),
    });
  }

  return (
    <div className="overflow-hidden">
      <ResizablePanelGroup orientation="horizontal" id="progest:main-shell" className="h-full">
        {panes.map((p, i) => (
          <React.Fragment key={p.key}>
            {i > 0 ? <ResizableHandle withHandle /> : null}
            {p.node}
          </React.Fragment>
        ))}
      </ResizablePanelGroup>
    </div>
  );
}

/**
 * Route the inspector pane between directory-mode and file-mode based
 * on the current selection. Empty selection falls back to the
 * directory inspector at project root, matching the previous default.
 */
function InspectorPane(props: { selection: Selection }) {
  if (props.selection?.kind === "file") {
    return <FileInspector hit={props.selection.hit} />;
  }
  const dir = props.selection?.kind === "dir" ? props.selection.path : "";
  return <DirectoryInspector dir={dir} />;
}

function Welcome() {
  const { recent, openPicker, pickRecent, openInitDialog, error } = useProject();
  return (
    <div className="flex flex-col items-center justify-center gap-6 overflow-auto p-6">
      <div className="text-center">
        <h1 className="text-2xl font-semibold tracking-tight">Progest</h1>
        <p className="text-xs text-muted-foreground">
          Open a project (a folder containing <code>.progest/</code>) or create a new one.
        </p>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-2">
        <Button onClick={() => void openPicker()}>
          <FolderOpen /> Open project…
        </Button>
        <Button variant="outline" onClick={() => openInitDialog("new")}>
          <Sparkles /> New project…
        </Button>
        <Button variant="outline" onClick={() => openInitDialog("existing")}>
          <FolderPlus /> Initialize folder…
        </Button>
      </div>
      {recent.length > 0 ? (
        <div className="grid w-full max-w-md gap-1 text-xs">
          <div className="text-muted-foreground">Recent</div>
          <ul className="grid gap-1">
            {recent.slice(0, 8).map((entry) => (
              <li key={entry.root}>
                <Button
                  variant="outline"
                  onClick={() => void pickRecent(entry)}
                  className="grid h-auto w-full grid-cols-[1fr_auto] items-center gap-2 px-2 py-1.5 text-left font-normal"
                >
                  <div className="min-w-0">
                    <div className="truncate">{entry.name || entry.root}</div>
                    <div className="truncate text-[0.625rem] text-muted-foreground">
                      {entry.root}
                    </div>
                  </div>
                  <span className="text-[0.625rem] text-muted-foreground">
                    {relTime(entry.last_opened)}
                  </span>
                </Button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {error ? <div className="text-xs text-destructive">{error}</div> : null}
    </div>
  );
}

/**
 * TreeView yields `DirEntry` for both directories and files; we only
 * route file rows into the selection slot, and reshape them into the
 * `RichSearchHit` envelope the inspector expects. Returns `null` for
 * directory entries or files that haven't been hydrated by the index
 * yet (the tree shows on-disk truth, the index lags behind reconcile).
 */
function treeEntryToHit(entry: DirEntry): RichSearchHit | null {
  if (entry.kind !== "file" || !entry.file) return null;
  return {
    file_id: entry.file.file_id ?? "",
    path: entry.path,
    name: entry.name,
    kind: entry.file.kind,
    ext: entry.file.ext,
    tags: entry.file.tags,
    violations: entry.file.violations,
    custom_fields: entry.file.custom_fields,
  };
}

function relTime(rfc3339: string): string {
  const t = Date.parse(rfc3339);
  if (Number.isNaN(t)) return "";
  const diff = Date.now() - t;
  const sec = Math.max(0, Math.floor(diff / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  return `${day}d ago`;
}
