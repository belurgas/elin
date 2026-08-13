import { useEffect } from "react";
import { api, pickExecutable } from "../lib/api";
import { useApp } from "../state";
import { Button, Card, PageShell, Pill } from "../components/ui";
import { cn } from "../lib/cn";

export function StudiosPage() {
  const { t, studios, preferredStudioId, setPreferredStudio, toggleStudio, refreshStudios, addStudio, ensureStudios } =
    useApp();

  useEffect(() => {
    void ensureStudios();
  }, [ensureStudios]);

  async function add() {
    const path = await pickExecutable();
    if (!path) return;
    addStudio(await api.importStudio(path));
  }

  return (
    <PageShell
      title={t.studios.title}
      subtitle={t.studios.subtitle}
      actions={
        <div className="flex gap-2">
          <Button variant="ghost" onClick={() => void refreshStudios()}>
            {t.studios.rescan}
          </Button>
          <Button onClick={() => void add()}>{t.studios.add}</Button>
        </div>
      }
    >
      <p className="text-[13px] text-mist-300">{t.studios.defaultEditor}</p>
      <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
        {studios.map((studio) => {
          const isDefault = preferredStudioId === studio.id;
          return (
            <Card key={studio.id} className={cn("p-3", isDefault && "ring-1 ring-elixir-500/50")}>
              <div className="flex items-center gap-3">
                <button type="button" onClick={() => toggleStudio(studio.id)} className="flex min-w-0 flex-1 items-center gap-3 text-left">
                  {studio.iconDataUrl ? (
                    <img src={studio.iconDataUrl} alt="" className="size-8 rounded-md" />
                  ) : (
                    <div className="flex size-8 items-center justify-center rounded-md bg-elixir-600/20 text-sm">
                      {studio.name.slice(0, 1)}
                    </div>
                  )}
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <h3 className="truncate text-[13px] font-medium">{studio.name}</h3>
                      {studio.detected ? <Pill tone="ok">{t.studios.detected}</Pill> : <Pill tone="mute">{t.studios.missing}</Pill>}
                      {isDefault ? <Pill>{t.studios.usingDefault}</Pill> : null}
                    </div>
                    <div className="truncate text-[11px] text-mist-300">{studio.family}</div>
                  </div>
                </button>
              </div>
              {studio.detected && (studio.cli || studio.executable) && !isDefault ? (
                <div className="mt-3">
                  <Button size="sm" variant="ghost" onClick={() => setPreferredStudio(studio.id)}>
                    {t.studios.useDefault}
                  </Button>
                </div>
              ) : null}
            </Card>
          );
        })}
      </div>
    </PageShell>
  );
}
