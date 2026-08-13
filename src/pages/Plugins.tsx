import { useEffect, useMemo, useState } from "react";
import { api, browse } from "../lib/api";
import { useApp } from "../state";
import { Button, Chip, Loader, PageShell, Pill } from "../components/ui";
import type { PluginStatus, Studio } from "../types";

export function PluginsPage() {
  const { t, studios, refreshStudios } = useApp();
  const [items, setItems] = useState<PluginStatus[]>([]);
  const [studioId, setStudioId] = useState<string>("all");
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanned, setScanned] = useState(false);
  const detected = useMemo(() => studios.filter((s) => s.detected), [studios]);
  const vscodeFamily = detected.filter((s) => s.family === "vscode");

  useEffect(() => {
    let live = true;
    void refreshStudios().finally(() => {
      if (live) setScanned(true);
    });
    return () => {
      live = false;
    };
  }, [refreshStudios]);

  useEffect(() => {
    if (!scanned) return;
    let live = true;
    setLoading(true);
    void api
      .plugins(detected)
      .then((list) => {
        if (live) setItems(list);
      })
      .finally(() => {
        if (live) setLoading(false);
      });
    return () => {
      live = false;
    };
  }, [detected, scanned]);

  const visible = useMemo(() => {
    if (studioId === "all") return items;
    const studio = detected.find((s) => s.id === studioId);
    if (!studio) return items;
    return items.filter((status) => status.plugin.family === studio.family);
  }, [detected, items, studioId]);

  async function installInto(status: PluginStatus, target: Studio) {
    if (!status.plugin.marketplaceId) return;
    setBusy(`${status.plugin.id}:${target.id}`);
    try {
      await api.installPlugin(target, status.plugin.marketplaceId);
      setItems(await api.plugins(detected));
    } finally {
      setBusy(null);
    }
  }

  async function copyNvim() {
    const snippet = await api.neovimSnippet();
    await navigator.clipboard.writeText(snippet);
  }

  return (
    <PageShell title={t.plugins.title} subtitle={t.plugins.subtitle} fill>
      <div className="flex shrink-0 flex-wrap gap-2">
        <Chip active={studioId === "all"} onClick={() => setStudioId("all")}>
          {t.plugins.filterAll}
        </Chip>
        {detected.map((studio) => (
          <Chip key={studio.id} active={studioId === studio.id} onClick={() => setStudioId(studio.id)}>
            {studio.name}
          </Chip>
        ))}
      </div>
      {loading ? (
        <Loader label={t.plugins.scanning} />
      ) : (
        <div className="surface min-h-0 flex-1 divide-y divide-white/6 overflow-y-auto rounded-xl">
          {visible.map((status) => {
            const familyStudios = detected.filter((s) => s.family === status.plugin.family);
            const targets = studioId === "all" ? familyStudios : familyStudios.filter((s) => s.id === studioId);
            return (
              <div key={status.plugin.id} className="px-4 py-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="text-[13px] font-medium">{status.plugin.name}</h3>
                      {status.plugin.beginner ? <Pill>{t.plugins.beginner}</Pill> : null}
                      {status.installedIn.length > 0 ? (
                        <Pill tone="ok">{t.plugins.installed}</Pill>
                      ) : (
                        <Pill tone="mute">{t.plugins.missing}</Pill>
                      )}
                    </div>
                    <p className="mt-1 max-w-2xl text-[13px] leading-5 text-mist-300">{status.plugin.summary}</p>
                  </div>
                  <Button variant="ghost" size="sm" onClick={() => void browse(status.plugin.url)}>
                    {t.plugins.open}
                  </Button>
                </div>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {status.plugin.family === "neovim" ? (
                    <Button size="sm" onClick={() => void copyNvim()}>
                      {t.plugins.copyNvim}
                    </Button>
                  ) : null}
                  {targets.map((studio) => {
                    const has = status.installedIn.includes(studio.id);
                    const canCli = Boolean(studio.cli && status.plugin.marketplaceId);
                    if (has) {
                      return (
                        <Button key={studio.id} variant="ghost" size="sm" disabled>
                          {studio.name} ✓
                        </Button>
                      );
                    }
                    if (canCli) {
                      return (
                        <Button
                          key={studio.id}
                          size="sm"
                          disabled={busy !== null}
                          onClick={() => void installInto(status, studio)}
                        >
                          {t.plugins.installIn} {studio.name}
                        </Button>
                      );
                    }
                    return (
                      <Button key={studio.id} variant="ghost" size="sm" onClick={() => void browse(status.plugin.url)}>
                        {studio.name} · {t.plugins.open}
                      </Button>
                    );
                  })}
                  {status.plugin.family === "vscode" &&
                  vscodeFamily.filter((s) => s.cli).length > 1 &&
                  status.plugin.marketplaceId ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={busy !== null}
                      onClick={async () => {
                        for (const studio of vscodeFamily.filter((s) => s.cli)) {
                          if (!status.installedIn.includes(studio.id)) {
                            await installInto(status, studio);
                          }
                        }
                      }}
                    >
                      {t.plugins.allVscode}
                    </Button>
                  ) : null}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </PageShell>
  );
}
