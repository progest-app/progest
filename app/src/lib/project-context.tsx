import * as React from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

import {
  appInfo,
  IpcError,
  projectOpen,
  projectRecentClear,
  projectRecentList,
  type ProjectInfo,
  type RecentProject,
} from "@/lib/ipc";

type ProjectContextValue = {
  project: ProjectInfo | null;
  recent: RecentProject[];
  error: string | null;
  /** Re-probe the backend for the currently attached project. */
  refresh: () => Promise<void>;
  /** Native folder picker → project_open. No-op when the user cancels. */
  openPicker: () => Promise<void>;
  /** Open one of the recent entries (skips the picker). */
  pickRecent: (entry: RecentProject) => Promise<void>;
  clearRecent: () => Promise<void>;
  /**
   * Monotonic counter bumped whenever indexed state (violations, tags,
   * search projection) may have changed out-of-band — e.g. the
   * directory inspector ran `lint_run` after an `[accepts]` save.
   * Long-lived consumers (FlatView, TreeView) include this in their
   * effect deps so they re-fetch.
   */
  refreshTick: number;
  /** Bump [`refreshTick`]. */
  bumpRefresh: () => void;
};

const Ctx = React.createContext<ProjectContextValue | null>(null);

export function ProjectProvider({ children }: { children: React.ReactNode }) {
  const [project, setProject] = React.useState<ProjectInfo | null>(null);
  const [recent, setRecent] = React.useState<RecentProject[]>([]);
  const [error, setError] = React.useState<string | null>(null);
  const [refreshTick, setRefreshTick] = React.useState(0);
  const bumpRefresh = React.useCallback(() => {
    setRefreshTick((n) => n + 1);
  }, []);

  const refresh = React.useCallback(async () => {
    try {
      const info = await appInfo();
      setProject(info.project);
      setError(null);
    } catch (e) {
      setProject(null);
      setError(e instanceof IpcError ? e.raw : String(e));
    }
  }, []);

  const refreshRecent = React.useCallback(async () => {
    try {
      setRecent(await projectRecentList());
    } catch (e) {
      console.warn("recent projects", e);
    }
  }, []);

  React.useEffect(() => {
    void refresh();
    void refreshRecent();
  }, [refresh, refreshRecent]);

  const attach = React.useCallback(
    async (path: string) => {
      try {
        const info = await projectOpen(path);
        setProject(info.project);
        setError(null);
        await refreshRecent();
      } catch (e) {
        setError(e instanceof IpcError ? e.raw : String(e));
      }
    },
    [refreshRecent],
  );

  const openPicker = React.useCallback(async () => {
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        title: "Open Progest project",
      });
      if (typeof picked !== "string") return;
      await attach(picked);
    } catch (e) {
      setError(e instanceof IpcError ? e.raw : String(e));
    }
  }, [attach]);

  const pickRecent = React.useCallback(
    async (entry: RecentProject) => {
      await attach(entry.root);
    },
    [attach],
  );

  const clearRecent = React.useCallback(async () => {
    try {
      await projectRecentClear();
      setRecent([]);
    } catch (e) {
      setError(e instanceof IpcError ? e.raw : String(e));
    }
  }, []);

  const value = React.useMemo<ProjectContextValue>(
    () => ({
      project,
      recent,
      error,
      refresh,
      openPicker,
      pickRecent,
      clearRecent,
      refreshTick,
      bumpRefresh,
    }),
    [
      project,
      recent,
      error,
      refresh,
      openPicker,
      pickRecent,
      clearRecent,
      refreshTick,
      bumpRefresh,
    ],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useProject(): ProjectContextValue {
  const v = React.useContext(Ctx);
  if (!v) throw new Error("useProject() outside ProjectProvider");
  return v;
}
