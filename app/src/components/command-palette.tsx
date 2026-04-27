import * as React from "react";
import { History, Triangle, Hash } from "lucide-react";

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import {
  appInfo,
  IpcError,
  searchExecute,
  searchHistoryClear,
  searchHistoryList,
  type HistoryEntry,
  type ProjectInfo,
  type RichSearchHit,
  type SearchResponse,
} from "@/lib/ipc";
import { ResultDetailDialog } from "@/components/result-detail-dialog";

// Debounce window between the last keystroke and the IPC search call.
// Tuned for "feels live" while keeping busy typing from saturating the
// executor on large projects.
const SEARCH_DEBOUNCE_MS = 200;

export function CommandPalette() {
  const [open, setOpen] = React.useState(false);
  const [project, setProject] = React.useState<ProjectInfo | null>(null);
  const [query, setQuery] = React.useState("");
  const [response, setResponse] = React.useState<SearchResponse | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [history, setHistory] = React.useState<HistoryEntry[]>([]);
  const [error, setError] = React.useState<string | null>(null);
  const [selected, setSelected] = React.useState<RichSearchHit | null>(null);

  // Boot probe: figure out whether a project is attached. Re-fetched on
  // every palette open so a future project switcher will surface here
  // without extra plumbing.
  const refreshProject = React.useCallback(async () => {
    try {
      const info = await appInfo();
      setProject(info.project);
    } catch (e) {
      setProject(null);
      setError(e instanceof IpcError ? e.raw : String(e));
    }
  }, []);

  React.useEffect(() => {
    void refreshProject();
  }, [refreshProject]);

  // Cmd+K / Ctrl+K toggle.
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const refreshHistory = React.useCallback(async () => {
    try {
      const list = await searchHistoryList();
      setHistory(list);
    } catch (e) {
      if (e instanceof IpcError && !e.isNoProject) {
        setError(e.raw);
      }
    }
  }, []);

  // Reload history every time the palette opens so it always reflects
  // the most-recent submissions. Cheap (atomic read of a small JSON).
  React.useEffect(() => {
    if (!open) return;
    void refreshHistory();
  }, [open, refreshHistory]);

  // Debounced search.
  React.useEffect(() => {
    const trimmed = query.trim();
    if (!open || trimmed.length === 0) {
      setResponse(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    const handle = setTimeout(async () => {
      try {
        const res = await searchExecute(trimmed);
        setResponse(res);
        setError(null);
      } catch (e) {
        const msg = e instanceof IpcError ? e.raw : String(e);
        setError(msg);
        setResponse(null);
      } finally {
        setLoading(false);
      }
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(handle);
  }, [query, open]);

  const onPickHit = (hit: RichSearchHit) => {
    setSelected(hit);
    setOpen(false);
  };

  const onPickHistory = (entry: HistoryEntry) => {
    setQuery(entry.query);
  };

  const onClearHistory = async () => {
    try {
      await searchHistoryClear();
      setHistory([]);
    } catch (e) {
      const msg = e instanceof IpcError ? e.raw : String(e);
      setError(msg);
    }
  };

  return (
    <>
      <PaletteShortcutHint hasProject={project !== null} onOpen={() => setOpen(true)} />
      <CommandDialog
        open={open}
        onOpenChange={setOpen}
        title="Search"
        description="Find files by tag, type, name, or arbitrary DSL query."
      >
        {/*
          shouldFilter={false}: results come pre-filtered from the
          backend; cmdk's built-in fuzzy filter would mangle the order
          and hide hits that don't match its lowercase substring rule.
        */}
        <Command shouldFilter={false}>
          <CommandInput
            value={query}
            onValueChange={setQuery}
            placeholder="tag:wip type:psd is:violation …  (Cmd+K)"
            autoFocus
          />
          <CommandList>
            {/* No-project empty state. Search is meaningless without an attached project. */}
            {project === null ? (
              <CommandEmpty>
                No Progest project attached. Launch <code>progest-desktop</code>
                {" from inside a project, or set "}
                <code>PROGEST_PROJECT</code>.
              </CommandEmpty>
            ) : query.trim().length === 0 ? (
              history.length > 0 ? (
                <CommandGroup heading="Recent">
                  {history.map((entry) => (
                    <CommandItem
                      key={entry.query}
                      value={entry.query}
                      onSelect={() => onPickHistory(entry)}
                    >
                      <History className="opacity-60" />
                      <span className="truncate">{entry.query}</span>
                      <CommandShortcut>{relTime(entry.ts)}</CommandShortcut>
                    </CommandItem>
                  ))}
                  <CommandSeparator />
                  <CommandItem value="__clear" onSelect={onClearHistory}>
                    <span className="text-muted-foreground">Clear recent queries</span>
                  </CommandItem>
                </CommandGroup>
              ) : (
                <CommandEmpty>
                  Start typing a query. e.g. <code>tag:wip</code>,{" "}
                  <code>type:psd</code>, <code>is:misplaced</code>.
                </CommandEmpty>
              )
            ) : (
              <SearchBody
                response={response}
                loading={loading}
                onPick={onPickHit}
              />
            )}
          </CommandList>
        </Command>
        {/* Status row pinned below the list — warnings, parse errors, IPC errors. */}
        <PaletteStatus
          response={response}
          error={error}
          loading={loading}
          project={project}
        />
      </CommandDialog>
      <ResultDetailDialog
        hit={selected}
        open={selected !== null}
        onOpenChange={(o) => {
          if (!o) setSelected(null);
        }}
      />
    </>
  );
}

function SearchBody(props: {
  response: SearchResponse | null;
  loading: boolean;
  onPick: (hit: RichSearchHit) => void;
}) {
  const { response, loading, onPick } = props;
  if (loading && !response) {
    return <CommandEmpty>Searching…</CommandEmpty>;
  }
  if (!response) return <CommandEmpty>Type to search.</CommandEmpty>;
  if (response.parse_error) {
    return (
      <CommandEmpty>
        <div className="text-destructive">Parse error: {response.parse_error.message}</div>
      </CommandEmpty>
    );
  }
  if (response.hits.length === 0) {
    return <CommandEmpty>No results.</CommandEmpty>;
  }
  return (
    <CommandGroup heading={`${response.hits.length} hit${response.hits.length === 1 ? "" : "s"}`}>
      {response.hits.map((hit) => (
        <CommandItem key={hit.file_id} value={hit.file_id} onSelect={() => onPick(hit)}>
          <span className="truncate font-mono">{hit.path}</span>
          <ViolationBadges counts={hit.violations} />
          {hit.tags.length > 0 ? (
            <CommandShortcut>
              <span className="opacity-70">{hit.tags.map((t) => `#${t}`).join(" ")}</span>
            </CommandShortcut>
          ) : null}
        </CommandItem>
      ))}
    </CommandGroup>
  );
}

function ViolationBadges({ counts }: { counts: { naming: number; placement: number; sequence: number } }) {
  const total = counts.naming + counts.placement + counts.sequence;
  if (total === 0) return null;
  return (
    <span className="ml-auto flex items-center gap-1 text-[0.625rem] tracking-wide">
      {counts.naming > 0 ? (
        <span className="rounded bg-amber-500/15 px-1 py-0.5 text-amber-600 dark:text-amber-300">
          <Triangle className="inline size-2.5" /> {counts.naming}
        </span>
      ) : null}
      {counts.placement > 0 ? (
        <span className="rounded bg-sky-500/15 px-1 py-0.5 text-sky-600 dark:text-sky-300">
          <Hash className="inline size-2.5" /> {counts.placement}
        </span>
      ) : null}
      {counts.sequence > 0 ? (
        <span className="rounded bg-violet-500/15 px-1 py-0.5 text-violet-600 dark:text-violet-300">
          ≡ {counts.sequence}
        </span>
      ) : null}
    </span>
  );
}

function PaletteStatus(props: {
  response: SearchResponse | null;
  error: string | null;
  loading: boolean;
  project: ProjectInfo | null;
}) {
  const { response, error, loading, project } = props;
  const lines: React.ReactNode[] = [];
  if (project) {
    lines.push(
      <span key="proj" className="text-muted-foreground">
        {project.name}
      </span>,
    );
  }
  if (loading) {
    lines.push(
      <span key="loading" className="text-muted-foreground">
        searching…
      </span>,
    );
  }
  if (response?.warnings.length) {
    lines.push(
      <span key="warn" className="text-amber-600 dark:text-amber-300">
        {response.warnings.length} warning{response.warnings.length === 1 ? "" : "s"}:{" "}
        {response.warnings.join("; ")}
      </span>,
    );
  }
  if (error) {
    lines.push(
      <span key="err" className="text-destructive">
        {error}
      </span>,
    );
  }
  if (lines.length === 0) return null;
  return (
    <div className="flex items-center gap-3 border-t px-3 py-1.5 text-[0.625rem]">
      {lines}
    </div>
  );
}

function PaletteShortcutHint(props: { hasProject: boolean; onOpen: () => void }) {
  return (
    <button
      type="button"
      onClick={props.onOpen}
      className="fixed top-3 right-3 z-30 inline-flex items-center gap-2 rounded-md border bg-card px-2.5 py-1 text-xs text-muted-foreground shadow-sm hover:bg-accent"
    >
      <span>Search</span>
      <kbd className="rounded bg-muted px-1.5 py-0.5 text-[0.625rem]">⌘K</kbd>
      {!props.hasProject ? (
        <span className="text-amber-600 dark:text-amber-300">no project</span>
      ) : null}
    </button>
  );
}

function relTime(rfc3339: string): string {
  const t = Date.parse(rfc3339);
  if (Number.isNaN(t)) return "";
  const diffMs = Date.now() - t;
  const sec = Math.max(0, Math.floor(diffMs / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  return `${day}d ago`;
}
