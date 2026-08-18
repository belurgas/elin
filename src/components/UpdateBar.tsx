import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { ArrowDownToLine, Sparkles } from "lucide-react";
import { useApp } from "../state";
import { api, browse, onUpdateProgress } from "../lib/api";
import { Button, Modal, ProgressBar } from "./ui";
import { Markdown } from "./Markdown";
import { cn } from "../lib/cn";

type UpdateFlow = {
  busy: boolean;
  percent: number;
  unknown: boolean;
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
            <p className="mt-0.5 line-clamp-1 text-[11px] text-mist-300">{plainPreview(appUpdate.notes)}</p>
          ) : null}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <Button variant="ghost" size="sm" onClick={() => setNotesOpen(true)}>
            {t.update.notes}
          </Button>
          <Button variant="ghost" size="sm" disabled={flow.busy} onClick={dismissUpdate}>
            {t.update.later}
          </Button>
          <Button size="sm" disabled={flow.busy} onClick={() => void flow.run()}>
            <ArrowDownToLine size={12} />
            {flow.busy ? `${Math.max(flow.percent, 1)}%` : t.update.install}
          </Button>
        </div>
      </div>
      {flow.busy ? (
        <div className="absolute inset-x-0 bottom-0">
          <ProgressBar value={flow.percent} unknown={flow.unknown} />
        </div>
      ) : null}
      <NotesDialog open={notesOpen} onClose={() => setNotesOpen(false)} />
    </div>
  );
}

function NotesDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t, appUpdate } = useApp();
  if (!appUpdate) return null;
  const published = formatPublished(appUpdate.publishedAt);
  const subtitle = [appUpdate.latest, published].filter(Boolean).join(" · ");

  return (
    <Modal
      open={open}
      onClose={onClose}
      size="lg"
      title={appUpdate.name || `Elin ${appUpdate.latest}`}
      subtitle={subtitle}
      footer={
        <>
          {appUpdate.htmlUrl ? (
            <Button variant="ghost" size="sm" onClick={() => void browse(appUpdate.htmlUrl)}>
              {t.update.openRelease}
            </Button>
          ) : null}
          <Button size="sm" onClick={onClose}>
            {t.common.dismiss}
          </Button>
        </>
      }
    >
      {appUpdate.notes.trim() ? (
        <Markdown source={appUpdate.notes} />
      ) : (
        <p className="text-[13px] leading-5 text-mist-300">{t.update.none}</p>
      )}
    </Modal>
  );
}

function useUpdateInstallState(): UpdateFlow {
  const { t, refreshAppUpdate } = useApp();
  const [busy, setBusy] = useState(false);
  const [percent, setPercent] = useState(0);
  const [unknown, setUnknown] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let un: (() => void) | undefined;
    void onUpdateProgress((payload) => {
      const stage = payload.stage || "download";
      if (stage === "error") {
        setBusy(false);
        setUnknown(false);
        setMessage(payload.message || t.update.failed);
        void refreshAppUpdate(true);
        return;
      }
      setBusy(true);
      setPercent(payload.percent);
      setUnknown(stage === "download" && payload.total === 0 && payload.percent < 20);
      setMessage(labelFor(t.update, stage, payload.percent, payload.downloaded, payload.total, payload.message));
    }).then((fn) => {
      un = fn;
    });
    return () => un?.();
  }, [refreshAppUpdate, t.update]);

  async function run() {
    if (busy) return;
    setBusy(true);
    setPercent(1);
    setUnknown(true);
    setMessage(t.update.downloading);
    try {
      await api.startAppUpdate(false);
    } catch (err) {
      setBusy(false);
      setUnknown(false);
      setMessage(err instanceof Error ? err.message : t.update.failed);
      void refreshAppUpdate(true);
    }
  }

  return { busy, percent, unknown, message, run };
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
          {appUpdate ? (
            <Button variant="ghost" size="sm" onClick={() => setNotesOpen(true)}>
              {t.update.notes}
            </Button>
          ) : null}
          {offerUpdate ? (
            <Button size="sm" disabled={flow.busy} onClick={() => void flow.run()}>
              {flow.busy ? `${Math.max(flow.percent, 1)}%` : t.update.install}
            </Button>
          ) : null}
        </div>
      </div>
      {flow.busy ? (
        <div className="px-4 pb-2">
          <ProgressBar value={flow.percent} unknown={flow.unknown} />
        </div>
      ) : null}
      <div className="border-t border-white/6 px-4 py-3 text-[12px] leading-5 text-mist-300">
        {flow.message ? (
          flow.message
        ) : offerUpdate ? (
          plainPreview(appUpdate?.notes || "") || t.update.available.replace("{version}", appUpdate?.latest ?? "")
        ) : appUpdate && !appUpdate.newer ? (
          t.update.upToDate
        ) : (
          t.update.none
        )}
        {offerUpdate ? (
          <Button variant="ghost" size="sm" className="ml-3" disabled={flow.busy} onClick={skipUpdate}>
            {t.update.skip}
          </Button>
        ) : null}
      </div>
      <NotesDialog open={notesOpen} onClose={() => setNotesOpen(false)} />
    </section>
  );
}

function labelFor(
  copy: { downloading: string; installing: string; ready: string },
  stage: string,
  percent: number,
  downloaded: number,
  total: number,
  fallback: string,
) {
  if (stage === "install") return copy.installing;
  if (stage === "download" && total > 0) {
    return `${copy.downloading} ${formatBytes(downloaded)} / ${formatBytes(total)} · ${percent}%`;
  }
  if (stage === "download" && downloaded > 0) {
    return `${copy.downloading} ${formatBytes(downloaded)}`;
  }
  if (stage === "download") return copy.downloading;
  return fallback || copy.ready;
}

function formatBytes(n: number) {
  if (n >= 1_048_576) return `${(n / 1_048_576).toFixed(1)} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
}

function formatPublished(iso?: string | null) {
  if (!iso) return null;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return null;
  return date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

function plainPreview(notes: string) {
  return notes
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/[#>*_`~\-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}
