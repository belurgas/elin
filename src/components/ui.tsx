import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "../lib/cn";

export function PageShell({
  kicker,
  title,
  subtitle,
  actions,
  children,
  fill,
}: {
  kicker?: string;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
  fill?: boolean;
}) {
  return (
    <div
      className={cn(
        "page-enter mx-auto flex w-full max-w-5xl flex-col gap-5 p-6",
        fill ? "h-full min-h-0 flex-1 overflow-hidden" : "min-h-full",
      )}
    >
      <div className="flex shrink-0 items-start justify-between gap-4">
        <div className="min-w-0">
          {kicker ? <div className="mb-1 text-[11px] text-elixir-400">{kicker}</div> : null}
          <h1 className="text-[22px] font-semibold tracking-tight text-mist-50">{title}</h1>
          {subtitle ? <p className="mt-1 max-w-xl text-[13px] leading-5 text-mist-300">{subtitle}</p> : null}
        </div>
        {actions}
      </div>
      {fill ? <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">{children}</div> : children}
    </div>
  );
}

export function Loader({ label }: { label?: string }) {
  return (
    <div className="flex items-center gap-3 px-1 py-6 text-[13px] text-mist-300">
      <span className="loader-spin size-4 shrink-0 rounded-full border-2 border-white/15 border-t-elixir-400" />
      {label ?? "Loading…"}
    </div>
  );
}

export function Card({
  children,
  className,
  onClick,
}: {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
}) {
  const classNames = cn("surface rounded-xl p-4", onClick && "pressable cursor-pointer", className);
  if (onClick) {
    return (
      <button type="button" onClick={onClick} className={cn(classNames, "w-full text-left")}>
        {children}
      </button>
    );
  }
  return <div className={classNames}>{children}</div>;
}

export function Button({
  children,
  onClick,
  variant = "primary",
  disabled,
  type = "button",
  size = "md",
  className,
  title,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "ghost" | "danger";
  disabled?: boolean;
  type?: "button" | "submit";
  size?: "md" | "sm";
  className?: string;
  title?: string;
}) {
  return (
    <button
      type={type}
      disabled={disabled}
      title={title}
      onClick={onClick}
      className={cn(
        "inline-flex cursor-pointer items-center justify-center gap-1.5 font-medium transition duration-150 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50",
        size === "md" ? "rounded-lg px-3 py-1.5 text-sm" : "rounded-md px-2 py-1 text-xs",
        variant === "primary" && "bg-elixir-600 text-white hover:bg-elixir-700",
        variant === "ghost" && "bg-white/6 text-mist-50 hover:bg-white/10",
        variant === "danger" && "bg-otp-500/15 text-otp-400 hover:bg-otp-500/25",
        className,
      )}
    >
      {children}
    </button>
  );
}

export function Chip({
  children,
  active,
  onClick,
}: {
  children: ReactNode;
  active?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition duration-150",
        active ? "bg-elixir-600 text-white" : "bg-white/6 text-mist-300 hover:bg-white/10 hover:text-white",
      )}
    >
      {children}
    </button>
  );
}

export function Pill({ children, tone = "violet" }: { children: ReactNode; tone?: "violet" | "rose" | "ok" | "mute" }) {
  return (
    <span
      className={cn(
        "inline-flex rounded-md px-1.5 py-0.5 text-[10px] font-medium",
        tone === "violet" && "bg-elixir-600/20 text-elixir-300",
        tone === "rose" && "bg-otp-500/15 text-otp-400",
        tone === "ok" && "bg-ok-400/15 text-ok-400",
        tone === "mute" && "bg-white/8 text-mist-300",
      )}
    >
      {children}
    </span>
  );
}

export function Dot({ tone }: { tone: "ok" | "warn" | "bad" | "mute" }) {
  return (
    <span
      className={cn(
        "size-1.5 shrink-0 rounded-full",
        tone === "ok" && "bg-ok-400",
        tone === "warn" && "bg-warn-400",
        tone === "bad" && "bg-otp-400",
        tone === "mute" && "bg-white/25",
      )}
    />
  );
}

export function Checkbox({
  checked,
  onChange,
  children,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  children: ReactNode;
}) {
  return (
    <label className="flex cursor-pointer select-none items-center gap-2 text-sm text-mist-100">
      <span
        className={cn(
          "relative flex size-4 shrink-0 items-center justify-center rounded border transition duration-150",
          checked ? "border-elixir-500 bg-elixir-600" : "border-white/18 bg-white/5 hover:border-elixir-400/50",
        )}
      >
        <input
          type="checkbox"
          className="peer sr-only"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
        />
        <svg
          viewBox="0 0 12 12"
          className={cn("size-2.5 text-white transition", checked ? "opacity-100" : "opacity-0")}
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M2 6.2 4.8 9 10 3" />
        </svg>
      </span>
      {children}
    </label>
  );
}

export function Menu({
  value,
  options,
  onChange,
  placeholder,
  className,
}: {
  value: string;
  options: Array<{ value: string; label: string; hint?: string }>;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const current = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    const onDoc = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  return (
    <div ref={root} className={cn("relative min-w-[200px]", className)}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="field flex w-full items-center justify-between gap-3 rounded-lg text-left text-sm"
      >
        <span className={current ? "truncate text-mist-50" : "truncate text-mist-300"}>
          {current?.label ?? placeholder ?? ""}
        </span>
        <span className={cn("size-3 shrink-0 text-mist-300 transition", open && "rotate-180")}>
          <svg viewBox="0 0 12 12" className="size-3" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M2.5 4.5 6 8l3.5-3.5" />
          </svg>
        </span>
      </button>
      {open ? (
        <div className="surface absolute z-30 mt-1.5 max-h-64 w-full overflow-y-auto rounded-lg p-1">
          {options.map((option) => (
            <button
              type="button"
              key={option.value}
              onClick={() => {
                onChange(option.value);
                setOpen(false);
              }}
              className={cn(
                "flex w-full cursor-pointer items-center justify-between gap-3 rounded-md px-2.5 py-1.5 text-left text-sm",
                option.value === value ? "bg-elixir-600/25 text-white" : "text-mist-100 hover:bg-white/6",
              )}
            >
              <span className="truncate">{option.label}</span>
              {option.hint ? <span className="shrink-0 font-mono text-[11px] text-mist-300">{option.hint}</span> : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
