import { useEffect, useId, useRef, useState, type InputHTMLAttributes, type ReactNode, type TextareaHTMLAttributes } from "react";
import { cn } from "../lib/cn";

/** Shared Elin controls. Screens import from here instead of native inputs. */

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

export function Field({
  label,
  children,
  className,
}: {
  label?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={cn("grid gap-2 text-sm text-mist-100", className)}>
      {label}
      {children}
    </label>
  );
}

export function Input({
  className,
  size = "md",
  ...props
}: Omit<InputHTMLAttributes<HTMLInputElement>, "size"> & { size?: "md" | "sm" }) {
  return (
    <input
      {...props}
      className={cn("field", size === "sm" && "px-2 py-1 text-xs", className)}
    />
  );
}

export function Textarea({
  className,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea {...props} className={cn("field field-area selectable", className)} />;
}

export function Chip({
  children,
  active,
  onClick,
  size = "md",
}: {
  children: ReactNode;
  active?: boolean;
  onClick?: () => void;
  size?: "md" | "sm";
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "cursor-pointer font-medium transition duration-150",
        size === "md" ? "rounded-md px-2.5 py-1 text-xs" : "rounded px-1.5 py-0.5 text-[10px]",
        active ? "bg-elixir-600 text-white" : "bg-white/6 text-mist-300 hover:bg-white/10 hover:text-white",
      )}
    >
      {children}
    </button>
  );
}

