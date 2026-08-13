import { ArrowRight } from "lucide-react";
import { useState } from "react";
import { useApp } from "../state";
import { Button, Dot } from "../components/ui";
import { api } from "../lib/api";
import { cn } from "../lib/cn";
import type { BinaryHit } from "../types";

export function HomePage() {
  const { t, catalog, toolchains, setPage, probe, refreshProbe } = useApp();
  const active = toolchains.find((x) => x.isActive);
  const hits: Array<{ label: string; hit: BinaryHit | null | undefined }> = [
    { label: "Elixir", hit: probe?.elixir },
    { label: "OTP", hit: probe?.erlang },
    { label: "Mix", hit: probe?.mix },
    { label: "Git", hit: probe?.git },
  ];
  const pathHits = hits.map((h) => h.hit).filter((h): h is BinaryHit => Boolean(h?.needsPathFix));

  return (
    <div className="page-enter mx-auto flex min-h-full max-w-3xl flex-col gap-8 p-8">
      <div>
        <h1 className="text-[1.75rem] font-semibold tracking-tight text-mist-50">{t.home.title}</h1>
        <p className="mt-2 max-w-xl text-[14px] leading-6 text-mist-300">{t.home.body}</p>
      </div>

      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[13px]">
        <span className="text-mist-50">{active?.elixir ?? catalog?.recommendedElixir ?? "Elixir"}</span>
        <span className="text-white/20">/</span>
        <span className="text-mist-300">OTP {active?.otp ?? catalog?.recommendedOtp ?? "—"}</span>
        {active ? <span className="text-ok-400">{t.home.already}</span> : null}
      </div>

      <div className="grid gap-2 sm:grid-cols-3">
        <HomeGo
          title={t.pages.install}
          hint={t.home.installHint}
          onClick={() => setPage("install")}
          primary
        />
        <HomeGo title={t.pages.projects} hint={t.home.projectsHint} onClick={() => setPage("projects")} />
        <HomeGo title={t.pages.doctor} hint={t.home.doctorHint} onClick={() => setPage("doctor")} />
      </div>

      <div className="surface overflow-hidden rounded-xl">
        {hits.map((row) => (
          <div key={row.label} className="flex items-center gap-3 border-b border-white/6 px-4 py-2 last:border-b-0">
            <Dot tone={!row.hit ? "bad" : row.hit.callable ? "ok" : "warn"} />
            <span className="w-14 shrink-0 text-[13px] text-mist-100">{row.label}</span>
            <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-mist-300">
              {hitText(row.hit, t.tray.missing)}
            </span>
          </div>
        ))}
      </div>

      {pathHits.length ? (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-warn-400/25 bg-warn-400/8 px-4 py-3">
          <p className="text-[13px] text-warn-400">{t.home.notOnPath}</p>
          <PathFix hits={pathHits} t={t} onFixed={refreshProbe} />
        </div>
      ) : null}
    </div>
  );
}

function hitText(hit: BinaryHit | null | undefined, missing: string) {
  if (!hit) return missing;
  const version = hit.version?.trim().split(/\r?\n/)[0];
  if (version) return version;
  const tail = hit.path.replace(/\\/g, "/").split("/").filter(Boolean).slice(-3).join("/");
  return tail || hit.source;
}

function HomeGo({
  title,
  hint,
  onClick,
  primary,
}: {
  title: string;
  hint: string;
  onClick: () => void;
  primary?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex flex-col items-start gap-1 rounded-xl px-4 py-3.5 text-left transition",
        primary ? "bg-elixir-600 text-white hover:bg-elixir-700" : "surface hover:bg-white/6",
      )}
    >
      <span className="flex items-center gap-1.5 text-[13px] font-medium">
        {title}
        {primary ? <ArrowRight size={14} /> : null}
      </span>
      <span className={cn("text-[12px] leading-4", primary ? "text-white/75" : "text-mist-300")}>{hint}</span>
    </button>
  );
}

function PathFix({
  hits,
  t,
  onFixed,
}: {
  hits: BinaryHit[];
  t: ReturnType<typeof useApp>["t"];
  onFixed: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  return (
    <div className="flex min-w-0 flex-col items-end gap-1">
      <Button
        size="sm"
        disabled={busy}
        onClick={() => {
          setBusy(true);
          setNote(null);
          void (async () => {
            try {
              const msgs: string[] = [];
              for (const hit of hits) {
                msgs.push(await api.addToPath(hit.name));
              }
              await onFixed();
              setNote(msgs.filter(Boolean).join(" "));
            } catch (err) {
              setNote(err instanceof Error ? err.message : String(err));
            } finally {
              setBusy(false);
            }
          })();
        }}
      >
        {busy ? "…" : t.home.addToPath}
      </Button>
      {note ? <p className="max-w-sm text-right text-[11px] text-mist-300">{note}</p> : null}
    </div>
  );
}
