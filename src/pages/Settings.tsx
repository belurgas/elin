import { useEffect, useState } from "react";
import { useApp } from "../state";
import { Button, Menu, PageShell, Pill } from "../components/ui";
import { api } from "../lib/api";
import { isLocale, locales } from "../i18n";
import type { CacheStatus } from "../types";

export function SettingsPage() {
  const { t, locale, setLocale, host, refreshCatalog, includePrerelease } = useApp();
  const [cache, setCache] = useState<CacheStatus | null>(null);

  async function loadCache() {
    setCache(await api.cacheStatus());
  }

  useEffect(() => {
    void loadCache();
  }, []);

  return (
    <PageShell title={t.settings.title}>
      <section className="surface rounded-xl">
        <div className="border-b border-white/6 px-4 py-3">
          <div className="text-[13px] font-medium text-mist-50">{t.settings.language}</div>
          <p className="mt-1 max-w-xl text-[12px] leading-5 text-mist-300">{t.settings.languageHint}</p>
        </div>
        <div className="p-3">
          <Menu
            className="max-w-sm"
            value={locale}
            onChange={(id) => {
              if (isLocale(id)) setLocale(id);
            }}
            options={locales.map((item) => ({
              value: item.id,
              label: item.native,
              hint: item.native === item.english ? item.id : `${item.english} · ${item.id}`,
            }))}
          />
        </div>
        <div className="border-t border-white/6 px-4 py-3">
          <div className="text-[13px] text-mist-50">{t.settings.contributeLocale}</div>
          <p className="mt-1 max-w-xl text-[12px] leading-5 text-mist-300">{t.settings.contributeLocaleBody}</p>
        </div>
      </section>

      <div className="surface divide-y divide-white/6 overflow-hidden rounded-xl">
        <div className="px-4 py-3">
          <div className="text-[13px] text-mist-300">{t.settings.installs}</div>
          <div className="selectable mt-1 font-mono text-[12px] text-elixir-300">{host?.installsDir}</div>
          <p className="mt-2 text-[13px] leading-5 text-mist-300">{t.settings.about}</p>
          <p className="mt-1.5 text-[13px] leading-5 text-mist-300">{t.settings.trayHint}</p>
        </div>
        <div className="px-4 py-3">
          <div className="flex items-center justify-between gap-3">
            <div className="text-[13px] text-mist-300">{t.settings.cache}</div>
            <div className="flex gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={async () => {
                  await refreshCatalog(includePrerelease, true);
                  await api.hex("", true);
                  await loadCache();
                  await api.toast({
                    id: "cache",
                    title: "Cache refreshed",
                    body: "Version catalog and Hex Radar were fetched again.",
                    kind: "ok",
                    page: "settings",
                  });
                }}
              >
                {t.settings.refresh}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={async () => {
                  await api.cacheClear();
                  await loadCache();
                }}
              >
                {t.settings.clear}
              </Button>
            </div>
          </div>
          {cache ? (
            <div className="mt-3 grid gap-2 sm:grid-cols-2">
              <CacheRow
                label="Hex Bob / OTP catalog"
                fresh={cache.catalogFresh}
                age={cache.catalogAgeSecs}
                t={t}
              />
              <CacheRow label="Hex Radar" fresh={cache.hexFresh} age={cache.hexAgeSecs} t={t} />
            </div>
          ) : null}
          {cache ? (
            <div className="selectable mt-2 font-mono text-[11px] text-mist-300">{cache.dir}</div>
          ) : null}
        </div>
      </div>
    </PageShell>
  );
}

function CacheRow({
  label,
  fresh,
  age,
  t,
}: {
  label: string;
  fresh: boolean;
  age?: number | null;
  t: ReturnType<typeof useApp>["t"];
}) {
  return (
    <div className="flex items-center justify-between gap-2 rounded-lg bg-black/20 px-3 py-2">
      <span className="text-[13px]">{label}</span>
      <span className="flex items-center gap-2">
        <span className="font-mono text-[11px] text-mist-300">{age == null ? "—" : `${Math.round(age / 60)} min`}</span>
        <Pill tone={fresh ? "ok" : "mute"}>{fresh ? t.settings.cacheFresh : t.settings.cacheStale}</Pill>
      </span>
    </div>
  );
}
