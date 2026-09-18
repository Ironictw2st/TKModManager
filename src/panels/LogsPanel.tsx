import { useEffect, useMemo, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { api, type LogSource } from "../ipc/commands";
import { formatBytes } from "../util/format";

/** Live view of the script extender log and the game-root logs mods write. */
export default function LogsPanel() {
  const [sources, setSources] = useState<LogSource[]>([]);
  const [current, setCurrent] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [follow, setFollow] = useState(true);
  const [filter, setFilter] = useState("");
  const box = useRef<HTMLPreElement>(null);

  // Refresh the source list (newest script/ironic log changes per session) and the text.
  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      try {
        const list = await api.logSources();
        if (cancelled) return;
        setSources(list);
        const id = current && list.some((s) => s.id === current) ? current : (list[0]?.id ?? null);
        if (id !== current) setCurrent(id);
        const src = list.find((s) => s.id === id);
        if (src) {
          const t = await api.logTail(src.path, 512 * 1024);
          if (!cancelled) {
            setText(t);
            setError(null);
          }
        } else if (!cancelled) setText("");
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    };
    void tick();
    const id = window.setInterval(tick, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [current]);

  const shown = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return text;
    return text
      .split("\n")
      .filter((l) => l.toLowerCase().includes(q))
      .join("\n");
  }, [text, filter]);

  useEffect(() => {
    if (follow && box.current) box.current.scrollTop = box.current.scrollHeight;
  }, [shown, follow]);

  const src = sources.find((s) => s.id === current);

  return (
    <div className="flex-1 min-w-0 flex flex-col text-[12px]">
      <div className="flex items-center gap-2 px-2 py-1.5 border-b border-edge">
        <select value={current ?? ""} onChange={(e) => setCurrent(e.target.value)} className="min-w-56">
          {sources.length === 0 && <option value="">No logs found yet</option>}
          {sources.map((s) => (
            <option key={s.id} value={s.id}>
              {s.label}
            </option>
          ))}
        </select>
        <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Filter lines (e.g. error)" className="w-64" />
        <label className="flex items-center gap-1 cursor-pointer">
          <input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} />
          follow
        </label>
        <div className="flex-1" />
        {src && (
          <>
            <span className="text-textMuted truncate max-w-[40%]" title={src.path}>
              {src.path.split(/[\\/]/).pop()} · {formatBytes(src.size)}
            </span>
            <button className="btn" onClick={() => void revealItemInDir(src.path)}>
              Show file
            </button>
          </>
        )}
      </div>
      {error && <div className="px-2 py-1 text-danger">{error}</div>}
      <pre ref={box} className="flex-1 overflow-auto m-0 p-2 font-mono text-[11px] leading-4 whitespace-pre-wrap break-words select-text bg-sunken">
        {shown || (sources.length ? "(empty)" : "Logs appear here once the game has run: the script extender log, lua_mod_log.txt, and the newest script / ironic logs.")}
      </pre>
    </div>
  );
}
