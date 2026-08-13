import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { memo, useEffect, useState } from "react";
import { useNav } from "../state";

export const Titlebar = memo(function Titlebar({
  heading,
  caption,
}: {
  heading?: string;
  caption?: string;
}) {
  const { t } = useNav();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    const sync = () => {
      void appWindow.isMaximized().then(setMaximized);
    };
    sync();
    const unlisten = appWindow.onResized(() => sync());
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <header className="relative z-40 flex h-11 shrink-0 items-center border-b border-white/8 bg-ink-900/80">
      <div
        data-tauri-drag-region
        className="drag-region flex h-full min-w-0 flex-1 items-center gap-3 px-3"
        onDoubleClick={() => void getCurrentWindow().toggleMaximize()}
      >
        <img src="/elin.svg" alt="" className="pointer-events-none size-6 rounded-md" />
        <div className="pointer-events-none min-w-0 leading-none">
          <div className="truncate text-[13px] font-semibold text-mist-50">{heading ?? t.app}</div>
        </div>
        <span className="pointer-events-none hidden min-w-0 truncate text-xs text-mist-300/80 sm:inline">
          {caption ?? t.tagline}
        </span>
      </div>
      <div className="no-drag flex items-center">
        <WindowButton
          title="Minimize"
          onClick={() => {
            const win = getCurrentWindow();
            if (win.label === "main") {
              void win.close();
            } else {
              void win.minimize();
            }
          }}
        >
          <Minus size={14} />
        </WindowButton>
        <WindowButton title="Maximize" onClick={() => void getCurrentWindow().toggleMaximize()}>
          <Square size={11} strokeWidth={maximized ? 2.4 : 2} />
        </WindowButton>
        <WindowButton close title="Close" onClick={() => void getCurrentWindow().close()}>
          <X size={14} />
        </WindowButton>
      </div>
    </header>
  );
});

function WindowButton({
  children,
  onClick,
  close,
  title,
}: {
  children: React.ReactNode;
  onClick: () => void;
  close?: boolean;
  title: string;
}) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={
        close
          ? "flex h-11 w-12 cursor-pointer items-center justify-center text-mist-300 hover:bg-[#c43b2e] hover:text-white"
          : "flex h-11 w-12 cursor-pointer items-center justify-center text-mist-300 hover:bg-white/10 hover:text-white"
      }
    >
      {children}
    </button>
  );
}
