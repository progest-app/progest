import "./App.css";

import { CommandPalette } from "@/components/command-palette";

export function App() {
  return (
    <main className="app">
      <CommandPalette />
      <header>
        <h1>Progest</h1>
        <p>
          Press <kbd>⌘K</kbd> to search.
        </p>
      </header>
    </main>
  );
}
