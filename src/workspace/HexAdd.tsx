import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { api, browse } from "../lib/api";
import { Button, Pill, Input } from "../components/ui";
import type { HexPackage, MixDep } from "../types";
import type { Dictionary } from "../i18n";
import { cn } from "../lib/cn";

export function HexAdd({
  projectPath,
  deps,
  locked,
  t,
  busy,
  onBusy,
  onDone,
  onError,
}: {
  projectPath: string;
  deps: MixDep[];
  locked: MixDep[];
  t: Dictionary;
  busy: boolean;
  onBusy: (v: boolean) => void;
  onDone: (log: string) => void | Promise<void>;
  onError: (msg: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [depQuery, setDepQuery] = useState("");
  const [items, setItems] = useState<HexPackage[]>([]);
  const [picked, setPicked] = useState<HexPackage | null>(null);
  const [loading, setLoading] = useState(true);
  const searchGen = useRef(0);
  const lockBy = new Map(locked.map((d) => [d.name, d.spec]));

  async function search(value = query) {
    const gen = ++searchGen.current;
    onError("");
    setLoading(true);
    try {
      const next = await api.hex(value, false);
      if (gen !== searchGen.current) return;
      setItems(next);
      setPicked((current) => next.find((p) => p.name === current?.name) ?? next[0] ?? null);
    } catch (err) {
      if (gen !== searchGen.current) return;
      onError(err instanceof Error ? err.message : String(err));
    } finally {
      if (gen === searchGen.current) setLoading(false);
    }
  }

  useEffect(() => {
    void search("");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!picked?.name) return;
    let cancelled = false;
    void api.hexPackage(picked.name).then((full) => {
      if (cancelled) return;
      setPicked((current) => (current?.name === full.name ? { ...current, ...full } : current));
    }).catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [picked?.name]);

  const inProject = picked ? deps.some((d) => d.name === picked.name) : false;
  const depShown = depQuery.trim()
    ? deps.filter((d) => d.name.toLowerCase().includes(depQuery.trim().toLowerCase()))
    : deps;

  return (
    <div className="flex h-full min-h-0">
      <div className="flex min-h-0 min-w-0 flex-1 flex-col border-r border-white/8">
        <div className="shrink-0 border-b border-white/8 px-4 py-3">
          <div className="mb-2 flex items-baseline justify-between gap-2">
            <div className="text-[10px] uppercase tracking-wider text-mist-300">{t.workspace.inProject}</div>
            <span className="font-mono text-[10px] text-mist-300">{deps.length}</span>
          </div>
          {deps.length > 8 ? (
            <Input
              size="sm"
              className="mb-2"
              value={depQuery}
              onChange={(e) => setDepQuery(e.target.value)}
              placeholder={t.workspace.depsFilter}
            />
          ) : null}
          {deps.length ? (
            <div className={cn("grid gap-0.5", deps.length > 8 && "max-h-52 overflow-y-auto")}>
              {depShown.map((dep) => {
                const lock = lockBy.get(dep.name);
                return (
                  <div
                    key={dep.name}
                    className={cn(
                      "flex min-w-0 items-center gap-2 rounded-md px-2 py-1 font-mono text-[11px] hover:bg-white/5",
                      picked?.name === dep.name && "bg-elixir-600/20",
                    )}
                  >
                    <button
                      type="button"
                      className="min-w-0 flex-1 truncate text-left"
                      onClick={() => {
                        setQuery(dep.name);
                        void search(dep.name);
                      }}
                    >
                      <span className="text-mist-50">{dep.name}</span>
                      <span className="ml-1.5 text-mist-300">{lock ?? dep.spec}</span>
                    </button>
                    <button
                      type="button"
                      title={t.workspace.removeDep}
                      className="shrink-0 text-mist-300 hover:text-otp-400"
                      disabled={busy}
                      onClick={() => {
                        onBusy(true);
                        void api
                          .removeHexDep(projectPath, dep.name)
                          .then(onDone)
                          .catch((err) => onError(err instanceof Error ? err.message : String(err)))
                          .finally(() => onBusy(false));
                      }}
                    >
                      <X size={11} />
                    </button>
                  </div>
                );
              })}
            </div>
          ) : (
            <p className="text-[12px] text-mist-300">{t.workspace.noDeps}</p>
          )}
        </div>
        <form
          className="flex shrink-0 gap-2 px-4 py-3"
          onSubmit={(e) => {
            e.preventDefault();
            void search();
          }}
        >
          <Input
            className="flex-1"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t.hex.placeholder}
          />
          <Button type="submit" disabled={busy}>
            {t.hex.search}
          </Button>
        </form>
        <div className="h-0 min-h-0 flex-1 overflow-y-auto px-2 pb-3">
          {loading && !items.length ? (
            <p className="px-3 py-6 text-[13px] text-mist-300">{t.common.loading}</p>
          ) : null}
          {items.map((pkg) => (
            <button
              type="button"
              key={pkg.name}
              onClick={() => setPicked(pkg)}
              className={cn(
                "flex w-full min-w-0 items-center gap-3 rounded-lg px-3 py-2 text-left hover:bg-white/5",
                picked?.name === pkg.name && "bg-elixir-600/18",
              )}
            >
              <span className="min-w-0 flex-1">
                <span className="flex items-baseline gap-2">
                  <span className="truncate text-[13px] font-medium">{pkg.name}</span>
                  <span className="shrink-0 font-mono text-[11px] text-elixir-300">{pkg.latest}</span>
                  {deps.some((d) => d.name === pkg.name) ? (
                    <span className="shrink-0 text-[10px] text-ok-400">in mix</span>
                  ) : null}
                </span>
                <span className="mt-0.5 block truncate text-[11px] text-mist-300">{pkg.description}</span>
              </span>
              <span className="shrink-0 font-mono text-[10px] text-mist-300">
                {compact(pkg.downloadsRecent ?? pkg.downloads ?? 0)}
              </span>
            </button>
          ))}
        </div>
      </div>
      <aside className="flex h-full w-[300px] shrink-0 flex-col">
        {picked ? (
          <>
            <div className="min-h-0 flex-1 overflow-y-auto p-4">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-[18px] font-semibold tracking-tight">{picked.name}</h2>
                {picked.latest ? <Pill>{picked.latest}</Pill> : null}
                {inProject ? <Pill tone="ok">mix.exs</Pill> : null}
              </div>
              <p className="mt-3 text-[13px] leading-5 text-mist-300">{picked.description}</p>
              {picked.licenses?.length ? (
                <div className="mt-3 text-[11px] text-mist-300">
                  {t.hex.licenses}: <span className="text-mist-100">{picked.licenses.join(", ")}</span>
                </div>
              ) : null}
              {picked.latest ? (
                <div className="mt-2 font-mono text-[11px] text-elixir-300">
                  {t.hex.mixTuple} {`{:${picked.name}, "~> ${picked.latest.replace(/^(\d+\.\d+).*/, "$1")}"}`}
                </div>
              ) : null}
              {picked.links && Object.keys(picked.links).length ? (
                <div className="mt-3 grid gap-1">
                  <div className="text-[10px] uppercase tracking-wider text-mist-300">{t.hex.links}</div>
                  {Object.entries(picked.links).map(([label, href]) => (
                    <button
                      key={label}
                      type="button"
                      className="truncate text-left font-mono text-[11px] text-elixir-300 hover:text-white"
                      onClick={() => void browse(href)}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              ) : null}
              <div className="mt-4 font-mono text-[11px] text-mist-300">
                {(picked.downloadsRecent ?? 0).toLocaleString()} {t.hex.recent}
              </div>
            </div>
            <div className="flex shrink-0 flex-col gap-2 border-t border-white/8 p-4">
              {inProject ? (
                <Button
                  variant="danger"
                  disabled={busy}
                  onClick={async () => {
                    onBusy(true);
                    try {
                      onDone(await api.removeHexDep(projectPath, picked.name));
                    } catch (err) {
                      onError(err instanceof Error ? err.message : String(err));
                    } finally {
                      onBusy(false);
                    }
                  }}
                >
                  {t.workspace.removeDep}
                </Button>
              ) : (
                <Button
                  disabled={busy || !picked.latest}
                  onClick={async () => {
                    onBusy(true);
                    try {
                      const req = picked.latest ? `~> ${picked.latest.replace(/^(\d+\.\d+).*/, "$1")}` : "~> 0.1";
                      onDone(await api.addHexDep(projectPath, picked.name, req));
                    } catch (err) {
                      onError(err instanceof Error ? err.message : String(err));
                    } finally {
                      onBusy(false);
                    }
                  }}
                >
                  {t.workspace.add}
                </Button>
              )}
              <div className="flex gap-2">
                <Button variant="ghost" onClick={() => void browse(picked.htmlUrl)}>
                  {t.hex.package}
                </Button>
                <Button variant="ghost" onClick={() => void browse(picked.docsUrl)}>
                  {t.hex.docs}
                </Button>
              </div>
            </div>
          </>
        ) : (
          <p className="p-4 text-[13px] text-mist-300">{t.hex.placeholder}</p>
        )}
      </aside>
    </div>
  );
}

function compact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}k`;
  return String(n);
}
