import { api } from "../../lib/api";
import { Button, Card, Menu, Pill } from "../../components/ui";
import type { MixProject } from "../../types";
import type { Labels } from "./types";

export function Overview({
  project,
  toolchains,
  busy,
  t,
  onBusy,
  onError,
  onProject,
}: {
  project: MixProject;
  toolchains: Array<{ elixir: string; otp: string }>;
  busy: boolean;
  t: Labels;
  onBusy: (v: boolean) => void;
  onError: (v: string) => void;
  onProject: (p: MixProject) => Promise<void>;
}) {
  return (
    <Card>
      <div className="text-[11px] text-mist-300">Elixir</div>
      {project.elixirReq ? (
        <p className="mt-2 text-sm text-mist-100">
          {t.elixirReq} <span className="font-mono text-elixir-300">{project.elixirReq}</span>
        </p>
      ) : null}
      {project.pinnedElixir ? (
        <p className="mt-1 text-sm">
          {t.pinned}{" "}
          <span className="font-mono text-elixir-300">
            {project.pinnedElixir}
            {project.pinnedOtp ? ` · OTP ${project.pinnedOtp}` : ""}
          </span>
        </p>
      ) : project.resolvedElixir ? (
        <p className="mt-1 text-sm">
          {t.matching}{" "}
          <span className="font-mono text-elixir-300">
            {project.resolvedElixir}
            {project.resolvedOtp ? ` · OTP ${project.resolvedOtp}` : ""}
          </span>
        </p>
      ) : (
        <p className="mt-2 text-sm text-mist-300">{t.noMatch}</p>
      )}
      <p className="mt-2 text-xs text-mist-300">{t.pinHint}</p>
      <div className="mt-4 flex flex-wrap items-center gap-2">
        {!project.resolvedElixir && project.elixirReq ? (
          <Button
            disabled={busy}
            onClick={async () => {
              onBusy(true);
              try {
                await onProject(await api.installProjectToolchain(project.path));
              } catch (err) {
                onError(err instanceof Error ? err.message : String(err));
              } finally {
                onBusy(false);
              }
            }}
          >
            {busy ? t.installing : t.installMatch}
          </Button>
        ) : project.resolvedElixir ? (
          <Pill tone="ok">{t.alreadyPinned}</Pill>
        ) : null}
        {toolchains.length ? (
          <Menu
            placeholder={t.pinVersion}
            value={
              project.pinnedElixir
                ? `${project.pinnedElixir}::${project.pinnedOtp ?? ""}`
                : project.resolvedElixir
                  ? `${project.resolvedElixir}::${project.resolvedOtp ?? ""}`
                  : ""
            }
            options={toolchains.map((pair) => ({
              value: `${pair.elixir}::${pair.otp}`,
              label: `Elixir ${pair.elixir} · OTP ${pair.otp}`,
            }))}
            onChange={(value) => {
              const [elixir, otp] = value.split("::");
              void api
                .pinProjectToolchain(project.path, elixir, otp)
                .then(onProject)
                .catch((err) => onError(err instanceof Error ? err.message : String(err)));
            }}
          />
        ) : null}
      </div>
    </Card>
  );
}
