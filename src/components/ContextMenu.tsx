import { useEffect, useRef } from "react";

export interface MenuItem {
  label: string;
  onClick?: () => void;
  disabled?: boolean;
  separator?: boolean;
}

/** Minimal right-click menu anchored at a screen position; closes on click-away / Escape. */
export default function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const away = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("mousedown", away);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", away);
      window.removeEventListener("keydown", key);
    };
  }, [onClose]);

  // Keep the menu on screen.
  const left = Math.min(x, window.innerWidth - 220);
  const top = Math.min(y, window.innerHeight - items.length * 26 - 12);

  return (
    <div ref={ref} className="fixed z-50 min-w-48 rounded border border-edge bg-panel shadow-xl py-1 text-[12px]" style={{ left, top }}>
      {items.map((it, i) =>
        it.separator ? (
          <div key={i} className="my-1 border-t border-edge" />
        ) : (
          <button
            key={i}
            className="w-full text-left px-3 py-1 hover:bg-hover disabled:opacity-50 disabled:hover:bg-transparent"
            disabled={it.disabled}
            onClick={() => {
              it.onClick?.();
              onClose();
            }}
          >
            {it.label}
          </button>
        ),
      )}
    </div>
  );
}
