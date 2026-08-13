import { useEffect, useState } from "react";
import { api, browse } from "../lib/api";
import { useApp } from "../state";
import { Button, Card, PageShell, Pill } from "../components/ui";
import type { HexPackage } from "../types";
import { cn } from "../lib/cn";

export function HexPage() {
  const { t } = useApp();
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<HexPackage[]>([]);
  const [selected, setSelected] = useState<HexPackage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(true);

  async function search(value = query, force = false) {
    setError(null);
    setBusy(true);
    try {
      const next = await api.hex(value, force);
      setItems(next);
      setSelected((current) => next.find((p) => p.name === current?.name) ?? next[0] ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void search("", false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selected?.name) return;
    let cancelled = false;
    void api.hexPackage(selected.name).then((full) => {
      if (cancelled) return;
      setSelected((current) => (current?.name === full.name ? { ...current, ...full } : current));
    }).catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [selected?.name]);

  const featured = selected ?? items[0];

  return (
    <PageShell
      title={t.hex.title}
      subtitle={t.hex.subtitle}
      fill
      actions={
        <Button variant="ghost" onClick={() => void search(query, true)}>
          {t.settings.refresh}
        </Button>
      }
    >
      <form
        className="flex shrink-0 gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void search();
        }}
      >
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t.hex.placeholder}
          className="field flex-1"
        />
        <Button type="submit">{t.hex.search}</Button>
      </form>
      {error && featured ? <Card className="text-sm text-otp-400">{error}</Card> : null}

      <div className="grid min-h-0 w-full flex-1 grid-cols-1 gap-5 overflow-hidden lg:h-full lg:grid-cols-[minmax(0,1fr)_300px]">
        <div className="surface flex h-full min-h-0 min-w-0 w-full flex-col overflow-hidden rounded-xl">
          <div className="h-0 min-h-0 flex-1 overflow-y-auto overscroll-contain">
            {items.map((pkg, index) => {
              const active = featured?.name === pkg.name;
              return (
                <button
                  type="button"
                  key={pkg.name}
                  onClick={() => setSelected(pkg)}
                  className={cn(
                    "flex w-full cursor-pointer items-center gap-3 border-b border-white/5 px-3 py-2 text-left last:border-b-0",
                    active ? "bg-white/6" : "hover:bg-white/4",
                  )}
                >
                  <span className="w-5 font-mono text-[11px] text-mist-300/70">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline gap-2">
                      <span className="truncate text-[13px] font-medium">{pkg.name}</span>
                      <span className="font-mono text-[11px] text-elixir-300">{pkg.latest ?? ""}</span>
                    </div>
                    <p className="mt-0.5 line-clamp-1 text-xs text-mist-300">{pkg.description ?? ""}</p>
                  </div>
                  <div className="shrink-0 text-right font-mono text-xs text-mist-50">
                    {compact(pkg.downloadsRecent ?? pkg.downloads ?? 0)}
                  </div>
                </button>
              );
            })}
          </div>
        </div>
        {featured ? (
          <Card className="h-fit min-w-0 w-full">
            <Pill>{query.trim() ? t.hex.search : t.hex.trending}</Pill>
            <h2 className="mt-2 text-2xl font-semibold tracking-tight">{featured.name}</h2>
            <p className="mt-2 text-[13px] leading-5 text-mist-300">{featured.description ?? ""}</p>
            {featured.licenses?.length ? (
              <p className="mt-2 text-[12px] text-mist-300">
                {t.hex.licenses}: <span className="text-mist-100">{featured.licenses.join(", ")}</span>
              </p>
            ) : null}
            {featured.latest ? (
              <p className="mt-2 font-mono text-[12px] text-elixir-300">
                {`{:${featured.name}, "~> ${featured.latest.replace(/^(\d+\.\d+).*/, "$1")}"}`}
              </p>
            ) : null}
            {featured.links && Object.keys(featured.links).length ? (
              <div className="mt-3 grid gap-1">
                {Object.entries(featured.links).map(([label, href]) => (
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
            <div className="mt-4 grid grid-cols-2 gap-2">
              <Stat label={t.hex.recent} value={(featured.downloadsRecent ?? 0).toLocaleString()} />
              <Stat label={t.hex.downloads} value={(featured.downloads ?? 0).toLocaleString()} />
            </div>
            <div className="mt-2 font-mono text-xs text-elixir-300">{featured.latest ?? ""}</div>
            <div className="mt-4 flex gap-2">
              <Button variant="ghost" size="sm" onClick={() => void browse(featured.htmlUrl)}>
                {t.hex.package}
              </Button>
              <Button size="sm" onClick={() => void browse(featured.docsUrl)}>
                {t.hex.docs}
              </Button>
            </div>
          </Card>
        ) : (
          <Card className="h-fit text-sm text-mist-300">
            {error ?? (busy ? t.common.loading : t.hex.placeholder)}
          </Card>
        )}
      </div>
    </PageShell>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-black/25 px-3 py-2">
      <div className="text-[11px] text-mist-300">{label}</div>
      <div className="mt-0.5 font-mono text-lg text-mist-50">{value}</div>
    </div>
  );
}

function compact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 10_000 ? 0 : 1)}k`;
  return String(n);
}
