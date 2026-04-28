import * as React from "react";

import { useProject } from "@/lib/project-context";

import type { PaletteCommand } from "./types";

/**
 * Project-scoped commands: open a fresh project via the native
 * folder picker, or jump straight into one of the recent entries.
 *
 * The recent list is dynamic — entries are emitted as commands so
 * `> open recent <name>` can fuzzy-match by either the project's
 * display name or its absolute root path.
 */
export function useProjectCommands(): PaletteCommand[] {
  const { recent, openPicker, pickRecent, openInitDialog } = useProject();
  return React.useMemo<PaletteCommand[]>(() => {
    const cmds: PaletteCommand[] = [
      {
        id: "project.open",
        title: "Open project…",
        group: "Project",
        hint: "folder picker",
        keywords: ["folder", "picker", "directory"],
        run: () => {
          void openPicker();
        },
      },
      {
        id: "project.init.new",
        title: "New project…",
        group: "Project",
        hint: "create a fresh folder",
        keywords: ["create", "init", "fresh", "scaffold"],
        run: () => {
          openInitDialog("new");
        },
      },
      {
        id: "project.init.existing",
        title: "Initialize existing folder…",
        group: "Project",
        hint: "init in place",
        keywords: ["init", "existing", "in-place", "convert"],
        run: () => {
          openInitDialog("existing");
        },
      },
    ];
    for (const entry of recent) {
      const display = entry.name || entry.root;
      cmds.push({
        id: `project.recent:${entry.root}`,
        title: `Open recent: ${display}`,
        group: "Recent projects",
        hint: entry.root,
        keywords: [entry.root, entry.name].filter(Boolean),
        run: () => {
          void pickRecent(entry);
        },
      });
    }
    return cmds;
  }, [recent, openPicker, pickRecent, openInitDialog]);
}
