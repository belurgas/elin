import { memo } from "react";
import {
  Activity,
  BookOpen,
  Boxes,
  FlaskConical,
  FolderGit2,
  HeartPulse,
  Home,
  PackageSearch,
  Puzzle,
  Settings,
  Wrench,
} from "lucide-react";
import { useData, useNav } from "../state";
import type { PageId } from "../types";
import { cn } from "../lib/cn";

const items: Array<{ id: PageId; icon: typeof Home }> = [
  { id: "home", icon: Home },
  { id: "install", icon: FlaskConical },
  { id: "toolchain", icon: Boxes },
  { id: "studios", icon: Wrench },
  { id: "plugins", icon: Puzzle },
  { id: "doctor", icon: HeartPulse },
  { id: "projects", icon: FolderGit2 },
  { id: "playground", icon: Activity },
  { id: "hex", icon: PackageSearch },
  { id: "learn", icon: BookOpen },
  { id: "settings", icon: Settings },
];

export const Sidebar = memo(function Sidebar() {
  const { page, setPage, t } = useNav();
  const { catalog, toolchains, probe } = useData();
  const active = toolchains.find((x) => x.isActive) ?? toolchains[0];
  const elixirLabel = active
    ? `elixir ${active.elixir}`
    : probe?.elixir?.version
      ? probe.elixir.version.split("\n")[0]
      : catalog?.recommendedElixir ?? "—";

  return (
    <aside className="flex w-[196px] shrink-0 flex-col border-r border-white/8 bg-ink-900/40 [contain:layout_paint]">
      <nav className="flex-1 space-y-px overflow-y-auto p-2">
        {items.map(({ id, icon: Icon }) => (
          <button
            type="button"
            key={id}
            onClick={() => setPage(id)}
            className={cn(
              "flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-[13px] transition duration-150",
              page === id
                ? "bg-white/8 text-white"
                : "text-mist-300 hover:bg-white/5 hover:text-white",
            )}
          >
            <Icon size={15} className={page === id ? "text-elixir-400" : "opacity-70"} />
            {t.pages[id]}
          </button>
        ))}
      </nav>
      <div className="m-2 rounded-lg border border-white/8 px-2.5 py-2">
        <div className="truncate font-mono text-[11px] text-elixir-300">{elixirLabel}</div>
        <div className="font-mono text-[11px] text-mist-300">
          {active ? `otp ${active.otp}` : catalog?.recommendedOtp ?? "—"}
        </div>
        {probe?.elixir && !probe.userPathHasElixir ? (
          <div className="mt-1 text-[10px] leading-4 text-warn-400">{t.home.notOnPath}</div>
        ) : null}
      </div>
    </aside>
  );
});
