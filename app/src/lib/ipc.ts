import { invoke } from "@tauri-apps/api/core";

// Wire types mirror crates/progest-tauri/src/commands.rs.
// Keep field names in sync with the Rust Serialize derives.

export type ProjectInfo = {
  root: string;
  name: string;
};

export type AppInfo = {
  project: ProjectInfo | null;
};

export type RichViolationCounts = {
  naming: number;
  placement: number;
  sequence: number;
};

export type RichCustomField = {
  key: string;
  // Tagged union: { type: "text", value: string } | { type: "integer", value: number }
  type: "text" | "integer";
  value: string | number;
};

export type RichSearchHit = {
  file_id: string;
  path: string;
  name: string | null;
  kind: string;
  ext: string | null;
  tags: string[];
  violations: RichViolationCounts;
  custom_fields: RichCustomField[];
};

export type ParseErrorPayload = {
  message: string;
  column: number | null;
};

export type SearchResponse = {
  query: string;
  hits: RichSearchHit[];
  warnings: string[];
  parse_error: ParseErrorPayload | null;
};

export type HistoryEntry = {
  query: string;
  ts: string; // RFC3339
};

export type RecentProject = {
  root: string;
  name: string;
  last_opened: string; // RFC3339
};

export type ViewDisplay = "list" | "grid";

export type View = {
  id: string;
  name: string;
  query: string;
  description?: string | null;
  group_by?: string | null;
  sort?: string | null;
  display: ViewDisplay;
};

export type DirEntryKind = "dir" | "file";

export type FileEntry = {
  file_id: string | null;
  kind: string;
  ext: string | null;
  tags: string[];
  violations: RichViolationCounts;
  custom_fields: RichCustomField[];
};

export type DirEntry = {
  name: string;
  path: string;
  kind: DirEntryKind;
  file?: FileEntry;
};

// IPC errors are plain strings on the JS side. The backend prefixes the
// no-project case with the discriminator `no_project:`; surface that as
// a typed flag so callers can branch without string-matching elsewhere.
export class IpcError extends Error {
  constructor(public readonly raw: string) {
    super(raw);
    this.name = "IpcError";
  }
  get isNoProject(): boolean {
    return this.raw.startsWith("no_project:");
  }
}

function toIpcError(e: unknown): IpcError {
  if (e instanceof IpcError) return e;
  if (typeof e === "string") return new IpcError(e);
  if (e instanceof Error) return new IpcError(e.message);
  return new IpcError(String(e));
}

export async function appInfo(): Promise<AppInfo> {
  try {
    return await invoke<AppInfo>("app_info");
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function searchExecute(query: string): Promise<SearchResponse> {
  try {
    return await invoke<SearchResponse>("search_execute", { query });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function searchHistoryList(): Promise<HistoryEntry[]> {
  try {
    return await invoke<HistoryEntry[]>("search_history_list");
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function searchHistoryClear(): Promise<void> {
  try {
    await invoke<void>("search_history_clear");
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function projectOpen(path: string): Promise<AppInfo> {
  try {
    return await invoke<AppInfo>("project_open", { path });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function projectRecentList(): Promise<RecentProject[]> {
  try {
    return await invoke<RecentProject[]>("project_recent_list");
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function projectRecentClear(): Promise<void> {
  try {
    await invoke<void>("project_recent_clear");
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function viewList(): Promise<View[]> {
  try {
    return await invoke<View[]>("view_list");
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function viewSave(view: View): Promise<void> {
  try {
    await invoke<void>("view_save", { view });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function viewDelete(id: string): Promise<void> {
  try {
    await invoke<void>("view_delete", { id });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function filesListDir(path: string): Promise<DirEntry[]> {
  try {
    return await invoke<DirEntry[]>("files_list_dir", { path });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function filesListAll(): Promise<RichSearchHit[]> {
  try {
    return await invoke<RichSearchHit[]>("files_list_all");
  } catch (e) {
    throw toIpcError(e);
  }
}

export type ExtensionSummary = {
  ext: string;
  count: number;
};

export async function extensionsCatalog(): Promise<ExtensionSummary[]> {
  try {
    return await invoke<ExtensionSummary[]>("extensions_catalog");
  } catch (e) {
    throw toIpcError(e);
  }
}

// --- project init ---------------------------------------------------------

export type InitPreview = {
  target_path: string;
  target_exists: boolean;
  is_existing_project: boolean;
  predicted_file_count: number | null;
  artifacts: string[];
  gitignore_exists: boolean;
};

export type InitResult = {
  project: ProjectInfo;
  scanned: number;
  added: number;
  orphan_metas: number;
};

export async function projectInitPreview(path: string): Promise<InitPreview> {
  try {
    return await invoke<InitPreview>("project_init_preview", { path });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function projectInitNew(parent: string, name: string): Promise<InitResult> {
  try {
    return await invoke<InitResult>("project_init_new", { parent, name });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function projectInitExisting(path: string, name: string | null): Promise<InitResult> {
  try {
    return await invoke<InitResult>("project_init_existing", { path, name });
  } catch (e) {
    throw toIpcError(e);
  }
}

// IpcError discriminator for "this directory already has a .progest/" — the
// backend prefixes the string so the UI can route to "open" instead of
// blocking the user with a flat error.
export function isAlreadyInitialized(e: unknown): boolean {
  return e instanceof IpcError && e.raw.startsWith("already_initialized:");
}

// --- accepts (directory inspector) ----------------------------------------

// Tagged-union mirror of `AcceptsTokenWire` in
// crates/progest-tauri/src/accepts_commands.rs. The backend serializes
// with `#[serde(tag = "type", rename_all = "lowercase")]`.
export type AcceptsToken = { type: "alias"; name: string } | { type: "ext"; value: string };

export type AcceptsMode = "strict" | "warn" | "hint" | "off";

export type RawAccepts = {
  inherit: boolean;
  exts: AcceptsToken[];
  mode: AcceptsMode;
};

export type EffectiveExt = {
  ext: string;
  source: "own" | "inherited";
};

export type EffectiveAccepts = {
  exts: EffectiveExt[];
  mode: AcceptsMode;
};

export type ChainEntry = {
  dir: string;
  accepts: RawAccepts;
};

export type AliasEntry = {
  name: string;
  exts: string[];
  builtin: boolean;
};

export type AcceptsReadResponse = {
  dir: string;
  own: RawAccepts | null;
  effective: EffectiveAccepts | null;
  chain: ChainEntry[];
  aliases: AliasEntry[];
  warnings: string[];
};

export async function acceptsRead(dir: string): Promise<AcceptsReadResponse> {
  try {
    return await invoke<AcceptsReadResponse>("accepts_read", { dir });
  } catch (e) {
    throw toIpcError(e);
  }
}

export async function acceptsWrite(dir: string, accepts: RawAccepts | null): Promise<void> {
  try {
    await invoke<void>("accepts_write", { dir, accepts });
  } catch (e) {
    throw toIpcError(e);
  }
}

// --- lint refresh ----------------------------------------------------------

export type LintRunResponse = {
  naming: number;
  placement: number;
  sequence: number;
  scanned: number;
};

export async function lintRun(): Promise<LintRunResponse> {
  try {
    return await invoke<LintRunResponse>("lint_run");
  } catch (e) {
    throw toIpcError(e);
  }
}
