import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../state/store";

interface Conflict {
  path: string;
  packs: string[];
}
interface ConflictReport {
  conflicts: Conflict[];
  perPack: Record<string, number>;
  errors: Record<string, string>;
}

/** Files present in more than one enabled pack. The last pack in load order wins. */
export default function ConflictsPanel() {
  const profile = useStore((s) => s.activeProfile());
  const modsByKey = useStore((s) => s.modsByKey);
  const workshop = useStore((s) => s.workshop);
  const [report, setReport] = useState<ConflictReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [onlyPack, setOnlyPack] = useState<string | null>(null);

  const enabledKeys = useMemo(
    () => profile.entries.filter((e) => e.enabled && modsByKey[e.key]).map((e) => e.key),
    [profile, modsByKey],
  );

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      setReport(await invoke<ConflictReport>("conflicts_for", { keys: enabledKeys }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabledKeys.join("|")]);

  const label = (key: string) => {
    const m = modsByKey[key];
    if (!m) return key;
    return (m.workshopId && workshop[m.workshopId]?.title) || m.file;
  };

  const rows = (report?.conflicts ?? []).filter(
    (c) => (!filter || c.path.includes(filter.toLowerCase())) && (!onlyPack || c.packs.includes(onlyPack)),
  );
  const perPack = Object.entries(report?.perPack ?? {}).sort((a, b) => b[1] - a[1]);

  return (
    <div className="flex-1 min-w-0 flex">
      <aside className="w-72 shrink-0 border-r border-edge bg-panel overflow-auto text-[12px]">
        <div className="p-2 border-b border-edge flex items-center gap-2">
          <span className="font-semibold">Packs with conflicts</span>
          <div className="flex-1" />
          <button className="btn" onClick={run} disabled={busy}>
            {busy ? "Scanning…" : "Rescan"}
          </button>
        </div>
        <button
          className={`w-full text-left px-2 py-1 hover:bg-hover ${onlyPack === null ? "bg-selected" : ""}`}
          onClick={() => setOnlyPack(null)}
        >
          All ({report?.conflicts.length ?? 0} files)
        </button>
        {perPack.map(([key, n]) => (
          <button
            key={key}
            className={`w-full text-left px-2 py-1 hover:bg-hover flex gap-2 ${onlyPack === key ? "bg-selected" : ""}`}
            onClick={() => setOnlyPack(key)}
            title={key}
          >
            <span className="truncate flex-1">{label(key)}</span>
            <span className="text-textMuted">{n}</span>
          </button>
        ))}
        {Object.entries(report?.errors ?? {}).map(([key, err]) => (
          <div key={key} className="px-2 py-1 text-danger" title={err}>
            ⚠ {label(key)}: unreadable
          </div>
        ))}
      </aside>
      <section className="flex-1 min-w-0 flex flex-col text-[12px]">
        <div className="p-2 border-b border-edge flex items-center gap-2">
          <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Filter paths (e.g. db/land_units)" className="w-80" />
          <span className="text-textMuted">
            {rows.length} shared files · later in load order wins (movie packs always last)
          </span>
        </div>
        {error && <div className="p-2 text-danger">{error}</div>}
        <div className="flex-1 overflow-auto">
          <table className="w-full border-collapse">
            <thead className="sticky top-0 bg-panelHeader">
              <tr className="text-left text-textMuted">
                <th className="px-2 py-1 font-normal">File</th>
                <th className="px-2 py-1 font-normal">Provided by (load order) → winner</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((c) => (
                <tr key={c.path} className="border-t border-edge/50 hover:bg-hover align-top">
                  <td className="px-2 py-1 font-mono text-[11px] break-all select-text">{c.path}</td>
                  <td className="px-2 py-1">
                    {c.packs.map((k, i) => (
                      <span key={k}>
                        {i > 0 && <span className="text-textMuted"> → </span>}
                        <span className={i === c.packs.length - 1 ? "text-ok font-semibold" : ""} title={k}>
                          {label(k)}
                        </span>
                      </span>
                    ))}
                  </td>
                </tr>
              ))}
              {!busy && rows.length === 0 && (
                <tr>
                  <td colSpan={2} className="px-2 py-6 text-center text-textMuted">
                    No overlapping files among the enabled packs.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
