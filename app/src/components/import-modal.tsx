import * as React from "react";
import { AlertTriangle, Check, Copy, FileWarning, FolderInput, Scissors } from "lucide-react";

import {
  importApply,
  importPreview,
  importRanking,
  type ImportConflict,
  type ImportOp,
  type ImportOutcome,
  type ImportPreview,
  type ImportRequestWire,
  type SuggestedDestination,
} from "@/lib/ipc";
import { useProject } from "@/lib/project-context";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

type ImportModalProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Absolute paths of files dropped by the user. */
  sources: string[];
  /** Pre-selected destination directory (from TreeView drop). */
  initialDest: string | undefined;
};

export function ImportModal(props: ImportModalProps) {
  const { open, onOpenChange, sources, initialDest } = props;
  const { bumpRefresh } = useProject();

  const [mode, setMode] = React.useState<"copy" | "move">("copy");
  const [dest, setDest] = React.useState("");
  const [suggestions, setSuggestions] = React.useState<SuggestedDestination[]>([]);
  const [preview, setPreview] = React.useState<ImportPreview | null>(null);
  const [outcome, setOutcome] = React.useState<ImportOutcome | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);

  // Fetch ranking suggestions when the modal opens.
  React.useEffect(() => {
    if (!open || sources.length === 0) return;
    setDest(initialDest ?? "");
    setPreview(null);
    setOutcome(null);
    setError(null);
    setMode("copy");
    setSuggestions([]);

    importRanking(sources)
      .then((resp) => {
        setSuggestions(resp.suggestions);
        const first = resp.suggestions[0];
        if (!initialDest && first) {
          setDest(first.path);
        }
      })
      .catch(() => {
        // Non-fatal — user can still type a dest.
      });
  }, [open, sources, initialDest]);

  // Build preview whenever dest or mode changes.
  React.useEffect(() => {
    if (!open || sources.length === 0) return;
    if (!dest && !initialDest) {
      setPreview(null);
      return;
    }

    const activeDest = dest || "";
    const requests: ImportRequestWire[] = sources.map((src) => {
      const filename = src.split("/").pop() ?? src;
      const destPath = activeDest ? `${activeDest}/${filename}` : filename;
      return { source: src, dest: destPath, mode };
    });

    let cancelled = false;
    importPreview(requests)
      .then((p) => {
        if (!cancelled) setPreview(p);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });

    return () => {
      cancelled = true;
    };
  }, [open, sources, dest, mode, initialDest]);

  const handleApply = React.useCallback(async () => {
    if (!preview || !preview.clean) return;
    setBusy(true);
    setError(null);
    try {
      const requests: ImportRequestWire[] = sources.map((src) => {
        const filename = src.split("/").pop() ?? src;
        const destPath = dest ? `${dest}/${filename}` : filename;
        return { source: src, dest: destPath, mode };
      });
      const result = await importApply(requests);
      setOutcome(result);
      bumpRefresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [preview, sources, dest, mode, bumpRefresh]);

  const fileNames = React.useMemo(() => sources.map((s) => s.split("/").pop() ?? s), [sources]);

  if (outcome) {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Check className="size-5 text-green-500" />
              Import complete
            </DialogTitle>
            <DialogDescription>
              {outcome.imported.length} file{outcome.imported.length === 1 ? "" : "s"} imported
              {outcome.warnings.length > 0 ? ` with ${outcome.warnings.length} warning(s)` : ""}
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-48 overflow-auto text-xs font-mono space-y-0.5">
            {outcome.imported.map((f) => (
              <div key={f.dest} className="flex items-center gap-1.5 text-muted-foreground">
                {f.mode === "move" ? (
                  <Scissors className="size-3 shrink-0" />
                ) : (
                  <Copy className="size-3 shrink-0" />
                )}
                <span className="truncate">{f.dest}</span>
              </div>
            ))}
          </div>
          {outcome.warnings.length > 0 ? (
            <div className="rounded border border-warning/30 bg-warning/5 p-2 text-xs space-y-0.5">
              {outcome.warnings.map((w, i) => (
                <div key={`${w.dest}-${i}`} className="text-warning">
                  {w.kind}: {w.message}
                </div>
              ))}
            </div>
          ) : null}
          <DialogFooter>
            <Button onClick={() => onOpenChange(false)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FolderInput className="size-5" />
            Import {sources.length} file{sources.length === 1 ? "" : "s"}
          </DialogTitle>
          <DialogDescription>Choose a destination directory and import mode.</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4">
          {/* File list preview */}
          <div className="max-h-32 overflow-auto rounded border bg-muted/30 p-2 text-xs font-mono space-y-0.5">
            {fileNames.map((name, i) => (
              <div key={`${name}-${i}`} className="truncate">
                {name}
              </div>
            ))}
          </div>

          {/* Destination */}
          <div className="grid gap-1.5">
            <Label htmlFor="import-dest">Destination directory</Label>
            {suggestions.length > 0 ? (
              <Select value={dest} onValueChange={setDest}>
                <SelectTrigger id="import-dest">
                  <SelectValue placeholder="Select destination…" />
                </SelectTrigger>
                <SelectContent>
                  {suggestions.map((s) => (
                    <SelectItem key={s.path} value={s.path}>
                      <span className="font-mono text-xs">{s.path || "(project root)"}</span>
                      <span className="ml-2 text-muted-foreground">
                        {s.score === 3
                          ? "exact match"
                          : s.score === 2
                            ? "alias match"
                            : "inherited"}
                      </span>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <Input
                id="import-dest"
                value={dest}
                onChange={(e) => setDest(e.target.value)}
                placeholder="e.g. assets/textures"
                className="font-mono text-xs"
              />
            )}
          </div>

          {/* Mode toggle */}
          <div className="grid gap-1.5">
            <Label>Import mode</Label>
            <div className="flex gap-2">
              <Button
                variant={mode === "copy" ? "default" : "outline"}
                size="sm"
                onClick={() => setMode("copy")}
              >
                <Copy className="size-3.5 mr-1" /> Copy
              </Button>
              <Button
                variant={mode === "move" ? "default" : "outline"}
                size="sm"
                onClick={() => setMode("move")}
              >
                <Scissors className="size-3.5 mr-1" /> Move
              </Button>
            </div>
            {mode === "move" ? (
              <p className="text-xs text-warning">Move deletes the original files after import.</p>
            ) : null}
          </div>

          {/* Preview / conflicts */}
          {preview ? (
            preview.clean ? (
              <div className="flex items-center gap-1.5 text-xs text-green-600 dark:text-green-400">
                <Check className="size-3.5" />
                {preview.ops.length} file{preview.ops.length === 1 ? "" : "s"} ready to import
              </div>
            ) : (
              <div className="space-y-1.5">
                <div className="flex items-center gap-1.5 text-xs text-warning">
                  <AlertTriangle className="size-3.5" />
                  {preview.conflict_count} conflict{preview.conflict_count === 1 ? "" : "s"}
                </div>
                <div className="max-h-32 overflow-auto rounded border border-warning/30 bg-warning/5 p-2 text-xs space-y-1">
                  {preview.ops
                    .filter((op) => op.conflicts.length > 0)
                    .map((op) => (
                      <ConflictRow key={op.source} op={op} />
                    ))}
                </div>
              </div>
            )
          ) : null}

          {error ? <div className="text-xs text-destructive">{error}</div> : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => void handleApply()} disabled={busy || !preview?.clean}>
            {busy
              ? "Importing…"
              : `Import ${sources.length} file${sources.length === 1 ? "" : "s"}`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ConflictRow(props: { op: ImportOp }) {
  const filename = props.op.source.split("/").pop() ?? props.op.source;
  return (
    <div>
      <div className="font-mono truncate font-medium">{filename}</div>
      {props.op.conflicts.map((c, i) => (
        <ConflictDetail key={i} conflict={c} />
      ))}
    </div>
  );
}

function ConflictDetail(props: { conflict: ImportConflict }) {
  const c = props.conflict;
  switch (c.kind) {
    case "dest_exists":
      return (
        <div className="flex items-center gap-1 text-warning pl-2">
          <FileWarning className="size-3 shrink-0" />
          Destination exists: {c.existing_path}
        </div>
      );
    case "source_missing":
      return (
        <div className="flex items-center gap-1 text-destructive pl-2">
          <AlertTriangle className="size-3 shrink-0" />
          Source not found: {c.reason}
        </div>
      );
    case "source_is_project":
      return (
        <div className="flex items-center gap-1 text-warning pl-2">
          <AlertTriangle className="size-3 shrink-0" />
          Already in project — use rename instead
        </div>
      );
    case "placement_mismatch":
      return (
        <div className="flex items-center gap-1 text-warning pl-2">
          <AlertTriangle className="size-3 shrink-0" />
          Dir doesn't accept this extension
          {c.suggestion ? ` (try ${c.suggestion})` : ""}
        </div>
      );
    default:
      return null;
  }
}
