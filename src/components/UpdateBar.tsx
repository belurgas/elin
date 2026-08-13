import { useEffect, useState } from "react";
import { ArrowDownToLine, Sparkles } from "lucide-react";
import { useApp } from "../state";
import { api, browse, onUpdateProgress } from "../lib/api";
import { Button } from "./ui";
import { cn } from "../lib/cn";

export function UpdateBar() {
  const { t, offerUpdate, appUpdate, dismissUpdate, setPage } = useApp();
  const flow = useUpdateInstall();

  if (!offerUpdate || !appUpdate) return null;

  return (
    <div className="relative shrink-0 border-b border-elixir-500/25 bg-elixir-600/15 px-4 py-2.5">
      <div className="flex flex-wrap items-center gap-3">
        <Sparkles size={14} className="text-elixir-400" />
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-medium text-mist-50">
            {t.update.available.replace("{version}", appUpdate.latest)}
          </div>
          {flow.message ? (
            <p className="mt-0.5 text-[11px] text-mist-300">{flow.message}</p>
          ) : appUpdate.notes ? (
            <p className="mt-0.5 line-clamp-1 text-[11px] text-mist-300">{appUpdate.notes}</p>
          ) : null}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <Button variant="ghost" size="sm" onClick={() => setPage("settings")}>
            {t.update.notes}
          </Button>
          <Button variant="ghost" size="sm" onClick={dismissUpdate}>
            {t.update.later}
          </Button>
          <Button size="sm" disabled={flow.busy} onClick={() => void flow.run()}>
            <ArrowDownToLine size={12} />
            {flow.busy ? `${flow.percent}%` : t.update.install}
          </Button>
        </div>
      </div>
      {flow.busy ? (
        <div className="absolute inset-x-0 bottom-0 h-0.5 bg-white/10">
          <div className="h-full bg-elixir-400 transition-[width] duration-200" style={{ width: `${flow.percent}%` }} />
        </div>
      ) : null}
    </div>
  );
}

export function useUpdateInstall() {
  const { t, refreshAppUpdate } = useApp();
  const [busy, setBusy] = useState(false);
  const [percent, setPercent] = useState(0);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let un: (() => void) | undefined;
    void onUpdateProgress((payload) => {
      setPercent(payload.percent);
      setMessage(payload.message);
    }).then((fn) => {
      un = fn;
    });
    return () => un?.();
  }, []);

  async function run() {
    setBusy(true);
    setPercent(0);
    setMessage(t.update.downloading);
    try {
      const path = await api.downloadAppUpdate(true);
      setMessage(t.update.ready);
      setPercent(100);
      await api.installAppUpdate(path);
    } catch (err) {
      setMessage(err instanceof Error ? err.message : t.update.failed);
      setBusy(false);
      void refreshAppUpdate(true);
    }
  }

  return { busy, percent, message, run };
}

export function UpdateSettingsCard() {
  const { t, appUpdate, host, refreshAppUpdate, skipUpdate, offerUpdate } = useApp();
  const flow = useUpdateInstall();
  const [checking, setChecking] = useState(false);

  return (
    <section className="surface overflow-hidden rounded-xl">
      <div className="border-b border-white/6 px-4 py-3">
        <div className="text-[13px] font-medium text-mist-50">{t.update.check}</div>
        <p className="mt-1 max-w-xl text-[12px] leading-5 text-mist-300">{t.update.hint}</p>
      </div>
      <div className="flex flex-wrap items-end justify-between gap-3 px-4 py-3">
        <div className="grid gap-1 font-mono text-[12px]">
          <div>
            <span className="text-mist-300">{t.update.current} · </span>
            <span className="text-mist-50">{host?.version ?? appUpdate?.current ?? "—"}</span>
          </div>
          <div>
            <span className="text-mist-300">{t.update.latest} · </span>
            <span className={cn(offerUpdate ? "text-elixir-300" : "text-mist-50")}>
              {appUpdate?.latest ?? "—"}
            </span>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={checking || flow.busy}
            onClick={() => {
              setChecking(true);
              void refreshAppUpdate(true).finally(() => setChecking(false));
            }}
          >
            {checking ? t.update.checking : t.update.check}
          </Button>
          {appUpdate?.htmlUrl ? (
            <Button variant="ghost" size="sm" onClick={() => void browse(appUpdate.htmlUrl)}>
              {t.update.openRelease}
            </Button>
          ) : null}
          {offerUpdate ? (
            <Button size="sm" disabled={flow.busy} onClick={() => void flow.run()}>
              {flow.busy ? `${flow.percent}%` : t.update.install}
            </Button>
          ) : null}
        </div>
      </div>
      <div className="border-t border-white/6 px-4 py-3 text-[12px] leading-5 text-mist-300">
        {flow.message ? (
          flow.message
        ) : offerUpdate ? (
          appUpdate?.notes || t.update.available.replace("{version}", appUpdate?.latest ?? "")
        ) : appUpdate && !appUpdate.newer ? (
          t.update.upToDate
        ) : (
          t.update.none
        )}
        {offerUpdate ? (
          <button type="button" className="ml-3 text-mist-300 underline decoration-white/20 hover:text-mist-50" onClick={skipUpdate}>
            {t.update.skip}
          </button>
        ) : null}
      </div>
    </section>
  );
}
