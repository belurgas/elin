import { Button, Pill } from "../components/ui";
import type { KitStatus, ScanReport } from "../types";
import type { Dictionary } from "../i18n";

export function QualityStudio({
  kits,
  report,
  busy,
  t,
  onScan,
  onFull,
  onFormat,
  onApply,
  onRemove,
  onWriteConfig,
  onOpenConfig,
  onCredoStrict,
}: {
  kits: KitStatus[];
  report: ScanReport | null;
  busy: boolean;
  t: Dictionary;
  onScan: () => void;
  onFull: () => void;
  onFormat: (check: boolean) => void;
  onApply: (id: string) => void;
  onRemove: (id: string) => void;
  onWriteConfig: (id: string) => void;
  onOpenConfig: (file: string) => void;
  onCredoStrict: (strict: boolean) => void;
}) {
  const p = t.projects;
  const w = t.workspace;
  return (
    <div className="studio-stage-enter grid h-full min-h-0 gap-4 overflow-hidden p-4 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
      <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
        <div className="studio-card shrink-0">
          <div className="mb-3 text-[11px] font-medium uppercase tracking-wide text-mist-300">{p.tabTools}</div>
          <div className="flex flex-wrap gap-2">
            <Button disabled={busy} onClick={onScan}>
              {p.scanCode}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={onFull}>
              {p.scanFull}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => onFormat(false)}>
              {p.formatFix}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={() => onFormat(true)}>
              {p.formatCheck}
            </Button>
          </div>
          <p className="mt-3 text-[11px] leading-4 text-mist-300">{t.workspace.scanHint}</p>
        </div>
        <div className="studio-card">
          <div className="mb-1 text-[11px] font-medium uppercase tracking-wide text-mist-300">{p.kits}</div>
          <p className="mb-3 text-[11px] leading-4 text-mist-300">{p.kitsHint}</p>
          <div className="grid gap-2">
            {kits.map((status) => (
              <div key={status.kit.id} className="rounded-lg px-2 py-2 hover:bg-white/4">
                <div className="flex items-start gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <span className="text-[13px] font-medium">{status.kit.name}</span>
                      <Pill tone={status.installed ? "ok" : "mute"}>{status.installed ? p.kitOn : p.kitOff}</Pill>
                      {status.kit.advanced ? <Pill>{p.kitAdvanced}</Pill> : null}
                      {status.kit.phoenixOnly ? <Pill tone="rose">{p.phoenixOnly}</Pill> : null}
                    </div>
                    <p className="mt-0.5 text-[11px] leading-4 text-mist-300">{status.kit.summary}</p>
                    {status.kit.configFile ? (
                      <div className="mt-1.5 flex flex-wrap items-center gap-2">
                        <span className="font-mono text-[10px] text-mist-300">
                          {status.kit.configFile}
                          {status.configPresent ? "" : ` · ${w.configMissing}`}
                        </span>
                        {status.configPresent ? (
                          <button
                            type="button"
                            className="text-[11px] text-elixir-300 hover:text-white"
                            onClick={() => onOpenConfig(status.kit.configFile!)}
                          >
                            {w.openConfig}
                          </button>
                        ) : (
                          <button
                            type="button"
                            className="text-[11px] text-elixir-300 hover:text-white"
                            disabled={busy}
                            onClick={() => onWriteConfig(status.kit.id)}
                          >
                            {w.writeConfig}
                          </button>
                        )}
                        {status.kit.id === "credo" && status.installed ? (
                          <label className="ml-1 flex cursor-pointer items-center gap-1.5 text-[11px] text-mist-100">
                            <input
                              type="checkbox"
                              checked={Boolean(status.credoStrict)}
                              disabled={busy}
                              onChange={(e) => onCredoStrict(e.target.checked)}
                            />
                            {w.credoStrict}
                          </label>
                        ) : null}
                      </div>
                    ) : null}
                  </div>
                  {status.kit.hex ? (
                    status.installed ? (
                      <Button variant="ghost" size="sm" disabled={busy} onClick={() => onRemove(status.kit.id)}>
                        {p.removeKit}
                      </Button>
                    ) : (
                      <Button size="sm" disabled={busy} onClick={() => onApply(status.kit.id)}>
                        {p.applyKit}
                      </Button>
                    )
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
      <div className="studio-card min-h-0 overflow-y-auto">
        <div className="mb-3 text-[11px] font-medium uppercase tracking-wide text-mist-300">{p.findings}</div>
        {report ? (
          <>
            <ul className="mb-4 grid gap-1 text-[12px]">
              {report.layers.map((layer) => (
                <li key={layer.id} className="flex gap-2">
                  <span className={layer.ok ? "text-ok-400" : "text-otp-400"}>
                    {layer.ran ? (layer.ok ? "ok" : "!!") : p.skipped}
                  </span>
                  <span>{layer.name}</span>
                  <span className="truncate text-mist-300">{layer.detail}</span>
                </li>
              ))}
            </ul>
            {report.findings.length === 0 ? (
              <p className="text-sm text-mist-300">—</p>
            ) : (
              <ul className="grid gap-1.5 font-mono text-[12px]">
                {report.findings.map((f, i) => (
                  <li key={`${f.tool}-${i}`}>
                    <span className={f.severity === "error" ? "text-otp-400" : "text-warn-400"}>{f.severity}</span>{" "}
                    {f.file}
                    {f.line ? `:${f.line}` : ""} {f.message}
                  </li>
                ))}
              </ul>
            )}
          </>
        ) : (
          <p className="text-sm text-mist-300">{t.workspace.scanHint}</p>
        )}
      </div>
    </div>
  );
}
