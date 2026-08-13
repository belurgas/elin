import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { HeartPulse, FolderGit2, FlaskConical, LogOut, Sparkles, ArrowDownToLine } from "lucide-react";
import { api } from "../lib/api";
import { detectLocale, dictionaries, subscribeLocale } from "../i18n";
import type { AppUpdate, InstalledPair } from "../types";
import { cn } from "../lib/cn";

export function TrayShell() {
  const [locale, setLocale] = useState(detectLocale);
  const t = dictionaries[locale];
  const [pairs, setPairs] = useState<InstalledPair[]>([]);
  const [update, setUpdate] = useState<AppUpdate | null>(null);

  useEffect(() => subscribeLocale(setLocale), []);

  useEffect(() => {
    void api.toolchains().then(setPairs).catch(() => undefined);
    void api
      .checkAppUpdate(false)
      .then((next) => {
        if (next.newer) setUpdate(next);
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    const opened = Date.now();
    const onBlur = () => {
      if (Date.now() - opened < 220) return;
      void getCurrentWindow().hide();
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, []);

  const active = pairs.find((p) => p.isActive);
  const elixirOk = Boolean(active);
  const callable = elixirOk;
  const elixirLine = active ? `Elixir ${active.elixir}` : t.tray.missing;
  const otpLine = active ? `OTP ${active.otp}` : "OTP —";

  return (
    <div className="flyout flyout-tray">
      <div data-tauri-drag-region className="drag-region flex items-center gap-3 px-4 pb-3 pt-4">
        <img src="/elin.svg" alt="" className="size-8 rounded-lg" />
        <div className="min-w-0">
          <div className="font-display text-[17px] leading-none text-mist-50">{t.app}</div>
          <div className="mt-1 truncate text-[11px] text-mist-300">{t.tagline}</div>
        </div>
      </div>

      <div className="mx-3 flex items-center gap-3 rounded-xl bg-white/[0.04] px-3 py-2.5">
        <span
          className={cn(
            "size-2 shrink-0 rounded-full",
            elixirOk && callable ? "bg-ok-400" : elixirOk ? "bg-warn-400" : "bg-otp-400",
          )}
        />
        <div className="min-w-0 flex-1">
          <div className="truncate font-mono text-[12px] text-elixir-300">{elixirLine}</div>
          <div className="mt-0.5 flex items-center gap-2 text-[11px]">
            <span className="truncate font-mono text-otp-400">{otpLine}</span>
            <span
              className={cn(
                "shrink-0",
                elixirOk && callable ? "text-ok-400" : elixirOk ? "text-warn-400" : "text-otp-400",
              )}
            >
              {!elixirOk ? t.tray.missing : callable ? t.tray.ready : t.tray.pathWarn}
            </span>
          </div>
        </div>
      </div>

      <div className="no-drag mt-2 flex flex-1 flex-col px-2">
        <TrayRow icon={Sparkles} label={t.tray.open} onClick={() => void api.focusMain()} />
        {update ? (
          <TrayRow
            icon={ArrowDownToLine}
            label={t.update.available.replace("{version}", update.latest)}
            onClick={() => void api.openPage("settings")}
          />
        ) : null}
        <TrayRow icon={FlaskConical} label={t.tray.install} onClick={() => void api.openPage("install")} />
        <TrayRow icon={HeartPulse} label={t.tray.doctor} onClick={() => void api.openPage("doctor")} />
        <TrayRow icon={FolderGit2} label={t.tray.projects} onClick={() => void api.openPage("projects")} />
      </div>

      <div className="no-drag mx-3 mb-3 mt-1 border-t border-white/[0.06] pt-2">
        <button
          type="button"
          onClick={() => void api.quit()}
          className="flex w-full cursor-pointer items-center gap-2.5 rounded-lg px-3 py-2 text-[13px] text-otp-400 hover:bg-otp-500/12"
        >
          <LogOut size={15} />
          {t.tray.quit}
        </button>
      </div>
    </div>
  );
}

function TrayRow({
  icon: Icon,
  label,
  onClick,
}: {
  icon: typeof Sparkles;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full cursor-pointer items-center gap-2.5 rounded-lg px-3 py-2 text-left text-[13px] text-mist-100 hover:bg-white/[0.06] active:bg-white/[0.09]"
    >
      <Icon size={15} className="text-elixir-400" />
      {label}
    </button>
  );
}
