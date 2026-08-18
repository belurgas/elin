# Roadmap

Elin is a desktop companion for Windows, macOS, and Linux: a matching Elixir+OTP pair, user PATH, the editor you already use, and a Mix workspace with a live module graph.

Mix already creates projects, fetches Hex, compiles, and tests. Elin does not replace it. The job is the layer Mix never productized: **rustup-style pairing** plus **a map of the project** a senior would draw after a week of reading.

Implementation order, acceptance criteria, and the agent contract live in [`PLAN.md`](PLAN.md). Dates are not promises. Order is.

---

## Now — workspace people will live in

The Studio window has to feel like an instrument, not a settings dump with extra screens.

1. **Graph — fluid, then the index** — 1a: 60fps pan/zoom on a real umbrella (cheap paint, LOD, no layout restart on every save). 1b: filters (lib, dirty, cycles, unwired), search that reaches the canvas, fit selection.
2. **Inspector as a briefing** — neighbours by edge kind, tests for this file, git pill, notes that stay secondary.
3. **Console as sessions** — start / stop / restart Mix tasks, one tab per job, Phoenix server included.
4. **Git you can read** — a real diff, then push when identity is set.
5. **Hex desk** — `mix.exs` vs lock, outdated, “why is this package here?”, update one dep.

This slice is how Elin stops being “the installer with extra screens”.

---

## Next — the map earns a command

`mix compile` tells you if it builds. Elin should tell you how the system is shaped — and CI should be able to say the same sentence.

6. **Scan on the graph** — cycles and `# elin:boundary` crossings as findings; Credo/format/Sobelow pins on nodes; stream into the console. Mix tools stay plugins. Elin does not reimplement Credo.
7. **`elin scan` as the habit** — JSON, exit codes a bot can trust, a GitHub Action example. Human output stays the default.

When this lands, `elin scan` on a Mix root should feel as obvious as `cargo clippy` on a crate — without claiming Mix is missing.

---

## Then — the clone is a working machine

8. **`elin.toml`** in the repo: pin, kits, scan defaults. Apply is explicit, never a silent rewrite of `mix.exs`. **Expert** as the recommended editor LSP; ElixirLS remains listed.

Windows, macOS, and Linux installers ship from the same tag. The first hour on a clean Windows box is a pride surface, not an afterthought.

---

## Later

- Docs as a signal (`@moduledoc` coverage vs the graph, open ExDoc from a node).
- AI on a leash: explain a module from the graph and `# elin:note`; never edit `mix.exs` except through the same buttons a human presses.
- Igniter / Phoenix kits with a file preview.
- Boundary compile errors on our graph when the kit is installed.
- Hex publish checklist — keys in the OS credential store.

---

## What we will not do

- Replace Mix, IEx, or ExUnit.
- Ship a full editor or a language server.
- Require an account, a cloud project, or a license server.
- Headline “Cargo for Elixir” as if Mix did not exist.
- Pretend a half-ported Unix build is cross-platform.

If a feature does not make Mix more honest or the first week on an inherited umbrella shorter, it waits.
