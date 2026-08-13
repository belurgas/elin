import { useEffect } from "react";
import { cn } from "../lib/cn";

export type MenuItem =
  | { kind: "item"; label: string; onClick: () => void; muted?: boolean }
  | { kind: "sep" };

export function ContextMenu({
  x,
  y,
  items,
  onClose,
}: {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}) {
  useEffect(() => {
    const close = () => onClose();
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", close);
    };
  }, [onClose]);

  return (
    <div
      className="studio-menu"
      style={{ left: Math.min(x, window.innerWidth - 220), top: Math.min(y, window.innerHeight - 160) }}
      onMouseDown={(e) => e.stopPropagation()}
    >
      {items.map((item, i) =>
        item.kind === "sep" ? (
          <div key={`s${i}`} className="studio-menu-sep" />
        ) : (
          <button
            key={`${item.label}-${i}`}
            type="button"
            className={cn("studio-menu-item", item.muted && "is-muted")}
            onClick={() => {
              item.onClick();
              onClose();
            }}
          >
            {item.label}
          </button>
        ),
      )}
    </div>
  );
}
