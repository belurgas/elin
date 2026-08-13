# The engine (`src-tauri`)

This folder is the part of Elin that actually touches your machine.

The screens you click live in `src/` (React). They cannot install OTP, edit PATH, or run Mix by themselves. They send a request here. This side is Rust, packed as a Tauri app — a small desktop wrapper around that Rust code, plus the webview that draws the UI.

Think of it as a workshop behind a shop window. The window is pretty. The workshop has the tools.

## What starts when you double-click Elin

`src/main.rs` looks at the command line.

- No arguments → open the app (or focus it if it is already running).
- Arguments like `elin scan` → skip the window, print to a console, exit.

Same `elin.exe` either way.

## The map

| Place | In plain language |
| --- | --- |
| `src/cli.rs` | The commands you type: `elin scan`, `elin kit add credo`, `elin path`. |
| `src/commands/` | The doorbell the UI rings. Each function is one job the React side can ask for. |
| `src/services/` | The work itself: download, unzip, PATH, git, Mix, Hex, the module graph. |
| `src/domain/` | Version numbers and “does this Elixir run on that OTP?”. No files, no network. |
| `src/desktop.rs` | Tray icon and the little toast window. |
| `src/instance.rs` | Makes sure a second Elin focuses the first one instead of opening a twin. |
| `tauri.conf.json` | Window size, installer (NSIS / MSI), icons. |
| `icons/` and `windows/` | Artwork for the app and the Windows installer. |

## Inside `services/` (the workshop benches)

You do not need to know all of these. This is the “where do I look?” list.

| File | Job |
| --- | --- |
| `catalog.rs` | Asks Hex and GitHub which Elixir / OTP versions exist. |
| `install.rs` | Downloads a pair and unpacks it next to `~/.elixir-install`. |
| `env.rs` | Reads and writes **user** PATH. Never asks for admin. |
| `probe.rs` | “Is `elixir` actually callable from a new terminal?” |
| `studios.rs` / `plugins.rs` | Finds editors and the Elixir plugins they understand. |
| `projects.rs` | Remembers Mix apps, pins, `mix.exs` facts. |
| `analyze.rs` | Walks `lib/` and `test/`, builds the module graph. No Mix compile. |
| `mixcmd.rs` / `winproc.rs` | Runs `mix.bat` / `elixir.bat` on Windows without quote hell. |
| `mixexs.rs` | Inserts or removes a dep line in `mix.exs`. |
| `kits.rs` | Credo, Sobelow, and friends: the dep plus a starter config. |
| `git.rs` | Status, commit. No push. |
| `hexpm.rs` | Search and package details from hex.pm. |
| `workspace.rs` | The Studio window: which folder it is, add/remove Hex, Mix tasks. |
| `watch.rs` | Notices when files change so the graph can refresh. |
| `doctor.rs` | The Doctor page’s checklist and one-click fixes. |
| `store.rs` / `cache.rs` | Durable data vs throwaway downloads. Clearing cache must not forget your projects. |

## Talking to the UI

`commands/` should stay thin. If a function grows into real logic, it belongs in `services/`. That way `elin scan` (CLI) and the Studio scan button can share the same code.

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

They do not install Elixir. They check parsers, PATH helpers, mix.exs edits, the graph on fixture snippets.
