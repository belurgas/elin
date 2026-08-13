import { useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import { cn } from "../lib/cn";
import type { GraphNode, ModuleGraph } from "../types";
import type { Dictionary } from "../i18n";
import { ContextMenu, type MenuItem } from "./ContextMenu";

type Branch = {
  name: string;
  full: string;
  node?: GraphNode;
  kids: Branch[];
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
  const tree = useMemo(() => build(graph?.nodes ?? [], query), [graph, query]);
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const [menu, setMenu] = useState<{ x: number; y: number; node: GraphNode } | null>(null);
  const stats = graph?.stats;

  function itemsFor(node: GraphNode): MenuItem[] {
    return [
      { kind: "item", label: t.workspace.openEditor, onClick: () => onOpen?.(node), muted: !onOpen },
      { kind: "item", label: t.workspace.copyModule, onClick: () => onCopy?.(node) },
      { kind: "item", label: t.workspace.copyPath, onClick: () => void navigator.clipboard.writeText(node.path ?? node.id) },
      { kind: "sep" },
      { kind: "item", label: t.workspace.focusGraph, onClick: () => onSelect(node) },
    ];
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      {stats ? (
        <div className="flex shrink-0 gap-3 border-b border-white/6 px-3 py-2 font-mono text-[10px] text-mist-300">
          <span>{stats.modules}</span>
          <span>{stats.unwired} {t.workspace.unwired.toLowerCase()}</span>
          <span>{stats.cycles} {t.workspace.cycles.toLowerCase()}</span>
        </div>
      ) : null}
      <div className="shrink-0 px-2 py-2">
        <input
          className="field"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          placeholder={t.workspace.modules}
        />
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
  onSelect: (node: GraphNode) => void;
  onOpen?: (node: GraphNode) => void;
  onMenu: (v: { x: number; y: number; node: GraphNode } | null) => void;
}) {
  const hasKids = branch.kids.length > 0;
  const onPath = Boolean(selectedId && (selectedId === branch.full || selectedId.startsWith(`${branch.full}.`)));
  const user = open[branch.full];
  const expanded = forceOpen || user === true || (user !== false && (depth === 0 || onPath));
  const active = branch.node && selectedId === branch.node.id;
  return (
    <div>
      <button
        type="button"
        title={branch.full}
        onClick={() => {
          if (branch.node) onSelect(branch.node);
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
        {branch.node?.git && branch.node.git !== "unchanged" ? (
          <span className="ml-auto shrink-0 font-mono text-[9px] text-ok-400">{branch.node.git[0]}</span>
        ) : null}
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
              onSelect={onSelect}
              onOpen={onOpen}
              onMenu={onMenu}
            />
          ))
        : null}
    </div>
  );
}

function build(nodes: GraphNode[], query: string): Branch[] {
  const q = query.trim().toLowerCase();
  const filtered = q
    ? nodes.filter((n) => n.id.toLowerCase().includes(q) || (n.path ?? "").toLowerCase().includes(q))
    : nodes;
  const root: Branch[] = [];
  for (const node of [...filtered].sort((a, b) => a.id.localeCompare(b.id))) {
    const parts = node.id.split(".");
    let level = root;
    let acc = "";
    for (let i = 0; i < parts.length; i++) {
      acc = acc ? `${acc}.${parts[i]}` : parts[i];
      let child = level.find((c) => c.name === parts[i]);
      if (!child) {
        child = { name: parts[i], full: acc, kids: [] };
        level.push(child);
      }
      if (i === parts.length - 1) child.node = node;
      level = child.kids;
    }
  }
  return root;
}
