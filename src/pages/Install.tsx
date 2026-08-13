import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import { useApp } from "../state";
import { Button, Card, Checkbox, Chip, PageShell, Pill } from "../components/ui";
import { cn } from "../lib/cn";
import type { InstallProgress } from "../types";

export function InstallPage() {
  const {
    t,
    catalog,
    catalogError,
    refreshCatalog,
    includePrerelease,
    setIncludePrerelease,
    refreshToolchains,
    toolchains,
  } = useApp();
  const [tab, setTab] = useState<"elixir" | "otp">("elixir");
  const [elixir, setElixir] = useState(catalog?.recommendedElixir ?? "");
  const [otp, setOtp] = useState(catalog?.recommendedOtp ?? "");
  const [addPath, setAddPath] = useState(true);
  const [installHex, setInstallHex] = useState(true);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [log, setLog] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (catalog?.recommendedElixir && !elixir) setElixir(catalog.recommendedElixir);
    if (catalog?.recommendedOtp && !otp) setOtp(catalog.recommendedOtp);
  }, [catalog, elixir, otp]);

  const selectedElixir = catalog?.elixir.find((x) => x.version === elixir);
  const compatible = useMemo(() => {
    if (!catalog || !selectedElixir) return [];
    return catalog.otp.filter((o) => selectedElixir.otpMajors.includes(o.major) && !o.isPrerelease);
  }, [catalog, selectedElixir]);

  const pairOk = Boolean(selectedElixir && compatible.some((o) => o.version === otp));
  const installedPair = toolchains.find(
    (p) => p.elixir === elixir && (p.otp === otp || p.otp.startsWith(`${otp.split(".")[0]}.`)),
  );
  const elixirInstalled = toolchains.some((p) => p.elixir === elixir);

  async function start() {
    if (!elixir || !otp) return;
    setBusy(true);
    setError(null);
    setLog("");
    setProgress({ stage: "start", message: t.common.loading, percent: 1 });
    const unlisten = await listen<InstallProgress>("install-progress", (event) => {
      setProgress(event.payload);
    });
    try {
      const result = await api.install(elixir, otp, addPath, installHex);
      setLog(result.elixirVersionOutput);
      await refreshToolchains();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      unlisten();
      setBusy(false);
    }
  }

  async function makeDefault() {
    if (!elixir || !otp) return;
    setBusy(true);
    setError(null);
    try {
      await api.activate(elixir, otp);
      await refreshToolchains();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <PageShell
      title={t.install.title}
      subtitle={t.install.subtitle}
      actions={
        <div className="flex items-center gap-3">
          <Checkbox
            checked={includePrerelease}
            onChange={(next) => {
              setIncludePrerelease(next);
              void refreshCatalog(next);
            }}
          >
            {t.install.prerelease}
          </Checkbox>
          <Button variant="ghost" onClick={() => void refreshCatalog(includePrerelease, true)}>
            {t.install.refresh}
          </Button>
        </div>
      }
    >
      {catalogError ? <Card className="text-sm text-otp-400">{catalogError}</Card> : null}

      <Card className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <Chip active={tab === "elixir"} onClick={() => setTab("elixir")}>
            {t.install.elixir} {elixir ? `· ${elixir}` : ""}
          </Chip>
          <Chip active={tab === "otp"} onClick={() => setTab("otp")}>
            {t.install.otp} {otp ? `· ${otp}` : ""}
          </Chip>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {installedPair ? <Pill tone="ok">{t.install.installed}</Pill> : null}
          {installedPair?.isActive ? <Pill>{t.install.alreadyDefault}</Pill> : null}
        </div>
      </Card>

      {tab === "elixir" ? (
        <Card>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold">{t.install.elixir}</h2>
            <Pill>{t.install.latest}</Pill>
          </div>
          <div className="max-h-[360px] space-y-1 overflow-y-auto pr-1">
            {(catalog?.elixir ?? []).slice(0, 40).map((rel) => {
              const onDisk = toolchains.some((p) => p.elixir === rel.version);
              return (
                <button
                  key={rel.version}
                  onClick={() => {
                    setElixir(rel.version);
                    const next =
                      catalog?.otp.find(
                        (o) => rel.otpMajors.includes(o.major) && o.version === catalog.recommendedOtp,
                      ) ?? catalog?.otp.find((o) => rel.otpMajors.includes(o.major));
                    if (next) setOtp(next.version);
                    setTab("otp");
                  }}
                  className={cn(
                    "flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-1.5 text-left text-sm",
                    elixir === rel.version ? "bg-elixir-600/25" : "hover:bg-white/5",
                  )}
                >
                  <span className="font-mono">{rel.version}</span>
                  <span className="flex gap-1">
                    {onDisk ? <Pill tone="ok">{t.install.installed}</Pill> : null}
                    {rel.isLatest ? <Pill>{t.install.latest}</Pill> : null}
                    {rel.version === catalog?.recommendedElixir ? <Pill tone="ok">{t.install.recommended}</Pill> : null}
                  </span>
                </button>
              );
            })}
          </div>
        </Card>
      ) : (
        <Card>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold">{t.install.otp}</h2>
            <span className="text-xs text-mist-300">
              {t.install.compatible}: {selectedElixir?.otpMajors.join(", ") ?? "—"}
            </span>
          </div>
          <div className="max-h-[360px] space-y-1 overflow-y-auto pr-1">
            {(catalog?.otp ?? []).slice(0, 50).map((rel) => {
              const ok = selectedElixir?.otpMajors.includes(rel.major);
              const onDisk = elixirInstalled && toolchains.some((p) => p.elixir === elixir && p.otp === rel.version);
              return (
                <button
                  key={rel.version}
                  disabled={!ok}
                  onClick={() => setOtp(rel.version)}
                  className={cn(
                    "flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-1.5 text-left text-sm disabled:cursor-not-allowed disabled:opacity-30",
                    otp === rel.version ? "bg-otp-500/20" : "hover:bg-white/5",
                  )}
                >
                  <span className="font-mono">{rel.version}</span>
                  <span className="flex gap-1">
                    {onDisk ? <Pill tone="ok">{t.install.installed}</Pill> : null}
                    {rel.isLatest ? <Pill tone="rose">{t.install.latest}</Pill> : null}
                    {rel.version === catalog?.recommendedOtp ? <Pill tone="ok">{t.install.recommended}</Pill> : null}
                  </span>
                </button>
              );
            })}
          </div>
        </Card>
      )}

      {!pairOk && elixir && otp ? (
        <div className="rounded-lg border border-otp-500/30 bg-otp-500/10 px-3 py-2 text-sm text-otp-400">
          {t.install.warning}
        </div>
      ) : null}

      <Card className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap gap-5 text-sm">
          <Checkbox checked={addPath} onChange={setAddPath}>
            {t.install.addPath}
          </Checkbox>
          <Checkbox checked={installHex} onChange={setInstallHex}>
            {t.install.hex}
          </Checkbox>
        </div>
        {installedPair?.isActive ? (
          <Pill tone="ok">{t.install.alreadyDefault}</Pill>
        ) : installedPair ? (
          <Button disabled={busy} onClick={() => void makeDefault()}>
            {t.install.makeDefault}
          </Button>
        ) : (
          <Button disabled={!pairOk || busy} onClick={() => void start()}>
            {busy ? `${progress?.percent ?? 0}%` : t.install.start}
          </Button>
        )}
      </Card>

      {busy || progress || log || error ? (
        <Card>
          <div className="mb-3 flex items-center justify-between text-sm">
            <span>{progress?.message ?? (error ? t.common.error : "elixir -v")}</span>
            <span className="font-mono text-elixir-300">{progress?.percent ?? 100}%</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-white/8">
            <div
              className="h-full rounded-full bg-gradient-to-r from-elixir-600 to-otp-400 transition-all"
              style={{ width: `${progress?.percent ?? (error ? 0 : 100)}%` }}
            />
          </div>
          {error ? <pre className="selectable mt-3 whitespace-pre-wrap text-xs text-otp-400">{error}</pre> : null}
          {log ? <pre className="selectable mt-3 whitespace-pre-wrap font-mono text-xs text-ok-400">{log}</pre> : null}
        </Card>
      ) : null}
    </PageShell>
  );
}
