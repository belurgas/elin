import { useEffect, useRef, useState } from "react";
import { Check, Copy, Plus, Terminal, X } from "lucide-react";
import { cn } from "../lib/cn";

export type ConsoleLine = { id: number; text: string; kind?: "in" | "out" };

export type TermSession = {
  id: string;
  title: string;
  lines: ConsoleLine[];
  running: boolean;
  ok: boolean | null;
  task: string | null;
};

const MIN_H = 140;
const DEFAULT_H = 248;

function maxH() {
  return Math.max(MIN_H, Math.floor(window.innerHeight * 0.5));
}

export function ConsoleDock({
  sessions,
  activeId,
  onActive,
  onNew,
  onClose,
  onSubmit,
  open,
  onToggle,
  empty,
  runningLabel,
  passedLabel,
  failedLabel,
  copyLabel,
  placeholder,
}: {
  sessions: TermSession[];
  activeId: string;
  onActive: (id: string) => void;
  onNew: () => void;
  onClose: (id: string) => void;
  onSubmit: (id: string, command: string) => void;
  open: boolean;
  onToggle: () => void;
  empty: string;
  runningLabel: string;
  passedLabel: string;
  failedLabel: string;
  copyLabel: string;
  placeholder: string;
}) {
  const active = sessions.find((s) => s.id === activeId) ?? sessions[0];
  const scroller = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const stick = useRef(true);
  const history = useRef<string[]>([]);
  const histIdx = useRef(-1);
  const [copied, setCopied] = useState(false);
  const [draft, setDraft] = useState("");
  const [height, setHeight] = useState(() => {
    const n = Number(localStorage.getItem("elin.consoleH"));
    return Number.isFinite(n) && n >= MIN_H ? n : DEFAULT_H;
  });

  useEffect(() => {
    const el = scroller.current;
    if (!el || !stick.current) return;
    el.scrollTop = el.scrollHeight;
  }, [active?.lines, open, activeId]);

  if (!active) return null;
  const status = active.running
    ? runningLabel
    : active.ok === true
      ? passedLabel
      : active.ok === false
        ? failedLabel
        : empty;

  return (
    <section className={cn("studio-console", open && "is-open")}>
      <div
        className="studio-console-grip"
        title="Resize"
        onMouseDown={(e) => {
          e.preventDefault();
          document.body.style.cursor = "ns-resize";
          document.body.style.userSelect = "none";
          const onMove = (ev: MouseEvent) => {
            const next = Math.min(maxH(), Math.max(MIN_H, window.innerHeight - ev.clientY - 8));
            setHeight(next);
            localStorage.setItem("elin.consoleH", String(next));
          };
          const onUp = () => {
            window.removeEventListener("mousemove", onMove);
            window.removeEventListener("mouseup", onUp);
            document.body.style.cursor = "";
            document.body.style.userSelect = "";
          };
          window.addEventListener("mousemove", onMove);
          window.addEventListener("mouseup", onUp);
        }}
      />
      <div className="studio-console-tabs">
        {sessions.map((s) => (
          <button
            key={s.id}
            type="button"
            onClick={() => {
              onActive(s.id);
              if (!open) onToggle();
            }}
            className={cn("studio-console-tab", s.id === active.id && "is-active")}
            title={s.task ?? s.title}
          >
            <span className={cn("studio-console-dot", s.running && "is-run", s.ok === true && "is-ok", s.ok === false && "is-err")} />
            <span className="max-w-[160px] truncate">{s.title}</span>
            {sessions.length > 1 ? (
              <span
                className="studio-console-x"
                onClick={(e) => {
                  e.stopPropagation();
                  onClose(s.id);
                }}
              >
                <X size={10} />
              </span>
            ) : null}
          </button>
        ))}
        <button type="button" className="studio-console-tab is-add" onClick={onNew} title="+">
          <Plus size={12} />
        </button>
        <button type="button" className="ml-auto px-2 text-mist-300 hover:text-white" onClick={onToggle}>
          {open ? "—" : "+"}
        </button>
      </div>
      {open ? (
        <div className="studio-console-body" style={{ height }}>
          <div className="studio-console-tools">
            <span className="mr-2 font-mono text-[10px] text-mist-300">{status}</span>
            <button
              type="button"
              className="studio-console-icon"
              title={copyLabel}
              onClick={() => {
                void navigator.clipboard.writeText(active.lines.map((l) => l.text).join("\n"));
                setCopied(true);
                window.setTimeout(() => setCopied(false), 1200);
              }}
            >
              {copied ? <Check size={12} /> : <Copy size={12} />}
            </button>
          </div>
          <div
            ref={scroller}
            className="studio-console-scroll"
            onScroll={() => {
              const el = scroller.current;
              if (!el) return;
              stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 28;
            }}
          >
            {active.lines.map((line) => (
              <div key={line.id} className={cn("studio-console-line", line.kind === "in" ? "is-in" : tone(line.text))}>
                {line.kind === "in" ? (
                  <>
                    <span className="text-elixir-400">❯ </span>
                    {line.text}
                  </>
                ) : (
                  line.text || " "
                )}
              </div>
            ))}
            {active.running ? (
              <div className="studio-console-wait">
                <span className="studio-cursor" />
              </div>
            ) : null}
          </div>
          <form
            className="studio-console-input"
            onSubmit={(e) => {
              e.preventDefault();
              const cmd = draft.trim();
              if (!cmd || active.running) return;
              history.current = [...history.current.filter((c) => c !== cmd), cmd];
              histIdx.current = -1;
              onSubmit(active.id, cmd);
              setDraft("");
            }}
          >
            <Terminal size={12} className="text-elixir-400" />
            <input
              ref={input}
              value={draft}
              disabled={active.running}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "ArrowUp") {
                  e.preventDefault();
                  const h = history.current;
                  if (!h.length) return;
                  const next = histIdx.current < 0 ? h.length - 1 : Math.max(0, histIdx.current - 1);
                  histIdx.current = next;
                  setDraft(h[next] ?? "");
                } else if (e.key === "ArrowDown") {
                  e.preventDefault();
                  const h = history.current;
                  if (histIdx.current < 0) return;
                  const next = histIdx.current + 1;
                  if (next >= h.length) {
                    histIdx.current = -1;
                    setDraft("");
                  } else {
                    histIdx.current = next;
                    setDraft(h[next] ?? "");
                  }
                }
              }}
              placeholder={placeholder}
              className="min-w-0 flex-1 bg-transparent font-mono text-[12px] text-mist-50 outline-none placeholder:text-mist-300/50"
            />
          </form>
        </div>
      ) : null}
    </section>
  );
}

export function tabTitle(command: string) {
  const t = command.trim().replace(/\s+/g, " ");
  return t.length > 28 ? `${t.slice(0, 27)}…` : t || "shell";
}

function tone(line: string): string {
  if (/\berror\b|\bfailed\b|\*\* \(|не является/i.test(line)) return "is-err";
  if (/\bwarning\b|\bwarn\b/i.test(line)) return "is-warn";
  if (/^==> |Compiling |Generated |Finished /i.test(line)) return "is-meta";
  if (/0 failures|Finished in |success/i.test(line)) return "is-ok";
  return "";
}
