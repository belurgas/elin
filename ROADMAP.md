# Roadmap

Elin today is a Windows companion: install a compatible pair, find an editor, open a Mix app, see the graph.

The job after that is larger.

**Be for Elixir what Cargo is for Rust — except the job is harder, so the product has to be sharper.** Not a clone of `mix`. A layer above it that understands the project the way a senior does after a week of reading.

Dates are not promises. Order is.

---

## Now — Studio that does not flinch

The workspace window is the product people will live in. It has to feel like an IDE panel, not a settings dump.

- Graph that stays put, fits the canvas, and still reads on a 400-module Phoenix app (collapse by boundary, search, “only dirty”, “only cycles”).
- Default editor everywhere: graph, notes, findings, git paths.
- Console as a real session: history, restart, kill, one tab per Mix task if you want it.
- Git that shows a diff, not only a file list. Push when identity is set.
- Hex in the project: lock vs `mix.exs`, outdated, retired, advisories — without a round trip to the browser.
- Quality scan that streams into the console and pins findings on the graph.

This slice is how Elin stops being “the installer with extra screens”.

---

## Next — Analyzer that earns the Cargo comparison

`mix compile` tells you if it builds. Elin should tell you how the system is shaped.

Cargo’s trick is not “it compiles”. It is that `cargo test`, `cargo clippy`, `cargo tree`, and the crate graph feel like one product. That is the bar — for Elixir.

- Module graph is the index, not a toy: aliases, `use`, `import`, `defdelegate`, calls, behaviours, supervision tree where we can see it.
- Boundaries (`# elin:boundary core|ui|data`) as first-class edges — violations as findings, not comments you forget.
- Cycles, unused, unwired, fan-in hotspots, “this file changed and these tests should run”.
- Git overlay: what the last commit actually touched in the graph.
- `elin scan` / `elin scan --full` as the CLI contract CI can run. JSON output. Exit codes a human and a bot can both trust.
- Mix tools as plugins of the scan (Credo, format, Sobelow, MixAudit, Dialyzer, coverage) — Elin orchestrates. It does not reimplement Credo.

When this lands, `elin scan` on a Mix root should feel as obvious as `cargo clippy` on a crate.

---

## Libraries — a Hex desk, not a search box

Hex Radar is a start. The desk is:

- Everything in `mix.exs` + `mix.lock`, with the spec you wrote vs the version you actually got.
- Outdated / retired / advisory in one list, with “update this one” that writes the tuple and runs `deps.get`.
- Transitive tree you can fold. “Why is `jason` here?”
- A local watchlist: packages you care about across projects (the ones you maintain, the ones that scare you).
- Publish checklist later — Hex API keys never in the repo, always in the OS credential store.

---

## Documentation as a signal, not a folder

Elixir already has `@moduledoc`, `@doc`, typespecs, ExDoc, Doctor. Elin should treat missing docs like missing tests: visible, ranked, not a lecture.

- Coverage of public modules vs the graph (the undocumented core is worse than an undocumented helper).
- Stale docs: the `@doc` still talks about a function you renamed.
- “Open ExDoc for this module” from the graph.
- A studio panel that is a map of the public API, not a second copy of the markdown.

---

## AI, on a leash

No chatbot wallpaper. No “ask GPT to rewrite the project”.

Useful, local-first jobs:

- Explain this module in one paragraph, grounded in the graph and the `# elin:note` you already wrote.
- “What breaks if I move `Accounts` behind this boundary?”
- Draft a Credo explanation or a commit message from the staged diff.
- Map a Hex advisory onto the modules that call the package.

The model does not get to edit `mix.exs` unless you press the same buttons a human presses. Elin stays the source of truth; the model is a reader with a mouth.

---

## Later — the rest of the map

- **macOS / Linux** installers, same catalog, same studio.
- **Umbrella / mix.exs paths** that are not a single app.
- **Phoenix generators** as first-class kits (auth, live, presence) with a preview of files before write.
- **Remote OTP/Elixir mirrors** for people who cannot hit GitHub.
- **Team defaults**: a checked-in `elin.toml` (pin, kits, boundaries, default editor family) so a clone is a working machine.

---

## What we will not do

- Replace Mix, IEx, or ExUnit.
- Ship a full editor. Elin opens yours.
- Require an account, a cloud project, or a license server.
- Pretend a Windows-only 0.1 is “cross-platform” in the README.

If a feature does not make `mix` more honest or the first week on Elixir shorter, it waits.
