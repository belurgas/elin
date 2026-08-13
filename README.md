



<p align="center">
  <img src="docs/banner.png" alt="Elin — Elixir on Windows" width="100%" />
</p>

# Elin

**The Windows companion Elixir never shipped.**  
Installer. Version picker. Editors. Studio. CLI. Same binary.

[Install](#install) · [App](#the-app) · [Studio](#studio) · [CLI](#cli) · [Translate](TRANSLATING.md) · [Roadmap](ROADMAP.md) · [Issues](#issues)



---

Elixir on a clean Windows machine is still a scavenger hunt. Language site. Zip. Then Erlang — a *compatible* Erlang, not the first GitHub asset you saw. Then PATH. Then Hex. Then which of twelve VS Code extensions is the real one. Then Mix cannot find `erl.exe` and you have burned twenty minutes.

Elin installs a matching Elixir + OTP pair, finds the editor you already paid for, writes user PATH without admin, and opens the Mix project in a studio with a live module graph.

UI in English and Russian. No account. The only network calls are Hex Bob, GitHub OTP releases, and hex.pm.



---



## Install

**Windows 10 / 11 x64.** That is the supported target. macOS and Linux are on the [roadmap](ROADMAP.md), not in this tree.

1. Open **[Releases](../../releases)** and grab the latest **NSIS** `.exe` (or `.msi` if you prefer WiX).
2. Install for the current user. No admin prompt.
3. Launch Elin. If this PC has no Elixir yet, hit **Install recommended**.
4. Optional: **Add to PATH**, then open a *new* terminal and run `elixir -v`.

From source, if you are hacking on Elin itself:

```bash
git clone https://github.com/belurgas/elin.git
cd elin
npm install
npm run tauri dev
```

Local installer:

```bash
npm run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/` (or under `x86_64-pc-windows-msvc` when you pass `--target`).

---



## The app

A desktop companion for Mix on Windows. Versions, PATH, editors, Hex, and the first project — in one window.

### Pairing brain

Nothing is hardcoded as “latest”. On refresh Elin:

1. Reads `[builds.hex.pm/builds/elixir/builds.txt](https://builds.hex.pm/builds/elixir/builds.txt)` — the same index asdf / mise / the official installer use.
2. Asks GitHub for OTP releases that actually ship `otp_win64_*.zip`.
3. Takes the newest stable Elixir, then the newest **compatible** OTP major that has a Windows asset.

Elixir 1.20 will not land on OTP 24. The official compatibility table is the law.

GitHub rate-limit? Disk cache, then the OTP majors already listed next to Elixir on Hex.

```
OTP     https://github.com/erlang/otp/releases/download/OTP-{ver}/otp_win64_{ver}.zip
Elixir  https://builds.hex.pm/builds/elixir/v{ver}-otp-{major}.zip
```

Layout on disk matches `~/.elixir-install/installs`. Pin a pair globally, or pin one Mix project to its own Elixir.



### Editors

**Studio Scout** finds VS Code, Cursor, VSCodium, Windsurf, IntelliJ, WebStorm, Neovim, Zed, Sublime, Emacs, Helix — real icons, real paths. Pick a default on the Studios page. Graph, notes, findings, and “Open in editor” all use that one.

Plugins that are not a 2019 blog post: ElixirLS, snippets, Credo, JetBrains Elixir, elixir-tools.nvim, Zed / Emacs / Sublime. One-click install for the VS Code family.

### Doctor, PATH, first project

- **Doctor** — Elixir, `erl`, Mix, Hex, Git, VC++ runtime, managed installs. One button when the binary is on disk but missing from a new console.
- **PATH surgeon** — user PATH only (`HKCU\Environment`). Switching the active pair rewrites Elin-managed entries. No admin.
- **Projects** — `mix new`, Mix+supervisor, Phoenix, Phoenix LiveView. Kits (Credo, Sobelow, …) written into `mix.exs` without stomping a file you already tuned.
- **Hex Radar** — search, downloads, docs. In Studio, add/remove the dep and run `mix deps.get`.
- **Playground** — snippet on disk, 8 second cap, 16 KB max. Nothing uploaded.





---



## Studio

Open a Mix app from Projects. You get a workspace window, not a settings page with extra steps.

### Graph

Modules as a force layout. Roles have their own color: GenServer, LiveView, supervisor, schema, router, test. Git-dirty nodes get a ring.

Right-click a node **or** a row in the module tree:

- open in the default editor (file + line)
- copy the module name or path
- reveal the folder in Explorer
- focus the graph

Nested module tree on the left. Inspector on the right. Notes are `# elin:note` in the source — click a note, jump to the file. The graph stays mounted when you switch tabs, so it does not explode across the canvas every time you come back.

### Hex, Git, Quality, Console

- **Hex** — packages already in `mix.exs` as chips. Search hex.pm beside them. Add / remove without leaving the graph.
- **Git** — changed files only, as a directory tree. Statuses in English (`new` / `edited` / `deleted`). Select all, commit. Init + `.gitignore` + license if the folder is not a repo yet.
- **Quality** — Credo, format, Hex audit, MixAudit, Sobelow, Dialyzer, docs coverage. Scan writes a report; kits patch `mix.exs`.
- **Console** — multiple tabs. Type `mix test`, `git status`, `elin -h` yourself. Mix streams into the tab instead of waiting for the process to die.





---



## CLI

Same binary as the GUI. `elin` with no args opens (or focuses) the app. With args, it is a toolchain:

```
elin                 open the app
elin add [path]      remember this Mix project
elin list            remembered projects
elin open [path]     open the Studio workspace
elin scan [path]     modules, git, enabled Mix tools
elin scan --full     also Dialyzer and tests
elin format [path]   mix format   (--check to report only)
elin kit list
elin kit add credo
elin kit remove credo
elin status [path]   pin, branch, dirty files
elin path            put this elin.exe on the user PATH
```

After install, **Elin → add to PATH** (or `elin path`) and a new terminal can call `elin scan` from any Mix root.

---

## Translate

English and Russian ship in the app. Anyone can add another language.

Settings in the main window has a language dropdown — it applies to Studio, the tray, and toasts, not only that screen.

The how-to (copy a file, register it, open a PR, keep it merged) is [TRANSLATING.md](TRANSLATING.md).

---

## Issues

Something broke on your machine? Open a **Bug** issue.

Want a command, a graph trick, a kit? Open a **Feature** issue.

Templates ask for Windows version, Elin version, and what you clicked. A Doctor screenshot or `elin status` saves a round trip.

Do not paste secrets from `.env` or production `mix.exs` deploy keys.

---



## Repo map

```
src/                 What you see (pages, Studio, i18n). See src/README.md
src-tauri/           The engine (install, PATH, Mix, git). See src-tauri/README.md
src/i18n/            One file per language. See TRANSLATING.md
scripts/gen-brand.py Icons + installer bitmaps
```

```bash
npm run test:rust    # cargo test --lib
npx tsc --noEmit
npm run brand        # regenerate icons / banner
```

---



## Security, short

- Network: Hex Bob, GitHub OTP releases, hex.pm. Playground code stays on disk.
- PATH edits are current-user only.
- Zip extract uses `enclosed_name()` (no zip-slip).
- Studio shell runs in the project folder with the pinned toolchain. Treat it like a terminal.

---



## License

MIT. Elixir and Erlang are trademarks of their holders. Elin is an independent companion, not an official installer.