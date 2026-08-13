import { useMemo, useState } from "react";
import { Button, Menu } from "../components/ui";
import type { GitFile, GitSnapshot } from "../types";
import type { Dictionary } from "../i18n";
import { cn } from "../lib/cn";
import { ChevronRight } from "lucide-react";

export function GitStudio({
  git,
  t,
  commitMsg,
  setCommitMsg,
  commitFiles,
  setCommitFiles,
  busy,
  onCommit,
  licenses,
  license,
  setLicense,
  onInit,
}: {
  git: GitSnapshot | null;
  t: Dictionary;
  commitMsg: string;
  setCommitMsg: (v: string) => void;
  commitFiles: string[];
  setCommitFiles: (v: string[]) => void;
  busy: boolean;
  onCommit: () => void;
  licenses: Array<{ id: string; name: string }>;
  license: string;
  setLicense: (v: string) => void;
  onInit: () => void;
}) {
  const w = t.workspace;
  const p = t.projects;
  if (!git?.repo) {
    return (
      <div className="studio-stage-enter flex h-full items-start p-6">
        <div className="max-w-sm">
          <h2 className="text-[16px] font-semibold">{w.initGit}</h2>
          <p className="mt-2 text-[13px] leading-5 text-mist-300">{w.initGitHint}</p>
          <p className="mt-3 text-[12px] text-mist-300">{w.gitignore}</p>
          <div className="mt-4">
            <div className="mb-1.5 text-[11px] text-mist-300">{w.license}</div>
            <Menu
              value={license}
              onChange={setLicense}
              options={licenses.map((l) => ({ value: l.id, label: l.name }))}
            />
          </div>
          <div className="mt-4">
            <Button disabled={busy} onClick={onInit}>
              {w.initGit}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  const files = git.files;
  const allOn = files.length > 0 && files.every((f) => commitFiles.includes(f.path));
  const staged = files.filter((f) => commitFiles.includes(f.path));

  return (
    <div className="studio-stage-enter flex h-full min-h-0 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-5">
        <div className="flex items-baseline gap-3">
          {git.branch ? <span className="font-mono text-[15px] text-elixir-300">{git.branch}</span> : null}
          <span className="text-[12px] text-mist-300">
            {commitFiles.length}/{files.length} {w.gitStaged}
          </span>
        </div>
        {!git.identityOk ? (
          <p className="mt-3 text-[12px] text-otp-400">{git.identityHint ?? p.identityMissing}</p>
        ) : null}
        {files.length === 0 ? (
          <p className="mt-8 text-[13px] text-mist-300">{w.gitClean}</p>
        ) : (
          <>
            <p className="mt-3 max-w-lg text-[13px] leading-5 text-mist-300">{w.gitHint}</p>
            <div className="mt-4">
              <Button size="sm" variant="ghost" onClick={() => setCommitFiles(allOn ? [] : files.map((f) => f.path))}>
                {allOn ? w.selectNone : w.selectAll}
              </Button>
            </div>
            {staged.length ? (
              <div className="mt-5 grid gap-0.5">
                {staged.slice(0, 12).map((file) => {
                  const mark = gitMark(file.status);
                  return (
                    <div key={file.path} className="flex min-w-0 items-center gap-2 py-0.5 font-mono text-[11px]">
                      <span className={cn("w-10 shrink-0 uppercase", mark.cls)}>{mark.label}</span>
                      <span className="min-w-0 truncate text-mist-100">{file.path}</span>
                      {file.added || file.deleted ? (
                        <span className="ml-auto shrink-0 text-mist-300">
                          +{file.added}/−{file.deleted}
                        </span>
                      ) : null}
                    </div>
                  );
                })}
                {staged.length > 12 ? (
                  <div className="pt-1 text-[11px] text-mist-300">+{staged.length - 12}</div>
                ) : null}
              </div>
            ) : (
              <p className="mt-6 text-[13px] text-mist-300">{w.gitEmpty}</p>
            )}
            {git.depChanges.length ? (
              <div className="mt-6">
                <div className="mb-1.5 text-[10px] uppercase tracking-wider text-mist-300">mix.lock</div>
                {git.depChanges.map((d) => (
                  <div key={`${d.name}-${d.kind}`} className="font-mono text-[11px] text-mist-300">
                    {d.name} {d.from ?? "—"} → {d.to ?? d.kind}
                  </div>
                ))}
              </div>
            ) : null}
          </>
        )}
      </div>
      <div className="flex shrink-0 gap-2 border-t border-white/8 px-6 py-4">
        <input
          className="field flex-1"
          value={commitMsg}
          onChange={(e) => setCommitMsg(e.target.value)}
          placeholder={p.commitMsg}
        />
        <Button disabled={busy || !git.identityOk || !commitFiles.length || !commitMsg.trim()} onClick={onCommit}>
          {p.commit}
        </Button>
      </div>
    </div>
  );
}

type GitBranch = { name: string; path: string; file?: GitFile; kids: GitBranch[] };

export function GitRail({
  files,
  commitFiles,
  setCommitFiles,
  selectAll,
  selectNone,
}: {
  files: GitFile[];
  commitFiles: string[];
  setCommitFiles: (v: string[]) => void;
  selectAll: string;
  selectNone: string;
}) {
  const tree = useMemo(() => buildGitTree(files), [files]);
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const allOn = files.length > 0 && files.every((f) => commitFiles.includes(f.path));
  const collapseDeep = files.length > 36;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center justify-between px-3 py-2">
        <span className="text-[11px] text-mist-300">{files.length}</span>
        <button
          type="button"
          className="text-[11px] text-elixir-300 hover:text-white"
          onClick={() => setCommitFiles(allOn ? [] : files.map((f) => f.path))}
        >
          {allOn ? selectNone : selectAll}
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-1 pb-2">
        {tree.map((b) => (
          <GitRow
            key={b.path}
            branch={b}
            depth={0}
            commitFiles={commitFiles}
            setCommitFiles={setCommitFiles}
            open={open}
            setOpen={setOpen}
            collapseDeep={collapseDeep}
          />
        ))}
      </div>
    </div>
  );
}

function GitRow({
  branch,
  depth,
  commitFiles,
  setCommitFiles,
  open,
  setOpen,
  collapseDeep,
}: {
  branch: GitBranch;
  depth: number;
  commitFiles: string[];
  setCommitFiles: (v: string[]) => void;
  open: Record<string, boolean>;
  setOpen: (v: Record<string, boolean> | ((p: Record<string, boolean>) => Record<string, boolean>)) => void;
  collapseDeep: boolean;
}) {
  const hasKids = branch.kids.length > 0;
  const expanded = open[branch.path] ?? (!collapseDeep || depth === 0);
  const file = branch.file;
  const checked = file ? commitFiles.includes(file.path) : false;
  const mark = file ? gitMark(file.status) : null;

  return (
    <div>
      <button
        type="button"
        title={branch.path}
        onClick={() => {
          if (file) {
            setCommitFiles(
              commitFiles.includes(file.path)
                ? commitFiles.filter((p) => p !== file.path)
                : [...commitFiles, file.path],
            );
          }
          if (hasKids) setOpen((p) => ({ ...p, [branch.path]: !expanded }));
        }}
        className={cn(
          "flex w-full min-w-0 items-center gap-1 rounded-md py-0.5 pr-2 text-left hover:bg-white/5",
          checked && "bg-white/6",
        )}
        style={{ paddingLeft: 8 + depth * 12 }}
      >
        {hasKids ? (
          <ChevronRight size={11} className={cn("shrink-0 text-mist-300 transition", expanded && "rotate-90")} />
        ) : (
          <span className="w-[11px] shrink-0" />
        )}
        {mark ? (
          <span className={cn("w-9 shrink-0 font-mono text-[9px] uppercase", mark.cls)}>{mark.label}</span>
        ) : null}
        <span className="min-w-0 truncate font-mono text-[11px]">{branch.name}</span>
      </button>
      {hasKids && expanded
        ? branch.kids.map((k) => (
            <GitRow
              key={k.path}
              branch={k}
              depth={depth + 1}
              commitFiles={commitFiles}
              setCommitFiles={setCommitFiles}
              open={open}
              setOpen={setOpen}
              collapseDeep={collapseDeep}
            />
          ))
        : null}
    </div>
  );
}

function buildGitTree(files: GitFile[]): GitBranch[] {
  const root: GitBranch[] = [];
  for (const file of [...files].sort((a, b) => a.path.localeCompare(b.path))) {
    const parts = file.path.replace(/\\/g, "/").split("/").filter(Boolean);
    let level = root;
    let acc = "";
    for (let i = 0; i < parts.length; i++) {
      acc = acc ? `${acc}/${parts[i]}` : parts[i];
      let child = level.find((c) => c.name === parts[i]);
      if (!child) {
        child = { name: parts[i], path: acc, kids: [] };
        level.push(child);
      }
      if (i === parts.length - 1) child.file = file;
      level = child.kids;
    }
  }
  return root;
}

function gitMark(status: string): { label: string; cls: string } {
  const s = status.replace(/ /g, "");
  if (s.includes("?")) return { label: "new", cls: "text-mist-300" };
  if (s.includes("D")) return { label: "deleted", cls: "text-otp-400" };
  if (s.includes("A")) return { label: "added", cls: "text-ok-400" };
  if (s.includes("R")) return { label: "renamed", cls: "text-elixir-300" };
  if (s.includes("U")) return { label: "conflict", cls: "text-otp-400" };
  return { label: "edited", cls: "text-warn-400" };
}
