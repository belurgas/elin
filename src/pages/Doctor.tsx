import { useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import { useApp } from "../state";
import { Button, Card, Dot, PageShell } from "../components/ui";
import type { DoctorReport } from "../types";
import { cn } from "../lib/cn";

const GROUP_KEYS = ["runtime", "path", "tooling", "system"] as const;

export function DoctorPage() {
  const { t, refreshProbe, probe } = useApp();
  const [report, setReport] = useState<DoctorReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [openId, setOpenId] = useState<string | null>(null);

  async function run() {
    setError(null);
    setLoading(true);
    try {
      const next = await api.doctor();
      setReport(next);
      const firstFail = next.checks.find((c) => !c.ok);
      setOpenId(firstFail?.id ?? null);
      await refreshProbe();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const grouped = useMemo(() => {
    const checks = report?.checks ?? [];
    return GROUP_KEYS.map((group) => ({
      group,
      items: checks.filter((c) => c.group === group),
    })).filter((g) => g.items.length);
  }, [report]);

  async function fix(fixId: string) {
    setBusy(fixId);
    try {
      await api.doctorFix(fixId);
      await run();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function copy() {
    if (!report) return;
    const text = report.checks
      .map((c) => `${c.ok ? "OK" : "FAIL"} [${c.group}] ${c.title}\n  ${c.detail}${c.path ? `\n  ${c.path}` : ""}`)
      .join("\n\n");
    await navigator.clipboard.writeText(text);
  }

  const groupLabel: Record<(typeof GROUP_KEYS)[number], string> = {
    runtime: t.doctor.groupRuntime,
    path: t.doctor.groupPath,
    tooling: t.doctor.groupTooling,
    system: t.doctor.groupSystem,
  };

  const failed = report?.checks.filter((c) => !c.ok).length ?? 0;
  const elixir =
    report?.elixirVersion ?? report?.probe.elixir?.version ?? (report?.probe.elixir ? t.doctor.system : t.doctor.unknown);
  const otp =
    report?.erlangVersion ?? report?.probe.erlang?.version ?? (report?.probe.erlang ? t.doctor.system : t.doctor.unknown);

  return (
    <PageShell
      title={t.doctor.title}
      subtitle={t.doctor.subtitle}
      actions={
        <div className="flex gap-2">
          <Button variant="ghost" size="sm" onClick={() => void copy()}>
            {t.doctor.copy}
          </Button>
          <Button size="sm" onClick={() => void run()}>
            {t.doctor.rerun}
          </Button>
        </div>
      }
    >
      {error ? <Card className="text-sm text-otp-400">{error}</Card> : null}
      {loading && !report ? (
        <Card className="text-sm text-mist-300">
          {t.doctor.checking}
          {probe?.notes?.length ? (
            <ul className="mt-2 grid gap-1 text-[13px] text-mist-100">
              {probe.notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          ) : null}
        </Card>
      ) : null}
      {report ? (
        <>
          <div className="flex flex-wrap items-baseline gap-x-6 gap-y-1 text-[13px]">
            <span className="text-2xl font-semibold tabular-nums text-mist-50">{report.score}</span>
            <span className="text-mist-300">{t.doctor.score}</span>
            <span className="font-mono text-elixir-300">Elixir {elixir}</span>
            <span className="font-mono text-mist-300">OTP {otp}</span>
            {failed ? <span className="text-warn-400">{failed}</span> : <span className="text-ok-400">ok</span>}
          </div>
          {report.probe.elixir ? (
            <div className="selectable -mt-3 truncate font-mono text-[11px] text-mist-300">{report.probe.elixir.path}</div>
          ) : null}

          <Card className="p-0">
            {grouped.map(({ group, items }, gi) => (
              <div key={group} className={cn(gi > 0 && "border-t border-white/6")}>
                <div className="px-4 pb-1 pt-3 text-[11px] text-mist-300">{groupLabel[group]}</div>
                {items.map((check) => {
                  const open = openId === check.id || (!check.ok && openId === null);
                  const tone = check.ok ? "ok" : check.severity === "warn" ? "warn" : "bad";
                  return (
                    <div key={check.id} className="border-t border-white/4 first:border-t-0">
                      <div className="flex items-center gap-3 px-4 py-2">
                        <button
                          type="button"
                          onClick={() => setOpenId(open && openId === check.id ? null : check.id)}
                          className="flex min-w-0 flex-1 items-center gap-3 text-left"
                        >
                          <Dot tone={tone} />
                          <span className="min-w-0 flex-1 truncate text-[13px]">{check.title}</span>
                        </button>
                        {check.fixId && !check.ok ? (
                          <Button size="sm" disabled={busy !== null} onClick={() => void fix(check.fixId!)}>
                            {busy === check.fixId ? "…" : t.doctor.fix}
                          </Button>
                        ) : null}
                      </div>
                      {open && (!check.ok || openId === check.id) ? (
                        <div className="space-y-1 px-4 pb-3 pl-[2.15rem] text-[12px] leading-5 text-mist-300">
                          <p>{check.detail}</p>
                          {check.path ? <p className="selectable font-mono text-[11px] text-elixir-300">{check.path}</p> : null}
                          {!check.ok ? <p className="text-elixir-300">{check.hint}</p> : null}
                        </div>
                      ) : null}
                    </div>
                  );
                })}
              </div>
            ))}
          </Card>
        </>
      ) : null}
    </PageShell>
  );
}
