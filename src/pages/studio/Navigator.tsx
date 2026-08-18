import { Button, Card, Pill, Input } from "../../components/ui";
import { cn } from "../../lib/cn";
import type { MixProject } from "../../types";
import type { Labels } from "./types";

export function Navigator({
  query,
  onQuery,
  scanning,
  empty,
  starred,
  recents,
  rest,
  selected,
  t,
  onSelect,
  onActivate,
  onAdd,
  onCreate,
  onScanQuick,
}: {
  query: string;
  onQuery: (v: string) => void;
  scanning: boolean;
  empty: boolean;
  starred: MixProject[];
  recents: MixProject[];
  rest: MixProject[];
  selected: MixProject | null;
  t: Labels;
  onSelect: (p: MixProject) => void;
  onActivate?: (p: MixProject) => void;
  onAdd: () => void;
  onCreate: () => void;
  onScanQuick: () => void;
}) {
  return (
    <div className="flex min-h-0 flex-col gap-3">
      <div className="flex gap-2">
        <Input
          className="flex-1"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          placeholder={t.search}
        />
        <Button variant="ghost" onClick={onAdd}>
          {t.addFolder}
        </Button>
      </div>
      <div className="flex gap-2">
        <Button variant="ghost" onClick={onCreate}>
          {t.newProject}
        </Button>
        <Button variant="ghost" disabled={scanning} onClick={onScanQuick}>
          {t.scanQuick}
        </Button>
      </div>
      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
        {empty ? (
          <Card className="text-sm text-mist-300">{t.empty}</Card>
        ) : (
          <>
            <NavGroup title={t.pinnedSection} items={starred} selected={selected} onSelect={onSelect} onActivate={onActivate} />
            <NavGroup title={t.recentsSection} items={recents} selected={selected} onSelect={onSelect} onActivate={onActivate} />
            <NavGroup title={t.allSection} items={rest} selected={selected} onSelect={onSelect} onActivate={onActivate} />
          </>
        )}
      </div>
    </div>
  );
}

function NavGroup({
  title,
  items,
  selected,
  onSelect,
  onActivate,
}: {
  title: string;
  items: MixProject[];
  selected: MixProject | null;
  onSelect: (p: MixProject) => void;
  onActivate?: (p: MixProject) => void;
}) {
  if (!items.length) return null;
  return (
    <div>
      <div className="mb-1.5 text-[11px] text-mist-300">{title}</div>
      <div className="surface divide-y divide-white/6 overflow-hidden rounded-lg">
        {items.map((project) => (
          <button
            type="button"
            key={project.path}
            onClick={() => onSelect(project)}
            onDoubleClick={() => onActivate?.(project)}
            className={cn(
              "w-full cursor-pointer px-3 py-2 text-left hover:bg-white/4",
              selected?.path === project.path && "bg-white/6",
            )}
          >
            <div className="flex items-center gap-2">
              <span className="truncate font-medium">{project.name}</span>
              {project.hasPhoenix ? <Pill>Phx</Pill> : null}
            </div>
            <div className="mt-0.5 truncate font-mono text-[10px] text-mist-300">{project.path}</div>
          </button>
        ))}
      </div>
    </div>
  );
}
