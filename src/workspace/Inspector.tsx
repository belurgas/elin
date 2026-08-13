import { useState } from "react";
import { Button } from "../components/ui";
import type { ElinComment, GraphNode, ModuleGraph } from "../types";
import type { Dictionary } from "../i18n";
import { cn } from "../lib/cn";

const TAGS = ["note", "todo", "warn", "ignore", "boundary"] as const;

export function Inspector({
  node,
  graph,
  t,
  busy,
  onSave,
  onOpenComment,
}: {
  node: GraphNode | null;
  graph: ModuleGraph | null;
  t: Dictionary;
  busy: boolean;
  onSave: (file: string, tag: string, value: string) => Promise<void>;
  onOpenComment: (comment: ElinComment) => void;
}) {
  const [tag, setTag] = useState<(typeof TAGS)[number]>("note");
  const [value, setValue] = useState("");
  const mine = (graph?.comments ?? []).filter((c) => sameFile(c.file, node?.path));
  const others = (graph?.comments ?? []).filter((c) => !sameFile(c.file, node?.path));

  return (
    <div className="flex h-full min-h-0 flex-col overflow-y-auto px-3 py-3">
      {node ? (
        <div className="mb-4">
          <div className="flex items-baseline gap-2">
            <h2 className="min-w-0 truncate text-[15px] font-semibold tracking-tight">{node.label}</h2>
            <span className="shrink-0 font-mono text-[10px] text-elixir-300">{node.role || node.kind}</span>
          </div>
          {node.path ? (
            <div className="mt-1 truncate font-mono text-[11px] text-mist-300" title={node.path}>
              {node.path}
            </div>
          ) : null}
          <div className="mt-3 flex gap-4 font-mono text-[11px] text-mist-300">
            <span>{node.loc ?? 0} {t.workspace.loc}</span>
            <span>
              {node.defs ?? 0}/{node.defps ?? 0} def
            </span>
            <span>
              {node.fanIn ?? 0}/{node.fanOut ?? 0} fan
            </span>
          </div>
          {node.wired ? null : <p className="mt-2 text-[11px] text-otp-400">{t.projects.notWired}</p>}
        </div>
      ) : (
        <p className="mb-4 text-[13px] text-mist-300">{t.workspace.pickModule}</p>
      )}

      <div className="mb-1 text-[10px] uppercase tracking-wider text-mist-300">{t.workspace.addNote}</div>
      <div className="mb-2 flex gap-1">
        {TAGS.map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => setTag(id)}
            className={cn(
              "rounded px-1.5 py-0.5 font-mono text-[10px]",
              tag === id ? "text-elixir-300" : "text-mist-300 hover:text-mist-50",
            )}
          >
            {id}
          </button>
        ))}
      </div>
      {tag !== "ignore" ? (
        <input
          className="field mb-2"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={t.workspace.notePlaceholder}
          disabled={!node?.path}
          onKeyDown={(e) => {
            if (e.key === "Enter" && node?.path && value.trim()) {
              void onSave(node.path, tag, value).then(() => setValue(""));
            }
          }}
        />
      ) : (
        <p className="mb-2 text-[11px] text-mist-300">{t.workspace.ignoreHint}</p>
      )}
      <Button
        size="sm"
        disabled={busy || !node?.path || (tag !== "ignore" && !value.trim())}
        onClick={() => {
          if (!node?.path) return;
          void onSave(node.path, tag, value).then(() => setValue(""));
        }}
      >
        {t.workspace.saveNote}
      </Button>

      {mine.length ? (
        <NoteList title={t.workspace.notesOnModule} items={mine} onOpen={onOpenComment} />
      ) : null}
      {others.length ? (
        <NoteList title={t.workspace.allNotes} items={others} onOpen={onOpenComment} />
      ) : null}
    </div>
  );
}

function NoteList({
  title,
  items,
  onOpen,
}: {
  title: string;
  items: ElinComment[];
  onOpen: (c: ElinComment) => void;
}) {
  return (
    <div className="mt-5">
      <div className="mb-1.5 text-[10px] uppercase tracking-wider text-mist-300">{title}</div>
      <div className="grid">
        {items.map((c, i) => (
          <button
            key={`${c.file}-${c.line}-${i}`}
            type="button"
            onClick={() => onOpen(c)}
            className="group rounded-md px-1.5 py-1.5 text-left hover:bg-white/5"
          >
            <div className="truncate font-mono text-[10px] text-elixir-300 group-hover:underline">
              {c.file}:{c.line} · {c.tag}
            </div>
            {c.value ? <div className="mt-0.5 truncate text-[12px] text-mist-100">{c.value}</div> : null}
          </button>
        ))}
      </div>
    </div>
  );
}

function sameFile(a: string, b?: string | null) {
  if (!b) return false;
  return a.replace(/\\/g, "/").toLowerCase() === b.replace(/\\/g, "/").toLowerCase();
}
