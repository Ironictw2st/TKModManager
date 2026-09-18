// In-app replacements for window.confirm / prompt / alert. Tauri shims those browser calls
// onto its dialog plugin (and `prompt` is not supported at all), so they are unreliable in the
// webview. These are promise-based and rendered by <DialogHost /> (mounted once in App).

import { useEffect, useRef, useState } from "react";

type Request =
  | { kind: "confirm"; message: string; okLabel?: string; resolve: (v: boolean) => void }
  | { kind: "prompt"; message: string; initial: string; resolve: (v: string | null) => void }
  | { kind: "alert"; message: string; resolve: () => void };

let push: ((r: Request) => void) | null = null;

export function askConfirm(message: string, okLabel?: string): Promise<boolean> {
  return new Promise((resolve) => (push ? push({ kind: "confirm", message, okLabel, resolve }) : resolve(false)));
}

export function askText(message: string, initial = ""): Promise<string | null> {
  return new Promise((resolve) => (push ? push({ kind: "prompt", message, initial, resolve }) : resolve(null)));
}

export function showMessage(message: string): Promise<void> {
  return new Promise((resolve) => (push ? push({ kind: "alert", message, resolve }) : resolve()));
}

export function DialogHost() {
  const [queue, setQueue] = useState<Request[]>([]);
  const [text, setText] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const current = queue[0];

  useEffect(() => {
    push = (r) => setQueue((q) => [...q, r]);
    return () => {
      push = null;
    };
  }, []);

  useEffect(() => {
    if (current?.kind === "prompt") {
      setText(current.initial);
      window.setTimeout(() => input.current?.select(), 0);
    }
  }, [current]);

  if (!current) return null;

  const finish = (ok: boolean) => {
    if (current.kind === "confirm") current.resolve(ok);
    else if (current.kind === "prompt") current.resolve(ok ? text : null);
    else current.resolve();
    setQueue((q) => q.slice(1));
  };

  return (
    <div
      className="fixed inset-0 z-[60] bg-black/50 flex items-center justify-center"
      onKeyDown={(e) => {
        if (e.key === "Escape") finish(false);
        if (e.key === "Enter") finish(true);
      }}
    >
      <div className="w-[420px] rounded-lg border border-edge bg-panel p-4 text-[12px] space-y-3 shadow-xl">
        <div className="whitespace-pre-wrap break-words select-text">{current.message}</div>
        {current.kind === "prompt" && (
          <input ref={input} autoFocus value={text} onChange={(e) => setText(e.target.value)} className="w-full" />
        )}
        <div className="flex justify-end gap-2">
          {current.kind !== "alert" && (
            <button className="btn" onClick={() => finish(false)}>
              Cancel
            </button>
          )}
          <button className="btn-accent" autoFocus={current.kind !== "prompt"} onClick={() => finish(true)}>
            {current.kind === "confirm" ? (current.okLabel ?? "OK") : "OK"}
          </button>
        </div>
      </div>
    </div>
  );
}
