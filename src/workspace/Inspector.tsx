import { useEffect, useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import { Button, Chip, Input, Pill } from "../components/ui";
import type { ElinComment, GraphNode, ModuleGraph } from "../types";
import type { Dictionary } from "../i18n";
import { appFolder, appLabel, moduleTail, samePath, shortPath } from "./paths";
import { cn } from "../lib/cn";

const TAGS = ["note", "todo", "warn", "ignore", "boundary"] as const;
const STORY = new Set(["note", "todo", "warn"]);

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
  const [filter, setFilter] = useState("");
  const [kind, setKind] = useState<"story" | "all" | (typeof TAGS)[number]>("story");
  const comments = graph?.comments ?? [];
  const peers = useMemo(
    () => [...new Set(comments.map((c) => appFolder(c.file)).filter((v): v is string => Boolean(v)))],
    [comments],
  );
  const mine = comments.filter((c) => onNode(c, node));
  const project = useMemo(
    () => comments.filter((c) => !onNode(c, node) && matchKind(c, kind) && matchQuery(c, filter)),
    [comments, node, kind, filter],
  );
  const groups = useMemo(() => groupByApp(project), [project]);
  const currentApp = node?.path ? appLabel(node.path, peers) : "";
  const w = t.workspace;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 border-b border-white/6 px-3 py-3">
        {node ? (
          <div className="mb-3">
            <div className="flex items-baseline gap-2">
              <h2 className="min-w-0 truncate text-[15px] font-semibold tracking-tight" title={node.id}>
                {moduleTail(node.id)}
              </h2>
              <span className="shrink-0 font-mono text-[10px] text-elixir-300">{node.role || node.kind}</span>
            </div>
            {node.path ? (
              <button
                type="button"
                title={node.path}
                className="mt-1 block max-w-full truncate text-left font-mono text-[11px] text-mist-300 hover:text-mist-50"
                onClick={() => void navigator.clipboard.writeText(node.path ?? "").catch(() => undefined)}
              >
                {shortPath(node.path)}
                {appLabel(node.path, peers) ? ` · ${appLabel(node.path, peers)}` : ""}
              </button>
            ) : null}
            <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 font-mono text-[10px] text-mist-300">
              <span>{node.loc ?? 0} {w.loc}</span>
              <span>{node.defs ?? 0} def</span>
              <span>{node.defps ?? 0} defp</span>
              <span>{node.fanIn ?? 0} {w.fanInShort}</span>
              <span>{node.fanOut ?? 0} {w.fanOutShort}</span>
            </div>
            {node.wired ? null : <p className="mt-2 text-[11px] text-otp-400">{t.projects.notWired}</p>}
          </div>
        ) : (
          <p className="mb-3 text-[13px] text-mist-300">{w.pickModule}</p>
        )}

        <div className="mb-1.5 text-[10px] uppercase tracking-wider text-mist-300">{w.addNote}</div>
        <div className="mb-2 flex gap-1 overflow-x-auto">
          {TAGS.map((id) => (
            <Chip key={id} size="sm" active={tag === id} onClick={() => setTag(id)}>
              {id}
            </Chip>
          ))}
        </div>
        {tag !== "ignore" ? (
          <Input
            size="sm"
            className="mb-2"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder={w.notePlaceholder}
            disabled={!node?.path}
            onKeyDown={(e) => {
              if (e.key === "Enter" && node?.path && value.trim()) {
                void onSave(node.path, tag, value).then(() => setValue(""));
              }
            }}
          />
        ) : (
          <p className="mb-2 text-[11px] text-mist-300">{w.ignoreHint}</p>
        )}
        <Button
          size="sm"
          disabled={busy || !node?.path || (tag !== "ignore" && !value.trim())}
          onClick={() => {
            if (!node?.path) return;
            void onSave(node.path, tag, value).then(() => setValue(""));
          }}
        >
          {w.saveNote}
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        {mine.length ? (
          <section className="mb-5">
            <div className="mb-1.5 text-[10px] uppercase tracking-wider text-mist-300">{w.notesOnModule}</div>
            <div className="grid gap-1">
              {mine.map((c, i) => (
                <NoteCard key={`${c.file}-${c.line}-${i}`} comment={c} onOpen={onOpenComment} local />
              ))}
            </div>
          </section>
        ) : node ? (
          <p className="mb-5 text-[11px] text-mist-300">{w.notesEmpty}</p>
        ) : null}

        <section>
          <div className="mb-1.5 flex items-baseline justify-between gap-2">
            <div className="text-[10px] uppercase tracking-wider text-mist-300">{w.notesProject}</div>
            <span className="font-mono text-[10px] text-mist-300">{project.length}</span>
          </div>
          <Input
            size="sm"
            className="mb-2"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={w.notesFilter}
          />
          <div className="mb-3 flex gap-1 overflow-x-auto">
            {(["story", "all", "note", "todo", "warn"] as const).map((id) => (
              <Chip key={id} size="sm" active={kind === id} onClick={() => setKind(id)}>
                {id === "story" ? w.notesStory : id === "all" ? w.notesAllTags : id}
              </Chip>
            ))}
          </div>
          {groups.length === 0 ? (
            <p className="text-[12px] text-mist-300">{w.notesNone}</p>
          ) : (
            <div className="grid gap-3">
              {groups.map((group) => (
                <NoteGroup
                  key={group.app}
                  group={group}
                  openDefault={currentApp === group.app || groups.length <= 3}
                  onOpen={onOpenComment}
                />
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

function NoteGroup({
  group,
  openDefault,
  onOpen,
}: {
  group: { app: string; folder: string; files: Array<{ file: string; items: ElinComment[] }> };
  openDefault: boolean;
  onOpen: (c: ElinComment) => void;
}) {
  const [open, setOpen] = useState(openDefault);
  useEffect(() => {
    if (openDefault) setOpen(true);
  }, [openDefault]);
  const count = group.files.reduce((n, f) => n + f.items.length, 0);
  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-1 rounded-md py-0.5 text-left hover:bg-white/5"
      >
        <ChevronRight size={11} className={cn("shrink-0 text-mist-300 transition", open && "rotate-90")} />
        <span className="text-[11px] font-medium uppercase tracking-wide text-mist-100">{group.app}</span>
        <span className="ml-auto font-mono text-[10px] text-mist-300">{count}</span>
      </button>
      {open
        ? group.files.map((file) => (
            <div key={file.file} className="mt-1.5">
              <div className="truncate px-1.5 font-mono text-[10px] text-elixir-300" title={file.file}>
                {shortPath(file.file)}
              </div>
              {file.items.map((c, i) => (
                <NoteCard key={`${c.line}-${i}`} comment={c} onOpen={onOpen} grouped />
              ))}
            </div>
          ))
        : null}
    </div>
  );
}

function NoteCard({
  comment,
  onOpen,
  local,
  grouped,
}: {
  comment: ElinComment;
  onOpen: (c: ElinComment) => void;
  local?: boolean;
  grouped?: boolean;
}) {
  const label = comment.module ? moduleTail(comment.module) : shortPath(comment.file);
  return (
    <button
      type="button"
      title={`${comment.file}:${comment.line}`}
      onClick={() => onOpen(comment)}
      className="w-full rounded-md px-1.5 py-1.5 text-left hover:bg-white/5"
    >
      <div className="flex min-w-0 items-center gap-1.5">
        <Pill tone={tagTone(comment.tag)}>{comment.tag}</Pill>
        {local || grouped ? null : (
          <span className="min-w-0 truncate font-mono text-[10px] text-elixir-300">{label}</span>
        )}
        <span className="ml-auto shrink-0 font-mono text-[10px] text-mist-300">:{comment.line}</span>
      </div>
      {comment.value ? (
        <p className="mt-1 line-clamp-3 text-[12px] leading-4 text-mist-100">{comment.value}</p>
      ) : null}
    </button>
  );
}

function groupByApp(items: ElinComment[]) {
  const peers = [...new Set(items.map((i) => appFolder(i.file)).filter((v): v is string => Boolean(v)))];
  const apps = new Map<string, { folder: string; files: Map<string, ElinComment[]> }>();
  for (const item of items) {
    const folder = appFolder(item.file) ?? "";
    const app = appLabel(item.file, peers);
    const bucket = apps.get(app) ?? { folder, files: new Map<string, ElinComment[]>() };
    const key = posixKey(item.file);
    const list = bucket.files.get(key) ?? [];
    list.push(item);
    bucket.files.set(key, list);
    apps.set(app, bucket);
  }
  return [...apps.entries()]
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([app, bucket]) => ({
      app,
      folder: bucket.folder,
      files: [...bucket.files.entries()]
        .map(([file, comments]) => ({ file, items: comments }))
        .sort((a, b) => shortPath(a.file).localeCompare(shortPath(b.file))),
    }));
}

function matchKind(comment: ElinComment, kind: "story" | "all" | string) {
  if (kind === "all") return true;
  if (kind === "story") return STORY.has(comment.tag);
  return comment.tag === kind;
}

function matchQuery(comment: ElinComment, query: string) {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    comment.value.toLowerCase().includes(q) ||
    comment.file.toLowerCase().includes(q) ||
    (comment.module ?? "").toLowerCase().includes(q) ||
    comment.tag.toLowerCase().includes(q)
  );
}

function tagTone(tag: string): "violet" | "rose" | "ok" | "mute" | "warn" {
  if (tag === "warn") return "warn";
  if (tag === "todo") return "rose";
  if (tag === "note") return "violet";
  if (tag === "ignore") return "mute";
  return "ok";
}

function onNode(comment: ElinComment, node: GraphNode | null) {
  if (!node) return false;
  if (node.path && samePath(comment.file, node.path)) return true;
  return Boolean(comment.module && comment.module === node.id);
}

function posixKey(path: string) {
  return path.replace(/\\/g, "/").toLowerCase();
}