export function Pill({
  children,
  tone = "violet",
}: {
  children: ReactNode;
  tone?: "violet" | "rose" | "ok" | "mute" | "warn";
}) {
  return (
    <span
      className={cn(
        "inline-flex rounded-md px-1.5 py-0.5 text-[10px] font-medium",
        tone === "violet" && "bg-elixir-600/20 text-elixir-300",
        tone === "rose" && "bg-otp-500/15 text-otp-400",
        tone === "ok" && "bg-ok-400/15 text-ok-400",
        tone === "mute" && "bg-white/8 text-mist-300",
        tone === "warn" && "bg-warn-400/15 text-warn-400",
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
  disabled,
  size = "md",
  className,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  children?: ReactNode;
  disabled?: boolean;
  size?: "md" | "sm";
  className?: string;
}) {
  return (
    <label
      className={cn(
        "flex cursor-pointer select-none items-center gap-2 text-mist-100",
        size === "sm" ? "text-[11px]" : "text-sm",
        disabled && "cursor-not-allowed opacity-50",
        className,
      )}
    >
      <span
        className={cn(
          "relative flex shrink-0 items-center justify-center rounded border transition duration-150 has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-elixir-500/70",
          size === "sm" ? "size-3.5" : "size-4",
          checked ? "border-elixir-500 bg-elixir-600" : "border-white/18 bg-white/5 hover:border-elixir-400/50",
          disabled && "hover:border-white/18",
        )}
      >
        <input
          type="checkbox"
          className="peer sr-only"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
        />
        <svg
          viewBox="0 0 12 12"
          className={cn(
            "text-white transition",
            size === "sm" ? "size-2" : "size-2.5",
            checked ? "opacity-100" : "opacity-0",
          )}
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
  disabled,
}: {
  value: string;
  options: Array<{ value: string; label: string; hint?: string }>;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const current = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    const onDoc = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={root} className={cn("relative min-w-[200px]", className)}>
      <button
        type="button"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className="field flex w-full items-center justify-between gap-3 rounded-lg text-left text-sm disabled:cursor-not-allowed disabled:opacity-50"
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
        <div
          role="listbox"
          className="popover absolute z-50 mt-1.5 max-h-64 w-full overflow-y-auto rounded-lg p-1"
        >
          {options.map((option) => (
            <button
              type="button"
              role="option"
              aria-selected={option.value === value}
              key={option.value}
              onClick={() => {
                onChange(option.value);
                setOpen(false);
              }}
              className={cn(
                "flex w-full cursor-pointer items-center justify-between gap-3 rounded-md px-2.5 py-1.5 text-left text-sm",
                option.value === value ? "bg-elixir-600 text-white" : "text-mist-100 hover:bg-white/8",
              )}
            >
              <span className="truncate">{option.label}</span>
              {option.hint ? (
                <span
                  className={cn(
                    "shrink-0 font-mono text-[11px]",
                    option.value === value ? "text-white/70" : "text-mist-300",
                  )}
                >
                  {option.hint}
                </span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

/** Opaque flyout panel. Use this — never `surface` — for dropdowns and menus. */
export function Popover({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("popover rounded-lg p-2", className)}>{children}</div>;
}

export function ProgressBar({
  value,
  unknown,
  className,
}: {
  value: number;
  unknown?: boolean;
  className?: string;
}) {
  const width = Math.min(100, Math.max(0, value));
  return (
    <div className={cn("progress-track", className)}>
      <div
        className={cn("progress-fill", unknown && "is-indeterminate")}
        style={unknown ? undefined : { width: `${width}%` }}
      />
    </div>
  );
}

/** Opaque dialog with enter/exit motion. Keep mounted and toggle `open` so hide can animate. */
export function Modal({
  open,
  onClose,
  title,
  subtitle,
  footer,
  children,
  size = "md",
  dismissible = true,
  className,
}: {
  open: boolean;
  onClose: () => void;
  title?: ReactNode;
  subtitle?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  size?: "md" | "lg";
  dismissible?: boolean;
  className?: string;
}) {
  const titleId = useId();
  const [shown, setShown] = useState(open);
  const [leaving, setLeaving] = useState(false);

  useEffect(() => {
    if (open) {
      setShown(true);
      setLeaving(false);
      return;
    }
    if (!shown) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setShown(false);
      setLeaving(false);
      return;
    }
    setLeaving(true);
  }, [open, shown]);

  useEffect(() => {
    if (!leaving) return;
    const id = window.setTimeout(() => {
      setShown(false);
      setLeaving(false);
    }, 240);
    return () => window.clearTimeout(id);
  }, [leaving]);

  useEffect(() => {
    if (!shown || !dismissible) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [shown, dismissible, onClose]);

  if (!shown) return null;

  return (
    <div
      className={cn("modal-root", leaving && "is-leave")}
      role="presentation"
      onAnimationEnd={(event) => {
        if (event.target !== event.currentTarget) return;
        if (leaving) {
          setShown(false);
          setLeaving(false);
        }
      }}
    >
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close"
        tabIndex={-1}
        disabled={!dismissible}
        onClick={() => {
          if (dismissible) onClose();
        }}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? titleId : undefined}
        className={cn("modal-panel", size === "lg" ? "max-w-2xl" : "max-w-lg", className)}
        onClick={(event) => event.stopPropagation()}
      >
        {title ? (
          <div className="flex shrink-0 items-start justify-between gap-3 border-b border-white/8 px-5 py-4">
            <div className="min-w-0">
              <h2 id={titleId} className="text-[15px] font-semibold tracking-tight text-mist-50">
                {title}
              </h2>
              {subtitle ? <p className="mt-1 text-[12px] leading-5 text-mist-300">{subtitle}</p> : null}
            </div>
            {dismissible ? (
              <button
                type="button"
                className="rounded-md p-1 text-mist-300 hover:bg-white/8 hover:text-mist-50"
                onClick={onClose}
                aria-label="Close"
              >
                <svg viewBox="0 0 14 14" className="size-3.5" fill="none" stroke="currentColor" strokeWidth="1.8">
                  <path d="M3 3l8 8M11 3l-8 8" />
                </svg>
              </button>
            ) : null}
          </div>
        ) : null}
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">{children}</div>
        {footer ? (
          <div className="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-white/8 px-5 py-3">
            {footer}
          </div>
        ) : null}
      </div>
    </div>
  );
}
