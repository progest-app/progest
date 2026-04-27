# Command Palette

Cmd+K でコマンドパレット起動。2 モード:

| プレフィックス | モード | 説明 |
| --- | --- | --- |
| なし | 検索 | 検索 DSL（`docs/SEARCH_DSL.md`）でファイル検索 |
| `>` | コマンド | アプリケーションコマンド実行（VS Code 風） |

`>` を入力するとコマンドモード、削除すると検索モードに戻る。

最終更新: 2026-04-27（初版、`>` プレフィックス + Project / Theme コマンド初期セット）

---

## 1. 利用可能なコマンド一覧

**新コマンドを追加した PR は本セクションも同時更新する**（拡張性方針）。

### 1.1 Project（`useProjectCommands`）

| Title | Hint | 動作 |
| --- | --- | --- |
| `Open project…` | `folder picker` | Tauri folder picker → `project_open(path)` → AppState 差替え |
| `Open recent: <name>` | `<absolute path>` | recent project エントリを即時 attach（ピッカー不要）。recent ごとに動的生成、`pickRecent(entry)` 経由 |

### 1.2 Theme（`useThemeCommands`）

| Title | Hint | 動作 |
| --- | --- | --- |
| `Set theme: System` | `active`（現在モード時） | `setTheme("system")` |
| `Set theme: Light` | 同上 | `setTheme("light")` |
| `Set theme: Dark` | 同上 | `setTheme("dark")` |

`next-themes` が localStorage（key: `progest:theme`）に永続化、OS 追従は system モード時のみ。

---

## 2. 設計

### 2.1 ファイル構成

```
app/src/lib/palette-commands/
├── types.ts            # PaletteCommand interface + fuzzyMatch
├── project-commands.ts # useProjectCommands
├── theme-commands.ts   # useThemeCommands
└── index.ts            # usePaletteCommands aggregator
```

`<CommandPalette>`（`app/src/components/command-palette.tsx`）が `usePaletteCommands()` を呼んで集約結果を `>` モードに表示。

### 2.2 拡張方法

新カテゴリのコマンドを追加する手順:

1. `app/src/lib/palette-commands/<topic>-commands.ts` を新規作成、`use<Topic>Commands(): PaletteCommand[]` を export。
2. `index.ts` の `usePaletteCommands()` で import + concat。
3. **本ドキュメント §1 にテーブル追加**。

`PaletteCommand` インターフェース:

```ts
interface PaletteCommand {
  id: string;            // 安定 ID（例: `project.open`）。cmdk `value` として使用
  title: string;         // 表示名
  group?: string;        // セクションヘッダー
  hint?: string;         // 右側補助ラベル
  keywords?: string[];   // 追加 fuzzy 一致語
  run: () => void | Promise<void>;
}
```

`run()` 内のエラーハンドリングは各コマンドの責任（パレットは catch しない）。破壊的操作を追加する場合は確認 Dialog を `run` 内で開くこと。

### 2.3 フィルタ

`>` 後の文字列を AND-match で全 needle が title / id / keywords のいずれかに含まれるかチェック（`fuzzyMatch` in `types.ts`）。cmdk の built-in filter は使わない（検索モードと同じ `shouldFilter={false}` を流用するため）。

---

## 3. 将来計画（v1.x 候補）

- `Reload window` / `Quit` 等のシステムコマンド（Tauri webview API 経由）
- `Lint current project` / `Reconcile now` 等の core 操作起動
- `Tag add: <name>` / `Tag remove: <name>` — 選択ファイルとの組合せ。コマンドパレット単独では難しいので、`tag.toml` の既存 tag を fuzzy 提示する形に
- `Reveal in Finder` for selected hit（コマンドパレット内検索結果クリック時の選択状態を引き継ぐ）
- 確認 Dialog 付き破壊的コマンド（Clear recent projects / Clear search history 等）

追加時は必ず本ドキュメント §1 を更新。
