import { useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import { Chip, Input } from "../components/ui";
import { cn } from "../lib/cn";
import type { ElinComment, GraphNode, ModuleGraph } from "../types";
import type { Dictionary } from "../i18n";
import { ContextMenu, type MenuItem } from "./ContextMenu";

type Branch = {
  name: string;
  full: string;
  node?: GraphNode;
  kids: Branch[];
  count: number;
};

export function ModuleTree({
  graph,
  selectedId,
  query,
  onQuery,
  onSelect,
  onOpen,
  onCopy,
  t,
}: {
  graph: ModuleGraph | null;
  selectedId?: string | null;
  query: string;
  onQuery: (v: string) => void;
  onSelect: (node: GraphNode) => void;
  onOpen?: (node: GraphNode) => void;
  onCopy?: (node: GraphNode) => void;
  t: Dictionary;
}) {
  const tests = (graph?.nodes ?? []).filter((n) => n.kind === "test").length;
  const [kind, setKind] = useState<"lib" | "test" | "all">(tests > 8 ? "lib" : "all");
  const noted = useMemo(() => notedFiles(graph?.comments ?? []), [graph?.comments]);
  const tree = useMemo(
    () => build(graph?.nodes ?? [], query, kind),
    [graph, query, kind],
  );
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const [menu, setMenu] = useState<{ x: number; y: number; node: GraphNode } | null>(null);
  const stats = graph?.stats;
  const shown = kind === "all" ? (stats?.modules ?? 0) : tree.reduce((n, b) => n + b.count, 0);

  function itemsFor(node: GraphNode): MenuItem[] {
    return [
      { kind: "item", label: t.workspace.openEditor, onClick: () => onOpen?.(node), muted: !onOpen },
      { kind: "item", label: t.workspace.copyModule, onClick: () => onCopy?.(node) },
      { kind: "item", label: t.workspace.copyPath, onClick: () => void navigator.clipboard.writeText(node.path ?? node.id).catch(() => undefined) },
      { kind: "sep" },
      { kind: "item", label: t.workspace.focusGraph, onClick: () => onSelect(node) },
    ];
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {stats ? (
        <div className="flex shrink-0 flex-wrap gap-x-3 gap-y-1 border-b border-white/6 px-3 py-2 font-mono text-[10px] text-mist-300">
          <span>{shown}/{stats.modules}</span>
          <span>{stats.unwired} {t.workspace.unwired.toLowerCase()}</span>
          <span>{stats.cycles} {t.workspace.cycles.toLowerCase()}</span>
        </div>
      ) : null}
      <div className="shrink-0 space-y-2 px-2 py-2">
        <Input
          size="sm"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          placeholder={t.workspace.modulesFilter}
        />
        {tests > 0 ? (
          <div className="flex flex-wrap gap-1">
            {(["lib", "test", "all"] as const).map((id) => (
              <Chip key={id} size="sm" active={kind === id} onClick={() => setKind(id)}>
                {id === "lib" ? t.workspace.libOnly : id === "test" ? t.workspace.testsOnly : t.workspace.modulesAll}
              </Chip>
            ))}
          </div>
        ) : null}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-1 pb-2">
        {tree.map((b) => (
          <Row
            key={b.full}
            branch={b}
            depth={0}
            selectedId={selectedId}
            open={open}
            setOpen={setOpen}
            forceOpen={Boolean(query.trim())}
            noted={noted}
            onSelect={onSelect}
            onOpen={onOpen}
            onMenu={setMenu}
          />
        ))}
      </div>
      {menu ? (
        <ContextMenu x={menu.x} y={menu.y} items={itemsFor(menu.node)} onClose={() => setMenu(null)} />
      ) : null}
    </div>
  );
}

function Row({
  branch,
  depth,
  selectedId,
  open,
  setOpen,
  forceOpen,
  noted,
  onSelect,
  onOpen,
  onMenu,
}: {
  branch: Branch;
  depth: number;
  selectedId?: string | null;
  open: Record<string, boolean>;
  setOpen: (v: Record<string, boolean> | ((p: Record<string, boolean>) => Record<string, boolean>)) => void;
  forceOpen: boolean;
  noted: Set<string>;
  onSelect: (node: GraphNode) => void;
  onOpen?: (node: GraphNode) => void;
  onMenu: (v: { x: number; y: number; node: GraphNode } | null) => void;
}) {
  const hasKids = branch.kids.length > 0;
  const onPath = Boolean(
    selectedId &&
      (selectedId === branch.full ||
        selectedId.startsWith(`${branch.full}.`) ||
        (branch.full.startsWith("b:") && selectedId.toLowerCase().includes(`.${branch.name}.`))),
  );
  const user = open[branch.full];
  const expanded = forceOpen || user === true || (user !== false && (depth === 0 || onPath));
  const active = branch.node && selectedId === branch.node.id;
  const hasNote = Boolean(branch.node?.path && noted.has(norm(branch.node.path)));
  return (
    <div>
      <button
        type="button"
        title={branch.node?.id ?? branch.full}
        onClick={() => {
          if (branch.node) onSelect(branch.node);
          else setOpen((p) => ({ ...p, [branch.full]: !expanded }));
        }}
        onDoubleClick={() => {
          if (branch.node) onOpen?.(branch.node);
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          if (branch.node) onMenu({ x: e.clientX, y: e.clientY, node: branch.node });
        }}
        className={cn(
          "flex w-full min-w-0 items-center gap-1 rounded-md py-0.5 pr-2 text-left text-[12px] hover:bg-white/5",
          active && "bg-elixir-600/20 text-mist-50",
        )}
        style={{ paddingLeft: 8 + depth * 11 }}
      >
        {hasKids ? (
          <span
            className="flex shrink-0"
            onClick={(e) => {
              e.stopPropagation();
              setOpen((p) => ({ ...p, [branch.full]: !expanded }));
            }}
          >
            <ChevronRight size={11} className={cn("text-mist-300 transition", expanded && "rotate-90")} />
          </span>
        ) : (
          <span className="w-[11px] shrink-0" />
        )}
        <span className="min-w-0 truncate">{branch.name}</span>
        <span className="ml-auto flex shrink-0 items-center gap-1.5">
          {hasKids && !branch.node ? (
            <span className="font-mono text-[9px] text-mist-300">{branch.count}</span>
          ) : null}
          {hasNote ? <span className="size-1.5 rounded-full bg-elixir-400" title="elin" /> : null}
          {branch.node?.git && branch.node.git !== "unchanged" ? (
            <span className="font-mono text-[9px] uppercase text-ok-400">{branch.node.git[0]}</span>
          ) : null}
        </span>
      </button>
      {hasKids && expanded
        ? branch.kids.map((k) => (
            <Row
              key={k.full}
              branch={k}
              depth={depth + 1}
              selectedId={selectedId}
              open={open}
              setOpen={setOpen}
              forceOpen={forceOpen}
              noted={noted}
              onSelect={onSelect}
              onOpen={onOpen}
              onMenu={onMenu}
            />
          ))
        : null}
    </div>
  );
}

function build(nodes: GraphNode[], query: string, kind: "lib" | "test" | "all"): Branch[] {
  const q = query.trim().toLowerCase();
  const filtered = nodes.filter((n) => {
    if (kind === "lib" && n.kind === "test") return false;
    if (kind === "test" && n.kind !== "test") return false;
    if (!q) return true;
    return n.id.toLowerCase().includes(q) || (n.path ?? "").toLowerCase().includes(q);
  });
  const groupApps = new Set(filtered.map((n) => n.boundary).filter(Boolean)).size >= 2;
  const root: Branch[] = [];
  for (const node of [...filtered].sort((a, b) => a.id.localeCompare(b.id))) {
    const segs = node.id.split(".");
    const parts = groupApps && node.boundary ? [node.boundary, ...segs.slice(1)] : segs;
    let level = root;
    for (let i = 0; i < parts.length; i++) {
      const name = parts[i];
      const leaf = i === parts.length - 1;
      const full = leaf ? node.id : groupApps && i === 0 ? `b:${name}` : segs.slice(0, i + 1).join(".");
      let child = level.find((c) => c.full === full && c.name === name);
      if (!child) {
        child = { name, full, kids: [], count: 0 };
        level.push(child);
      }
      child.count += 1;
      if (leaf) child.node = node;
      level = child.kids;
    }
  }
  return root;
}

function notedFiles(comments: ElinComment[]) {
  const set = new Set<string>();
  for (const c of comments) {
    if (c.tag === "note" || c.tag === "todo" || c.tag === "warn") set.add(norm(c.file));
  }
  return set;
}

function norm(path: string) {
  return path.replace(/\\/g, "/").toLowerCase();
}
