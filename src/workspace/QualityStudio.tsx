import { Button, Checkbox, Input, Pill } from "../components/ui";
import type { KitStatus, ScanFinding, ScanReport } from "../types";
import type { Dictionary } from "../i18n";
import { shortPath } from "./paths";
import { cn } from "../lib/cn";
import { ChevronRight } from "lucide-react";
import { useMemo, useState } from "react";

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
  onOpenFinding,
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
  onOpenFinding?: (file: string, line?: number | null) => void;
}) {
  const p = t.projects;
  const w = t.workspace;
  const [findingQuery, setFindingQuery] = useState("");
  const groups = useMemo(() => groupFindings(report?.findings ?? [], findingQuery), [report, findingQuery]);
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
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => onOpenConfig(status.kit.configFile!)}
                          >
                            {w.openConfig}
                          </Button>
                        ) : (
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={busy}
                            onClick={() => onWriteConfig(status.kit.id)}
                          >
                            {w.writeConfig}
                          </Button>
                        )}
                        {status.kit.id === "credo" && status.installed ? (
                          <Checkbox
                            size="sm"
                            className="ml-1"
                            checked={Boolean(status.credoStrict)}
                            disabled={busy}
                            onChange={onCredoStrict}
                          >
                            {w.credoStrict}
                          </Checkbox>
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
              <>
                <div className="mb-2 flex items-center gap-2">
                  <Input
                    size="sm"
                    value={findingQuery}
                    onChange={(e) => setFindingQuery(e.target.value)}
                    placeholder={w.findingsFilter}
                  />
                  <span className="shrink-0 font-mono text-[10px] text-mist-300">
                    {groups.reduce((n, g) => n + g.items.length, 0)}/{report.findings.length}
                  </span>
                </div>
                <div className="grid gap-1">
                  {groups.map((group) => (
                    <FindingGroup
                      key={group.file}
                      group={group}
                      openDefault={groups.length <= 8}
                      onOpen={onOpenFinding}
                    />
                  ))}
                </div>
              </>
            )}
          </>
        ) : (
          <p className="text-sm text-mist-300">{t.workspace.scanHint}</p>
        )}
      </div>
    </div>
  );
}

function FindingGroup({
  group,
  openDefault,
  onOpen,
}: {
  group: { file: string; items: ScanFinding[] };
  openDefault: boolean;
  onOpen?: (file: string, line?: number | null) => void;
}) {
  const [open, setOpen] = useState(openDefault);
  const label = group.file === "—" ? group.file : shortPath(group.file);
  return (
    <div>
      <button
        type="button"
        title={group.file}
        onClick={() => setOpen((v) => !v)}
        className="flex w-full min-w-0 items-center gap-1 rounded-md py-0.5 text-left hover:bg-white/5"
      >
        <ChevronRight size={11} className={cn("shrink-0 text-mist-300 transition", open && "rotate-90")} />
        <span className="min-w-0 truncate font-mono text-[11px] text-elixir-300">{label}</span>
        <span className="ml-auto shrink-0 font-mono text-[10px] text-mist-300">{group.items.length}</span>
      </button>
      {open ? (
        <ul className="mt-0.5 grid gap-0.5 pb-1 pl-4">
          {group.items.map((f, i) => (
            <li key={`${f.tool}-${f.line}-${i}`}>
              <button
                type="button"
                className="w-full rounded-md px-1 py-0.5 text-left text-[12px] leading-4 hover:bg-white/5"
                onClick={() => {
                  if (group.file !== "—") onOpen?.(group.file, f.line);
                }}
              >
                <span className={f.severity === "error" ? "text-otp-400" : "text-warn-400"}>{f.severity}</span>{" "}
                <span className="text-mist-100">{f.message}</span>
                {f.line ? <span className="font-mono text-[10px] text-mist-300"> :{f.line}</span> : null}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function groupFindings(findings: ScanFinding[], query: string) {
  const q = query.trim().toLowerCase();
  const filtered = q
    ? findings.filter(
        (f) =>
          f.message.toLowerCase().includes(q) ||
          (f.file ?? "").toLowerCase().includes(q) ||
          f.tool.toLowerCase().includes(q),
      )
    : findings;
  const map = new Map<string, ScanFinding[]>();
  for (const item of filtered) {
    const key = item.file || "—";
    const list = map.get(key) ?? [];
    list.push(item);
    map.set(key, list);
  }
  return [...map.entries()].map(([file, items]) => ({ file, items }));
}
