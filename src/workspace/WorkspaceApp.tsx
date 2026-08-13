import { useEffect, useRef, useState } from "react";
import { ChevronDown, Flame, GitBranch, Package, Play, Share2, ShieldCheck } from "lucide-react";
import { api, onMixLine, onWorkspaceFs } from "../lib/api";
import { useApp } from "../state";
import { Titlebar } from "../components/Titlebar";
import { Button } from "../components/ui";
import { ForceGraph } from "./ForceGraph";
import { HexAdd } from "./HexAdd";
import { ModuleTree } from "./ModuleTree";
import { ConsoleDock, tabTitle, type TermSession } from "./Console";
import { Inspector } from "./Inspector";
import { QualityStudio } from "./QualityStudio";
import { GitRail, GitStudio } from "./GitStudio";
import { Overview } from "../pages/studio/Overview";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { cn } from "../lib/cn";
import type { ElinComment, GitSnapshot, GraphNode, KitStatus, MixProject, ModuleGraph, ScanReport } from "../types";

type Stage = "graph" | "hex" | "git" | "quality" | "elixir";

function newTerm(id: string, title = "shell"): TermSession {
  return { id, title, lines: [], running: false, ok: null, task: null };
}

export function WorkspaceApp({ projectPath }: { projectPath: string }) {
  const { t, studios, preferredStudioId, toolchains, refreshToolchains, ensureStudios } = useApp();
  const [project, setProject] = useState<MixProject | null>(null);
  const [graph, setGraph] = useState<ModuleGraph | null>(null);
  const [git, setGit] = useState<GitSnapshot | null>(null);
  const [kits, setKits] = useState<KitStatus[]>([]);
  const [picked, setPicked] = useState<GraphNode | null>(null);
  const [stage, setStage] = useState<Stage>("graph");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<ScanReport | null>(null);
  const [commitMsg, setCommitMsg] = useState("");
  const [commitFiles, setCommitFiles] = useState<string[]>([]);
  const [consoleOpen, setConsoleOpen] = useState(true);
  const [terms, setTerms] = useState<TermSession[]>(() => [newTerm("t1", "mix")]);
  const [activeTerm, setActiveTerm] = useState("t1");
  const [live, setLive] = useState(false);
  const [licenses, setLicenses] = useState<Array<{ id: string; name: string }>>([]);
  const [license, setLicense] = useState("MIT");
  const [menu, setMenu] = useState<{ x: number; y: number; node: GraphNode } | null>(null);
  const lineId = useRef(0);
  const termSeq = useRef(1);
  const quietUntil = useRef(0);
  const editors = studios.filter((s) => s.detected && (s.cli || s.executable));
  const [runCmd, setRunCmd] = useState(() => localStorage.getItem(`elin.run.${projectPath}`) ?? "");
  const [runOpen, setRunOpen] = useState(false);
  const runMenu = useRef<HTMLDivElement>(null);
  const primary = editors.find((s) => s.id === preferredStudioId) ?? editors[0];

  function patchTerm(id: string, fn: (s: TermSession) => TermSession) {
    setTerms((all) => all.map((s) => (s.id === id ? fn(s) : s)));
  }

  async function reloadGraph() {
    const g = await api.graph(projectPath);
    setGraph(g);
    setPicked((current) => (current ? g.nodes.find((n) => n.id === current.id) ?? current : current));
  }

  async function reload(path = projectPath) {
    const next = await api.inspectProject(path);
    setProject(next);
    const [g, snap, k] = await Promise.all([api.graph(path), api.projectGit(path), api.listKits(path)]);
    setGraph(g);
    setGit(snap);
    setKits(k);
  }

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const next = await api.inspectProject(projectPath);
        if (cancelled) return;
        setProject(next);
        const [snap, k, lic] = await Promise.all([
          api.projectGit(projectPath),
          api.listKits(projectPath),
          api.gitLicenses(),
        ]);
        if (cancelled) return;
        setGit(snap);
        setKits(k);
        setLicenses(lic);
        const g = await api.graph(projectPath);
        if (!cancelled) setGraph(g);
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    })();
    void ensureStudios();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectPath]);

  useEffect(() => {
    void api.watchStart(projectPath).then(() => setLive(true)).catch(() => setLive(false));
    return () => {
      void api.watchStop(projectPath);
    };
  }, [projectPath]);

  useEffect(() => {
    let unFs: (() => void) | undefined;
    let unMix: (() => void) | undefined;
    void onWorkspaceFs((tick) => {
      if (Date.now() < quietUntil.current) return;
      if (tick.path.replace(/\//g, "\\").toLowerCase() !== projectPath.replace(/\//g, "\\").toLowerCase()) return;
      if (tick.graph) void reloadGraph().catch(() => undefined);
      if (tick.git) void api.projectGit(projectPath).then(setGit).catch(() => undefined);
      if (tick.lock) void api.listKits(projectPath).then(setKits).catch(() => undefined);
    }).then((fn) => {
      unFs = fn;
    });
    void onMixLine((payload) => {
      patchTerm(payload.session, (s) => {
        const last = s.lines[s.lines.length - 1];
        if (last?.text === payload.line) return s;
        return { ...s, lines: [...s.lines.slice(-500), { id: ++lineId.current, text: payload.line }] };
      });
    }).then((fn) => {
      unMix = fn;
    });
    return () => {
      unFs?.();
      unMix?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectPath]);

  useEffect(() => {
    if (!runOpen) return;
    const onDoc = (event: MouseEvent) => {
      if (!runMenu.current?.contains(event.target as Node)) setRunOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [runOpen]);

  async function run(task: string) {
    const cmd = task === "format.check" ? "mix format --check-formatted" : `mix ${task}`;
    await runShell(activeTerm, cmd);
  }

  async function runShell(id: string, command: string) {
    setConsoleOpen(true);
    setActiveTerm(id);
    setBusy(true);
    setError(null);
    patchTerm(id, (s) => ({
      ...s,
      running: true,
      ok: null,
      task: command,
      title: tabTitle(command),
      lines: [...s.lines, { id: ++lineId.current, text: command, kind: "in" as const }],
    }));
    try {
      await api.projectShell(projectPath, id, command);
      patchTerm(id, (s) => ({ ...s, running: false, ok: true }));
      if (/\bdeps\.(get|unlock)\b/.test(command)) await reload();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      patchTerm(id, (s) => {
        const streamed = s.lines.some((l) => l.kind !== "in" && l.text.trim());
        if (streamed) return { ...s, running: false, ok: false };
        return {
          ...s,
          running: false,
          ok: false,
          lines: [
            ...s.lines,
            ...msg.split(/\r?\n/).map((text) => ({ id: ++lineId.current, text: text || " " })),
          ],
        };
      });
    } finally {
      setBusy(false);
    }
  }

  async function runScan(full: boolean) {
    setBusy(true);
    setError(null);
    setStage("quality");
    try {
      const next = await api.projectScan(projectPath, full, true);
      setReport(next);
      setGraph(next.graph);
      setGit(next.git);
      setKits(next.kits);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  function selectNode(node: GraphNode) {
    setPicked(node);
    setStage("graph");
  }

  function openNode(node: GraphNode, line?: number) {
    if (!primary) {
      setError(t.workspace.openEditor);
      return;
    }
    if (project) {
      void api.openInStudio(primary, project.path, node.path, line).catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
      });
    }
  }

  function dirOf(file?: string | null) {
    if (!file) return projectPath.replace(/\//g, "\\");
    const norm = file.replace(/\//g, "\\");
    const abs = /^[a-zA-Z]:\\/.test(norm) || norm.startsWith("\\\\");
    const full = abs ? norm : `${projectPath.replace(/\//g, "\\")}\\${norm}`;
    const cut = full.lastIndexOf("\\");
    return cut > 0 ? full.slice(0, cut) : projectPath.replace(/\//g, "\\");
  }

  function menuItems(node: GraphNode): MenuItem[] {
    return [
      { kind: "item", label: t.workspace.openEditor, onClick: () => openNode(node), muted: !primary },
      { kind: "item", label: t.workspace.copyModule, onClick: () => void navigator.clipboard.writeText(node.id) },
      { kind: "item", label: t.workspace.copyPath, onClick: () => void navigator.clipboard.writeText(node.path ?? node.id) },
      { kind: "item", label: t.workspace.openFolder, onClick: () => void api.openPath(dirOf(node.path)) },
      { kind: "sep" },
      { kind: "item", label: t.workspace.focusGraph, onClick: () => selectNode(node) },
    ];
  }

  function goToComment(comment: ElinComment) {
    const node = graph?.nodes.find(
      (n) => n.path && n.path.replace(/\\/g, "/").toLowerCase() === comment.file.replace(/\\/g, "/").toLowerCase(),
    );
    if (node) selectNode(node);
    if (primary) {
      void api.openInStudio(primary, projectPath, comment.file, comment.line).catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
      });
    }
  }

  const stages: Array<[Stage, typeof Share2, string]> = [
    ["graph", Share2, t.workspace.graph],
    ["hex", Package, t.hex.title],
    ["git", GitBranch, "Git"],
    ["quality", ShieldCheck, t.workspace.quality],
    ["elixir", Flame, t.workspace.elixir],
  ];
  const showRail = stage === "graph" || stage === "git";

  return (
    <div className="aurora flex h-full flex-col">
      <div className="grain" aria-hidden />
      <Titlebar heading={project?.name ?? t.workspace.title} caption={projectPath} />
      <div className="flex min-h-0 flex-1">
        <nav className="studio-activity">
          {stages.map(([id, Icon, label]) => (
            <button
              key={id}
              type="button"
              title={label}
              onClick={() => setStage(id)}
              className={cn("studio-activity-btn", stage === id && "is-active")}
            >
              <Icon size={18} />
              <span>{label}</span>
            </button>
          ))}
        </nav>
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-white/8 px-3 py-1.5">
            <div ref={runMenu} className="relative inline-flex">
              <div className="inline-flex overflow-hidden rounded-md">
                <Button
                  size="sm"
                  disabled={busy}
                  className="rounded-none"
                  onClick={() => void runShell(activeTerm, startCommand(project, runCmd))}
                >
                  <Play size={12} />
                  {t.workspace.run}
                </Button>
                <Button
                  size="sm"
                  disabled={busy}
                  title={t.workspace.runCustom}
                  className="rounded-none border-l border-white/20 px-1.5"
                  onClick={() => setRunOpen((v) => !v)}
                >
                  <ChevronDown size={12} className={cn("transition", runOpen && "rotate-180")} />
                </Button>
              </div>
              {runOpen ? (
                <div className="absolute left-0 top-full z-30 mt-1 w-80 rounded-lg border border-white/10 bg-ink-800 p-2 shadow-xl">
                  <input
                    className="field w-full font-mono text-[12px]"
                    value={runCmd}
                    placeholder={startCommand(project, "")}
                    onChange={(e) => {
                      setRunCmd(e.target.value);
                      localStorage.setItem(`elin.run.${projectPath}`, e.target.value);
                    }}
                  />
                  <p className="mt-1.5 text-[10px] leading-4 text-mist-300">{t.workspace.runCustom}</p>
                </div>
              ) : null}
            </div>
            <Button size="sm" disabled={busy} onClick={() => void run("compile")}>
              {t.workspace.compile}
            </Button>
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run("test")}>
              {t.workspace.test}
            </Button>
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run("format")}>
              {t.projects.formatFix}
            </Button>
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run("deps.get")}>
              {t.workspace.depsGet}
            </Button>
            <div className="ml-auto flex flex-wrap items-center gap-2">
              <span className={cn("studio-live", live && "is-on")}>{live ? t.workspace.live : t.workspace.liveOff}</span>
              {git?.branch ? <span className="font-mono text-[11px] text-elixir-300">{git.branch}</span> : null}
              {primary && project ? (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() =>
                    void api.openInStudio(primary, project.path, picked?.path).catch((err) => {
                      setError(err instanceof Error ? err.message : String(err));
                    })
                  }
                >
                  {t.workspace.openEditor}
                </Button>
              ) : null}
              <Button size="sm" variant="ghost" onClick={() => void api.openPath(projectPath)}>
                {t.workspace.openFolder}
              </Button>
            </div>
          </div>
          <div
            className={cn(
              "studio-grid min-h-0 flex-1",
              stage === "git" && "is-git",
              !showRail && "is-wide",
            )}
          >
            {showRail ? (
              <aside className="studio-rail min-h-0 min-w-0 overflow-hidden">
                {stage === "graph" ? (
                  <ModuleTree
                    graph={graph}
                    selectedId={picked?.id}
                    query={query}
                    onQuery={setQuery}
                    onSelect={selectNode}
                    onOpen={openNode}
                    onCopy={(n) => void navigator.clipboard.writeText(n.id)}
                    t={t}
                  />
                ) : (
                  <GitRail
                    files={git?.files ?? []}
                    commitFiles={commitFiles}
                    setCommitFiles={setCommitFiles}
                    selectAll={t.workspace.selectAll}
                    selectNone={t.workspace.selectNone}
                  />
                )}
              </aside>
            ) : null}
            <section className="relative min-h-0 min-w-0 overflow-hidden">
              <div className={cn("absolute inset-0", stage !== "graph" && "invisible pointer-events-none")}>
                <ForceGraph
                  graph={graph}
                  selectedId={picked?.id}
                  active={stage === "graph"}
                  onSelect={setPicked}
                  onMenu={(node, x, y) => setMenu({ node, x, y })}
                />
                {stage === "graph" ? (
                  <p className="pointer-events-none absolute bottom-3 left-4 text-[11px] text-mist-300/70">
                    {t.workspace.dragHint}
                  </p>
                ) : null}
              </div>
              {stage === "hex" ? (
                <div className="relative z-10 h-full min-h-0">
                <HexAdd
                  projectPath={projectPath}
                  deps={project?.deps ?? []}
                  locked={project?.locked ?? []}
                  t={t}
                  busy={busy}
                  onBusy={setBusy}
                  onDone={async (log) => {
                    const id = activeTerm;
                    setConsoleOpen(true);
                    patchTerm(id, (s) => ({
                      ...s,
                      task: "mix deps.get",
                      title: tabTitle("mix deps.get"),
                      ok: !/не является|error/i.test(log),
                      lines: [
                        ...s.lines,
                        ...log.split("\n").map((text) => ({ id: ++lineId.current, text })),
                      ],
                    }));
                    await reload();
                  }}
                  onError={(msg) => setError(msg || null)}
                />
                </div>
              ) : null}
              {stage === "git" ? (
                <div className="relative z-10 h-full min-h-0">
                <GitStudio
                  git={git}
                  t={t}
                  commitMsg={commitMsg}
                  setCommitMsg={setCommitMsg}
                  commitFiles={commitFiles}
                  setCommitFiles={setCommitFiles}
                  busy={busy}
                  licenses={licenses}
                  license={license}
                  setLicense={setLicense}
                  onInit={async () => {
                    setBusy(true);
                    setError(null);
                    try {
                      setGit(await api.gitInit(projectPath, license));
                    } catch (err) {
                      setError(err instanceof Error ? err.message : String(err));
                    } finally {
                      setBusy(false);
                    }
                  }}
                  onCommit={async () => {
                    setBusy(true);
                    setError(null);
                    try {
                      const log = await api.projectCommit(projectPath, commitMsg, commitFiles);
                      const id = activeTerm;
                      setConsoleOpen(true);
                      patchTerm(id, (s) => ({
                        ...s,
                        title: tabTitle("git commit"),
                        task: "git commit",
                        ok: true,
                        lines: [
                          ...s.lines,
                          { id: ++lineId.current, text: "git commit", kind: "in" as const },
                          ...log.split("\n").map((text) => ({ id: ++lineId.current, text })),
                        ],
                      }));
                      setGit(await api.projectGit(projectPath));
                      setCommitMsg("");
                      setCommitFiles([]);
                    } catch (err) {
                      setError(err instanceof Error ? err.message : String(err));
                    } finally {
                      setBusy(false);
                    }
                  }}
                />
                </div>
              ) : null}
              {stage === "quality" ? (
                <div className="relative z-10 h-full min-h-0">
                <QualityStudio
                  kits={kits}
                  report={report}
                  busy={busy}
                  t={t}
                  onScan={() => void runScan(false)}
                  onFull={() => void runScan(true)}
                  onFormat={(check) => void run(check ? "format.check" : "format")}
                  onApply={async (id) => {
                    setBusy(true);
                    try {
                      const log = await api.applyKits(projectPath, [id]);
                      patchTerm(activeTerm, (s) => ({
                        ...s,
                        ok: true,
                        lines: [
                          ...s.lines,
                          ...log.split("\n").map((text) => ({ id: ++lineId.current, text })),
                        ],
                      }));
                      setConsoleOpen(true);
                      await reload();
                    } catch (err) {
                      setError(err instanceof Error ? err.message : String(err));
                    } finally {
                      setBusy(false);
                    }
                  }}
                  onRemove={async (id) => {
                    setBusy(true);
                    try {
                      const log = await api.removeKit(projectPath, id);
                      patchTerm(activeTerm, (s) => ({
                        ...s,
                        ok: true,
                        lines: [
                          ...s.lines,
                          ...log.split("\n").map((text) => ({ id: ++lineId.current, text })),
                        ],
                      }));
                      setConsoleOpen(true);
                      await reload();
                    } catch (err) {
                      setError(err instanceof Error ? err.message : String(err));
                    } finally {
                      setBusy(false);
                    }
                  }}
                  onWriteConfig={async (id) => {
                    setBusy(true);
                    try {
                      const log = await api.writeKitConfig(projectPath, id);
                      patchTerm(activeTerm, (s) => ({
                        ...s,
                        lines: [...s.lines, { id: ++lineId.current, text: log }],
                      }));
                      setKits(await api.listKits(projectPath));
                    } catch (err) {
                      setError(err instanceof Error ? err.message : String(err));
                    } finally {
                      setBusy(false);
                    }
                  }}
                  onOpenConfig={(file) => {
                    const full = `${projectPath.replace(/[/\\]+$/, "")}\\${file}`;
                    if (primary) {
                      void api.openInStudio(primary, projectPath, file).catch(() => {
                        void api.openPath(full);
                      });
                    } else {
                      void api.openPath(full);
                    }
                  }}
                  onCredoStrict={async (strict) => {
                    setBusy(true);
                    try {
                      const log = await api.setCredoStrict(projectPath, strict);
                      patchTerm(activeTerm, (s) => ({
                        ...s,
                        lines: [...s.lines, { id: ++lineId.current, text: log }],
                      }));
                      setKits(await api.listKits(projectPath));
                    } catch (err) {
                      setError(err instanceof Error ? err.message : String(err));
                    } finally {
                      setBusy(false);
                    }
                  }}
                />
                </div>
              ) : null}
              {stage === "elixir" && project ? (
                <div className="studio-stage-enter relative z-10 h-full overflow-y-auto p-4">
                  <Overview
                    project={project}
                    toolchains={toolchains}
                    busy={busy}
                    t={t.projects}
                    onBusy={setBusy}
                    onError={(v) => setError(v)}
                    onProject={async (next) => {
                      setProject(next);
                      await refreshToolchains();
                    }}
                  />
                </div>
              ) : null}
            </section>
            {stage === "graph" ? (
              <aside className="studio-inspector min-h-0 min-w-0 overflow-hidden">
                <Inspector
                  node={picked}
                  graph={graph}
                  t={t}
                  busy={busy}
                  onOpenComment={goToComment}
                  onSave={async (file, tag, value) => {
                    setError(null);
                    quietUntil.current = Date.now() + 2800;
                    await api.addComment(projectPath, file, tag, value);
                    setGraph((g) => {
                      if (!g) return g;
                      const next = (g.comments ?? []).filter((c) => !(c.file === file && c.tag === tag));
                      next.push({ file, line: 1, tag, value, module: picked?.id });
                      const nodes = g.nodes.map((n) =>
                        n.path === file && tag === "note"
                          ? { ...n, notes: [...(n.notes ?? []).filter((x) => x !== value), value] }
                          : n,
                      );
                      return { ...g, comments: next, nodes };
                    });
                  }}
                />
              </aside>
            ) : null}
          </div>
          {error ? (
            <div className="selectable shrink-0 border-t border-otp-500/20 px-4 py-1.5 text-[12px] text-otp-400">
              {error.split("\n")[0]}
            </div>
          ) : null}
          <ConsoleDock
            sessions={terms}
            activeId={activeTerm}
            onActive={setActiveTerm}
            onNew={() => {
              termSeq.current += 1;
              const id = `t${termSeq.current}`;
              setTerms((all) => [...all, newTerm(id, `shell ${termSeq.current}`)]);
              setActiveTerm(id);
              setConsoleOpen(true);
            }}
            onClose={(id) => {
              setTerms((all) => {
                const next = all.filter((s) => s.id !== id);
                if (!next.length) return all;
                setActiveTerm((cur) => (cur === id ? next[0].id : cur));
                return next;
              });
            }}
            onSubmit={(id, command) => void runShell(id, command)}
            open={consoleOpen}
            onToggle={() => setConsoleOpen((v) => !v)}
            empty={t.workspace.consoleEmpty}
            runningLabel={t.workspace.running}
            passedLabel={t.workspace.passed}
            failedLabel={t.workspace.failed}
            copyLabel={t.workspace.copy}
            placeholder={t.workspace.shellHint}
          />
        </div>
      </div>
      {menu ? <ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.node)} onClose={() => setMenu(null)} /> : null}
    </div>
  );
}

function startCommand(project: { hasPhoenix: boolean; hasApplication?: boolean } | null, custom: string) {
  const trimmed = custom.trim();
  if (trimmed) return trimmed;
  if (project?.hasPhoenix) return "mix phx.server";
  if (project?.hasApplication) return "mix run --no-halt";
  return "mix run";
}
