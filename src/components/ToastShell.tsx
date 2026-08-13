import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { CheckCircle2, Info, TriangleAlert, X } from "lucide-react";
import { api, onToast } from "../lib/api";
import { playChime } from "../lib/chime";
import { detectLocale, dictionaries, subscribeLocale } from "../i18n";
import type { ToastPayload } from "../types";
import { cn } from "../lib/cn";

export function ToastShell() {
  const [locale, setLocale] = useState(detectLocale);
  const t = dictionaries[locale];
  const [toast, setToast] = useState<ToastPayload | null>(null);
  const [lifeKey, setLifeKey] = useState(0);

  useEffect(() => subscribeLocale(setLocale), []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void (async () => {
      if (await getCurrentWindow().isVisible()) {
        const last = await api.lastToast();
        if (last) {
          setToast(last);
          setLifeKey((n) => n + 1);
        }
      }
      unlisten = await onToast((next) => {
        setToast(next);
        setLifeKey((n) => n + 1);
        playChime();
      });
    })();
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => {
      void api.hideToast();
    }, 5600);
    return () => window.clearTimeout(timer);
  }, [toast, lifeKey]);

  async function activate() {
    if (toast?.page) {
      await api.openPage(toast.page);
    } else {
      await api.focusMain();
      await api.hideToast();
    }
  }

  if (!toast) {
    return <div className="h-full w-full bg-transparent" />;
  }

  const kind = toast.kind === "ok" || toast.kind === "warn" || toast.kind === "error" ? toast.kind : "info";
  const Icon = kind === "ok" ? CheckCircle2 : kind === "warn" || kind === "error" ? TriangleAlert : Info;

  return (
    <div className="flyout flyout-toast">
      <div className={cn("flyout-accent", `flyout-accent-${kind}`)} />
      <button type="button" className="flyout-toast-hit" onClick={() => void activate()}>
        <div className={cn("flyout-icon", `flyout-icon-${kind}`)}>
          <Icon size={16} />
        </div>
        <div className="min-w-0 flex-1 text-left">
          <div className="text-[13px] font-semibold leading-tight text-mist-50">{toast.title}</div>
          <p className="mt-1 line-clamp-2 text-[12px] leading-4 text-mist-300">{toast.body}</p>
          {toast.page ? <div className="mt-1.5 text-[10px] font-medium tracking-wide text-elixir-400">{t.toast.click}</div> : null}
        </div>
      </button>
      <button
        type="button"
        className="flyout-close"
        aria-label={t.common.dismiss}
        onClick={() => void api.hideToast()}
      >
        <X size={13} />
      </button>
      <div key={lifeKey} className={cn("toast-life", `toast-life-${kind}`)} />
    </div>
  );
}
