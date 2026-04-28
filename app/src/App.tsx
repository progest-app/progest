import * as React from "react";
import { FolderOpen, FolderPlus, Sparkles } from "lucide-react";

import { CommandPalette } from "@/components/command-palette";
import { DirectoryInspector } from "@/components/directory-inspector";
import { TreeView } from "@/components/tree-view";
import { FlatView } from "@/components/flat-view";
import { InitProjectDialog } from "@/components/init-project-dialog";
import { ResultDetailDialog } from "@/components/result-detail-dialog";
import { StatusBar } from "@/components/status-bar";
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
  // Currently-selected file from the tree (DirEntry) or flat view
  // (RichSearchHit). Both feed a shared detail dialog so the user can
  // inspect a file without losing their place in the tree.
  const [hitDetail, setHitDetail] = React.useState<RichSearchHit | null>(null);
  const [treeDetail, setTreeDetail] = React.useState<DirEntry | null>(null);
  // The tree's currently-selected directory feeds the right-pane
  // inspector. Default to the project root so the inspector always has
  // something to render.
  const [selectedDir, setSelectedDir] = React.useState<string>("");
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
  // inspector keeps trying to read accepts for a dir that may not
  // exist in the new project.
  React.useEffect(() => {
    setSelectedDir("");
  }, [project?.root]);

  return (
    <>
      <div className="grid h-screen grid-rows-[auto_1fr_auto] bg-background">
        <TitleBar panels={panels} onTogglePanel={togglePanel} />
        {project ? (
          <MainShell
            onPickHit={(h) => setHitDetail(h)}
            onPickTreeFile={(e) => setTreeDetail(e)}
            selectedDir={selectedDir}
            onSelectDir={setSelectedDir}
            panels={panels}
          />
        ) : (
          <Welcome />
        )}
        <StatusBar />
      </div>
      {/*
        CommandPalette is mounted globally so Cmd+K works even from the
        Welcome screen. Its hit handler routes through the same detail
        dialog as the FlatView selection.
      */}
      <CommandPalette onPickHit={(h) => setHitDetail(h)} />
      <InitProjectDialog />
      <ResultDetailDialog
        hit={hitDetail}
        open={hitDetail !== null}
        onOpenChange={(o) => {
          if (!o) setHitDetail(null);
        }}
      />
      <TreeFileDetail
        entry={treeDetail}
        onOpenChange={(o) => {
          if (!o) setTreeDetail(null);
        }}
      />
    </>
  );
}

function MainShell(props: {
  onPickHit: (hit: RichSearchHit) => void;
  onPickTreeFile: (entry: DirEntry) => void;
  selectedDir: string;
  onSelectDir: (path: string) => void;
  panels: PanelVisibility;
}) {
  // Build the panel list dynamically so hidden panels disappear from
  // the Resizable group entirely (and so do their handles). Each
  // ResizablePanel keeps its `id` so the library's persisted layout
  // can rehydrate correctly when the panel returns.
  const panes: { key: PanelKey; node: React.ReactNode }[] = [];
  if (props.panels.tree) {
    panes.push({
      key: "tree",
      node: (
        <ResizablePanel id="tree" key="tree" defaultSize={22} minSize={12}>
          <aside className="h-full overflow-hidden">
            <TreeView
              onPickFile={props.onPickTreeFile}
              selectedDir={props.selectedDir}
              onSelectDir={props.onSelectDir}
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
          <main className="h-full overflow-hidden">
            <FlatView onPickHit={props.onPickHit} />
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
            <DirectoryInspector dir={props.selectedDir} />
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

function TreeFileDetail(props: { entry: DirEntry | null; onOpenChange: (open: boolean) => void }) {
  // The tree node carries the same FileEntry shape the flat view does,
  // but without the file_id-keyed `path` field. We synthesize a
  // RichSearchHit-shaped payload so ResultDetailDialog can render it.
  const hit = React.useMemo<RichSearchHit | null>(() => {
    const entry = props.entry;
    if (!entry || entry.kind !== "file" || !entry.file) return null;
    return {
      file_id: entry.file.file_id ?? "(unindexed)",
      path: entry.path,
      name: entry.name,
      kind: entry.file.kind,
      ext: entry.file.ext,
      tags: entry.file.tags,
      violations: entry.file.violations,
      custom_fields: entry.file.custom_fields,
    };
  }, [props.entry]);
  return <ResultDetailDialog hit={hit} open={hit !== null} onOpenChange={props.onOpenChange} />;
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
