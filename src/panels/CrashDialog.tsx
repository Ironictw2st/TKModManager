import { useEffect, useState } from "react";
import { api, type CrashReport } from "../ipc/commands";
import { useStore } from "../state/store";
import { formatDate } from "../util/format";

/** Details for an abnormal game exit: what changed since the last clean launch + log tail. */
export default function CrashDialog() {
  const crash = useStore((s) => s.crash);
  const clearCrash = useStore((s) => s.clearCrash);
  const setPanel = useStore((s) => s.setPanel);
  const [open, setOpen] = useState(false);
  const [report, setReport] = useState<CrashReport | null>(null);
  const [logTail, setLogTail] = useState("");

  useEffect(() => {
    if (!open || !crash) return;
    let cancelled = false;
    (async () => {
      const r = await api.crashReport(crash.historyId).catch(() => null);
      if (cancelled) return;
      setReport(r);
      try {
        const sources = await api.logSources();
        const src = sources.find((s) => s.id === "script_log") ?? sources.find((s) => s.id === "dll") ?? sources[0];
        if (src) {
          const t = await api.logTail(src.path, 16 * 1024);
          if (!cancelled) setLogTail(`${src.label}\n${t.split("\n").slice(-40).join("\n")}`);
        }
      } catch {
        /* no logs */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, crash]);

  if (!crash) return null;

  if (!open) {
    return (
      <div className="fixed z-40 left-3 bottom-14 rounded border border-danger bg-sunken shadow-xl text-[12px] px-3 py-2 flex items-center gap-2">
        <span className="text-danger font-semibold">The game ended abnormally</span>
        <span className="text-textMuted">exit code {crash.exitCode != null ? `0x${crash.exitCode.toString(16).toUpperCase()}` : "?"}</span>
        <button className="btn" onClick={() => setOpen(true)}>
          Details
        </button>
        <button className="text-textMuted hover:text-text" onClick={clearCrash} title="Dismiss">
          ✕
        </button>
      </div>
    );
  }

  const d = report?.diff;
  const nothing = d && !d.added.length && !d.removed.length && !d.changed.length && !d.reordered;

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center" onClick={() => setOpen(false)}>
      <div className="w-[720px] max-h-[80vh] overflow-auto rounded-lg border border-edge bg-panel p-4 text-[12px] space-y-3" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-2">
          <div className="font-semibold text-[14px]">Abnormal exit</div>
          <span className="text-textMuted">
            profile {report?.record.profile ?? "?"} · exit code {crash.exitCode != null ? `0x${crash.exitCode.toString(16).toUpperCase()}` : "?"}
          </span>
          <div className="flex-1" />
          <button className="btn" onClick={() => setOpen(false)}>
            Close
          </button>
        </div>
        <div className="text-textMuted">
          An exit code other than 0 usually means a crash, but it is also what you get when the game is closed from Task Manager.
        </div>
        <div>
          <div className="font-semibold mb-1">
            Changed since the last clean launch{report?.baseline ? ` (${formatDate(report.baseline.started)}, ${report.baseline.profile})` : ""}
          </div>
          {!report?.baseline && <div className="text-textMuted">No earlier launch that exited cleanly is on record yet.</div>}
          {nothing && <div className="text-textMuted">Nothing: same packs, same files, same order.</div>}
          {d && (
            <ul className="space-y-0.5">
              {d.added.map((f) => (
                <li key={`a${f}`}>
                  <span className="badge border-ok text-ok">added</span> {f}
                </li>
              ))}
              {d.changed.map((f) => (
                <li key={`c${f}`}>
                  <span className="badge border-warn text-warn">file changed</span> {f}
                </li>
              ))}
              {d.removed.map((f) => (
                <li key={`r${f}`}>
                  <span className="badge border-danger text-danger">removed</span> {f}
                </li>
              ))}
              {d.reordered && (
                <li>
                  <span className="badge border-edge text-textMuted">order</span> the load order of the common packs changed
                </li>
              )}
            </ul>
          )}
        </div>
        {logTail && (
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="font-semibold">Last log lines</span>
              <button
                className="btn"
                onClick={() => {
                  setOpen(false);
                  setPanel("logs");
                }}
              >
                Open Logs tab
              </button>
            </div>
            <pre className="m-0 p-2 max-h-64 overflow-auto font-mono text-[11px] leading-4 whitespace-pre-wrap break-words select-text bg-sunken rounded">{logTail}</pre>
          </div>
        )}
      </div>
    </div>
  );
}
