import * as React from "react";
import { useTheme } from "next-themes";

import type { PaletteCommand } from "./types";

/**
 * Theme switcher commands. Three discrete `Set theme: …` entries
 * mirror what the TopBar icon button cycles through, so the user can
 * jump straight to a specific mode without click-cycling.
 *
 * The currently-active mode gets `hint: "active"` so the palette
 * shows a tiny indicator next to it.
 */
export function useThemeCommands(): PaletteCommand[] {
  const { theme, setTheme } = useTheme();
  return React.useMemo<PaletteCommand[]>(
    () => [
      {
        id: "theme.system",
        title: "Set theme: System",
        group: "Theme",
        hint: theme === "system" ? "active" : undefined,
        keywords: ["auto", "os"],
        run: () => setTheme("system"),
      },
      {
        id: "theme.light",
        title: "Set theme: Light",
        group: "Theme",
        hint: theme === "light" ? "active" : undefined,
        run: () => setTheme("light"),
      },
      {
        id: "theme.dark",
        title: "Set theme: Dark",
        group: "Theme",
        hint: theme === "dark" ? "active" : undefined,
        run: () => setTheme("dark"),
      },
    ],
    [theme, setTheme],
  );
}
