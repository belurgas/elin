import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { ArrowDownToLine, Sparkles, X } from "lucide-react";
import { useApp } from "../state";
import { api, browse, onUpdateProgress } from "../lib/api";
import { Button } from "./ui";
import { cn } from "../lib/cn";

type UpdateFlow = {
  busy: boolean;
  percent: number;
  message: string | null;
  run: () => Promise<void>;
};

const UpdateFlowCtx = createContext<UpdateFlow | null>(null);

export function UpdateInstallProvider({ children }: { children: ReactNode }) {
  const value = useUpdateInstallState();
  return <UpdateFlowCtx.Provider value={value}>{children}</UpdateFlowCtx.Provider>;
}

function useFlow(): UpdateFlow {
  const ctx = useContext(UpdateFlowCtx);
  if (!ctx) {
    throw new Error("UpdateInstallProvider is missing");
  }
  return ctx;
}

export function UpdateBar() {
  const { t, offerUpdate, appUpdate, dismissUpdate } = useApp();
  const flow = useFlow();
  const [notesOpen, setNotesOpen] = useState(false);

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
          <Button variant="ghost" size="sm" onClick={() => setNotesOpen(true)}>
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
          <div className="h-full bg-elixir-400 transition-[width] duration-200" style={{ width: `${Math.max(flow.percent, 2)}%` }} />
        </div>
      ) : null}
      {notesOpen ? <NotesDialog onClose={() => setNotesOpen(false)} /> : null}
    </div>
  );
}

function NotesDialog({ onClose }: { onClose: () => void }) {
  const { t, appUpdate } = useApp();
  if (!appUpdate) return null;

  return (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-6"
      onClick={onClose}
    >
      <div
        className="surface flex max-h-[min(72vh,640px)] w-full max-w-lg flex-col overflow-hidden rounded-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-3 border-b border-white/6 px-4 py-3">
          <div className="min-w-0">
            <div className="text-[13px] font-medium text-mist-50">{appUpdate.name || `Elin ${appUpdate.latest}`}</div>
            <p className="mt-0.5 font-mono text-[11px] text-mist-300">{appUpdate.latest}</p>
          </div>
          <button
            type="button"
            className="rounded-md p-1 text-mist-300 hover:bg-white/8 hover:text-mist-50"
            onClick={onClose}
            aria-label={t.common.dismiss}
          >
            <X size={14} />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3">
          <pre className="whitespace-pre-wrap font-sans text-[13px] leading-5 text-mist-100">
            {appUpdate.notes.trim() || t.update.none}
          </pre>
        </div>
        <div className="flex justify-end gap-2 border-t border-white/6 px-4 py-3">
          {appUpdate.htmlUrl ? (
            <Button variant="ghost" size="sm" onClick={() => void browse(appUpdate.htmlUrl)}>
              {t.update.openRelease}
            </Button>
          ) : null}
          <Button size="sm" onClick={onClose}>
            {t.common.dismiss}
          </Button>
        </div>
      </div>
    </div>
  );
}

function useUpdateInstallState(): UpdateFlow {
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
    setPercent(1);
    setMessage(t.update.downloading);
    const unlisten = await onUpdateProgress((payload) => {
      setPercent(payload.percent);
      setMessage(payload.message);
    });
    try {
      const path = await api.downloadAppUpdate(true);
      setMessage(t.update.ready);
      setPercent(100);
      await api.installAppUpdate(path);
    } catch (err) {
      setMessage(err instanceof Error ? err.message : t.update.failed);
      setBusy(false);
      void refreshAppUpdate(true);
    } finally {
      unlisten();
    }
  }

  return { busy, percent, message, run };
}

export function UpdateSettingsCard() {
  const { t, appUpdate, host, refreshAppUpdate, skipUpdate, offerUpdate } = useApp();
  const flow = useFlow();
  const [checking, setChecking] = useState(false);
  const [notesOpen, setNotesOpen] = useState(false);

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
            <Button variant="ghost" size="sm" onClick={() => setNotesOpen(true)}>
              {t.update.notes}
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
      {notesOpen ? <NotesDialog onClose={() => setNotesOpen(false)} /> : null}
    </section>
  );
}
