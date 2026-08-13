import { api } from "../lib/api";
import { useApp } from "../state";
import { Button, PageShell, Pill } from "../components/ui";

export function ToolchainPage() {
  const { t, toolchains, refreshToolchains } = useApp();
  const onlyOne = toolchains.length === 1;
  const active = toolchains.find((p) => p.isActive);

  return (
    <PageShell title={t.toolchain.title} subtitle={onlyOne && active ? t.toolchain.onlyOne : t.toolchain.sideBySide}>
      {toolchains.length === 0 ? (
        <p className="text-[13px] text-mist-300">{t.toolchain.empty}</p>
      ) : (
        <div className="surface divide-y divide-white/6 overflow-hidden rounded-xl">
          {toolchains.map((pair) => (
            <div key={`${pair.elixir}-${pair.otp}`} className="flex items-center gap-3 px-4 py-2.5">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[13px]">Elixir {pair.elixir}</span>
                  {pair.isActive ? <Pill tone="ok">{t.toolchain.active}</Pill> : null}
                </div>
                <div className="truncate font-mono text-[11px] text-mist-300">
                  OTP {pair.otp} · {pair.elixirPath}
                </div>
              </div>
              <div className="flex gap-1.5">
                {pair.isActive ? null : (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void api.activate(pair.elixir, pair.otp).then(() => refreshToolchains())}
                  >
                    {t.toolchain.activate}
                  </Button>
                )}
                <Button
                  variant="danger"
                  size="sm"
                  onClick={() => void api.remove(pair.elixir, pair.otp).then(() => refreshToolchains())}
                >
                  {t.toolchain.remove}
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </PageShell>
  );
}
