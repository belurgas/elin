# The interface (`src`)

This folder is what you see: windows, buttons, the graph, Settings.

Elin is a desktop app. The window is not a website, but the screens are still React. Rust (the folder next to this one, `src-tauri`) does the real work — install Elixir, talk to Mix, read git. This folder asks it questions and draws the answers.

## How a click becomes something on disk

1. You press a button on a page.
2. The page calls a function in `lib/api.ts` — a thin list of “please do X”.
3. That message crosses into Rust.
4. Rust does the job and sends text, a list, or an error back.
5. The page updates.

If something feels like “the UI is lying”, the bug is often in Rust. If a label is wrong, it is almost always here.

## The map

| Place | What it is for |
| --- | --- |
| `pages/` | Screens of the **main** window: Home, Install, Doctor, Projects, Hex, Settings… |
| `workspace/` | The **Studio** window you get when you open a Mix project: graph, git, hex, console, kits. |
| `components/` | Shared chrome: title bar, sidebar, tray flyout, toasts, **and the UI kit** (`components/ui.tsx`). |
| `i18n/` | Every sentence the user reads. One file per language. See [TRANSLATING.md](../TRANSLATING.md). |
| `lib/` | Talks to Rust (`api.ts`) and tiny helpers. |
| `state.tsx` | Remembers the current page, language, catalog, editors. |
| `types.ts` | Shared shapes: a project, a package, a kit. |
| `index.css` | Colors, the studio layout, the console. |
| `App.tsx` | Picks which shell to show: main app, studio, tray, or toast. |

`pages/studio/` is leftover pieces the **Projects** screen still uses (create dialog, project list). The live Studio workspace is `workspace/`.

## UI kit

Every form control lives in `components/ui.tsx`: `Button`, `Input`, `Textarea`, `Field`, `Checkbox`, `Menu`, `Popover`, `Modal`, `ProgressBar`, `Chip`, `Pill`. Release notes use `components/Markdown.tsx`. Pages and Studio import those — they do not invent a native checkbox, `<select>`, or a transparent dropdown. Cards may use the glass `surface` class; menus and modals stay opaque so text behind them stays hidden.

## Language

`i18n/en.ts` is English. `i18n/ru.ts` is Russian. `i18n/index.ts` holds the `locales` list — that list is the dropdown on the Settings page. Add a language there (and a matching dictionary file) and it shows up in the menu. Do not add a second picker in the title bar.

Changing language in Settings writes it to this PC and tells every open Elin window.

## If you are lost

- New screen in the main window → `pages/`, then a row in `App.tsx` and a label in `i18n/en.ts`.
- New control inside Studio → `workspace/`, still from the same kit.
- New sentence → `i18n/en.ts` first, then every other language file, or `tsc` will complain.
