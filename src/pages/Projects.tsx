import { useEffect, useMemo, useState } from "react";
import { api, onScanProgress, pickFolder } from "../lib/api";
import { useApp } from "../state";
import { Button, Card, PageShell, Pill } from "../components/ui";
import type { Kit, MixProject, ScanProgress } from "../types";
import { CreateDialog } from "./studio/CreateDialog";
import { Navigator } from "./studio/Navigator";
import { Overview } from "./studio/Overview";

export function ProjectsPage() {
  const { t, host, studios, preferredStudioId, toolchains, refreshToolchains, ensureStudios, pendingProject, clearPendingProject } =
    useApp();
  const [projects, setProjects] = useState<MixProject[]>([]);
  const [selected, setSelected] = useState<MixProject | null>(null);
  const [catalog, setCatalog] = useState<Kit[]>([]);
  const [query, setQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [busy, setBusy] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [scan, setScan] = useState<ScanProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [output, setOutput] = useState("");
  const editors = studios.filter((s) => s.detected && (s.cli || s.executable));
  const primary = editors.find((s) => s.id === preferredStudioId) ?? editors[0];

  useEffect(() => {
    void ensureStudios();
    void api.kitCatalog().then(setCatalog).catch(() => undefined);
  }, [ensureStudios]);

  useEffect(() => {
    void api
      .projects()
      .then((list) => {
        setProjects(list);
        setSelected((current) => current ?? list[0] ?? null);
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onScanProgress(setScan).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!pendingProject) return;
    void api
      .addProject(pendingProject)
      .then((project) => {
        setProjects((current) => [project, ...current.filter((p) => p.path !== project.path)]);
        setSelected(project);
        clearPendingProject();
        void api.openWorkspace(project.path).catch(() => undefined);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
        clearPendingProject();
      });
  }, [pendingProject, clearPendingProject]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = q
      ? projects.filter((p) => p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q))
      : projects;
    const starred = list.filter((p) => p.starred);
    const recents = list
      .filter((p) => !p.starred && (p.lastOpened ?? 0) > 0)
      .sort((a, b) => (b.lastOpened ?? 0) - (a.lastOpened ?? 0))
      .slice(0, 8);
    const rest = list.filter((p) => !starred.includes(p) && !recents.includes(p));
    return { starred, recents, rest };
  }, [projects, query]);

  function upsert(project: MixProject) {
    setSelected(project);
    setProjects((current) => [project, ...current.filter((p) => p.path !== project.path)]);
  }

  async function openWorkspace(project: MixProject) {
    setError(null);
    try {
      await api.openWorkspace(project.path);
      upsert(await api.inspectProject(project.path));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function addFolder() {
    const folder = await pickFolder();
    if (!folder) return;
    setError(null);
    try {
      upsert(await api.addProject(folder));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function scanQuick() {
    setScanning(true);
    setError(null);
    try {
      const list = await api.scanQuick();
      setProjects(list);
      setSelected((current) => current ?? list[0] ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setScanning(false);
    }
  }

  async function scanDeep() {
    setScanning(true);
    setError(null);
    setScan({ visited: 0, found: 0, current: "", done: false });
    try {
      const found = await api.scanDeep([]);
      setProjects(found);
      setSelected(found[0] ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setScanning(false);
      setScan((s) => (s ? { ...s, done: true } : s));
    }
  }

  return (
    <PageShell
      title={t.projects.title}
      subtitle={t.projects.subtitle}
      fill
      actions={
        scanning ? (
          <Button variant="danger" onClick={() => void api.cancelScan()}>
            {t.projects.cancel}
          </Button>
        ) : (
          <Button variant="ghost" onClick={() => void scanDeep()}>
            {t.projects.scanDeep}
          </Button>
        )
      }
    >
      {error ? <Card className="selectable shrink-0 text-sm text-otp-400">{error}</Card> : null}
      {scanning || scan ? (
        <Card className="shrink-0">
          <div className="text-sm text-mist-300">{scanning ? t.projects.scanning : t.projects.deepWarn}</div>
          {scan ? (
            <div className="mt-2 font-mono text-[11px] text-elixir-300">
              {scan.visited} dirs · {scan.found} mix · {scan.current}
            </div>
          ) : null}
        </Card>
      ) : null}
      <div className="grid min-h-0 flex-1 gap-5 overflow-hidden lg:grid-cols-[minmax(260px,0.9fr)_minmax(0,1.4fr)]">
        <Navigator
          query={query}
          onQuery={setQuery}
          scanning={scanning}
          empty={projects.length === 0}
          starred={filtered.starred}
          recents={filtered.recents}
          rest={filtered.rest}
          selected={selected}
          t={t.projects}
          onSelect={setSelected}
          onActivate={(p) => void openWorkspace(p)}
          onAdd={() => void addFolder()}
          onCreate={() => setCreating(true)}
          onScanQuick={() => void scanQuick()}
        />
        {selected ? (
          <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
            <Card className="shrink-0">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h2 className="text-xl font-semibold">{selected.name}</h2>
                    {selected.hasPhoenix ? <Pill>Phoenix</Pill> : null}
                    {selected.starred ? <Pill tone="ok">{t.projects.pinnedSection}</Pill> : null}
                  </div>
                  <div className="selectable mt-1 truncate font-mono text-[11px] text-mist-300">{selected.path}</div>
                </div>
                <div className="flex flex-wrap gap-2">
                  <Button onClick={() => void openWorkspace(selected)}>{t.projects.openWorkspace}</Button>
                  <Button
                    variant="ghost"
                    onClick={() => void api.starProject(selected.path).then(upsert).catch((err) => setError(String(err)))}
                  >
                    {selected.starred ? t.projects.unstar : t.projects.star}
                  </Button>
                  {primary ? (
                    <Button variant="ghost" onClick={() => void api.openInStudio(primary, selected.path)}>
                      {t.projects.primaryOpen} · {primary.name}
                    </Button>
                  ) : (
                    <Button variant="ghost" onClick={() => void api.openPath(selected.path)}>
                      {t.projects.open}
                    </Button>
                  )}
                </div>
              </div>
            </Card>
            <Overview
              project={selected}
              toolchains={toolchains}
              busy={busy}
              t={t.projects}
              onBusy={setBusy}
              onError={setError}
              onProject={async (next) => {
                upsert(next);
                await refreshToolchains();
              }}
            />
          </div>
        ) : (
          <Card className="text-sm text-mist-300">{t.projects.empty}</Card>
        )}
      </div>
      {output ? (
        <Card className="selectable max-h-40 shrink-0 overflow-y-auto whitespace-pre-wrap font-mono text-xs text-mist-100">
          {output}
        </Card>
      ) : null}
      {creating ? (
        <CreateDialog
          t={t.projects}
          host={host?.home ?? ""}
          catalog={catalog}
          busy={busy}
          onClose={() => setCreating(false)}
          onCreate={async (name, directory, template, kitIds) => {
            setBusy(true);
            setError(null);
            try {
              const result = await api.spark(name, directory, template, kitIds);
              setOutput(`${result.path}\n\n${result.output}`);
              const project = await api.inspectProject(result.path);
              upsert(project);
              setCreating(false);
              await api.openWorkspace(project.path);
            } catch (err) {
              setError(err instanceof Error ? err.message : String(err));
            } finally {
              setBusy(false);
            }
          }}
        />
      ) : null}
    </PageShell>
  );
}
