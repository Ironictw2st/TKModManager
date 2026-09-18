import { useState } from "react";
import { askText } from "../components/Dialogs";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, type CollectionResult } from "../ipc/commands";
import { useStore } from "../state/store";
import { isSeparator } from "../state/separators";
import { buildExport, parseExport, verifyExport, type SyncResult, type SyncStatus } from "../util/sync";
import { formatBytes } from "../util/format";

const STATUS_STYLE: Record<SyncStatus, string> = {
  match: "border-ok text-ok",
  unverified: "border-edge text-textMuted",
  disabled: "border-warn text-warn",
  different: "border-danger text-danger",
  missing: "border-danger text-danger",
};
const STATUS_LABEL: Record<SyncStatus, string> = {
  match: "identical",
  unverified: "present (no hash)",
  disabled: "installed, not enabled",
  different: "different file",
  missing: "not installed",
};

/** Import (CA list, Workshop collection, text), export with hashes, and the co-op sync check. */
export default function ProfileTools() {
  const paths = useStore((s) => s.paths);
  const profiles = useStore((s) => s.profiles);
  const activeProfile = useStore((s) => s.activeProfile);
  const createProfile = useStore((s) => s.createProfile);
  const updateProfile = useStore((s) => s.updateProfile);
  const mods = useStore((s) => s.mods);
  const modsByKey = useStore((s) => s.modsByKey);
  const setWorkshop = useStore((s) => s.setWorkshop);
  const [text, setText] = useState("");
  const [mode, setMode] = useState<"export" | "import" | "verify" | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<string | null>(null);
  const [result, setResult] = useState<SyncResult | null>(null);
  const [collection, setCollection] = useState<{ res: CollectionResult; missing: string[] } | null>(null);

  const enabledKeys = () =>
    activeProfile()
      .entries.filter((e) => e.enabled && !isSeparator(e) && modsByKey[e.key])
      .map((e) => e.key);

  /** Create a profile with `keys` enabled first, in that order. */
  const makeProfile = async (name: string, keys: string[]) => {
    await createProfile(name);
    await updateProfile(name, (p) => {
      const rest = p.entries.filter((e) => !keys.includes(e.key));
      return { ...p, entries: [...keys.map((key) => ({ key, enabled: true })), ...rest] };
    });
  };

  const hashWithProgress = async (keys: string[]) => {
    const un = await listen<{ file: string; doneBytes: number; totalBytes: number }>("hash-progress", (e) =>
      setProgress(`Hashing ${e.payload.file} — ${formatBytes(e.payload.doneBytes)} / ${formatBytes(e.payload.totalBytes)}`),
    );
    try {
      return await api.hashPacks(keys);
    } finally {
      un();
      setProgress(null);
    }
  };

  const importCa = async () => {
    let file = paths?.usedModsFile ?? null;
    if (!file) {
      const picked = await open({ multiple: false, filters: [{ name: "Mod list", extensions: ["txt"] }], title: "Select used_mods.txt" });
      if (typeof picked !== "string") return;
      file = picked;
    }
    const name = await askText("Name for the imported profile:", "CA launcher");
    if (!name) return;
    try {
      const imported = await api.importModList(file);
      const keys = imported.map((i) => i.key).filter((k): k is string => !!k);
      const unmatched = imported.filter((i) => !i.key).map((i) => i.file);
      await makeProfile(name, keys);
      setMsg(`Imported ${keys.length} mods into "${name}"${unmatched.length ? `; not installed: ${unmatched.join(", ")}` : ""}`);
    } catch (e) {
      setMsg(String(e instanceof Error ? e.message : e));
    }
  };

  const importCollection = async () => {
    const input = await askText("Workshop collection link or id:");
    if (!input) return;
    setBusy(true);
    setMsg(null);
    try {
      const res = await api.workshopCollection(input);
      setWorkshop(res.items);
      const keys = res.children.map((id) => mods.find((m) => m.workshopId === id)?.key).filter((k): k is string => !!k);
      const missing = res.children.filter((id) => !mods.some((m) => m.workshopId === id));
      const name = await askText("Name for the new profile:", res.title || `Collection ${res.id}`);
      if (!name) return;
      await makeProfile(name, keys);
      setCollection({ res, missing });
      setMsg(`"${name}": ${keys.length} of ${res.children.length} collection items are installed and enabled.`);
    } catch (e) {
      setMsg(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };

  const exportText = async () => {
    setBusy(true);
    setMsg(null);
    try {
      const hashes = await hashWithProgress(enabledKeys());
      setText(buildExport(profiles.active, hashes));
      setMode("export");
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  const importText = async () => {
    const rows = parseExport(text);
    if (!rows.length) return setMsg("Nothing to import");
    const name = await askText("Name for the imported profile:", "Imported");
    if (!name) return;
    // Workshop rows keep their key (shown as missing when not subscribed); others match by file.
    const keys = rows.map((r) => (r.key.startsWith("ws:") ? r.key : (mods.find((m) => m.source !== "workshop" && m.file.toLowerCase() === r.file.toLowerCase())?.key ?? r.key)));
    const unknown = keys.filter((k) => !modsByKey[k]).length;
    try {
      await makeProfile(name, keys);
      setMsg(`Imported ${keys.length - unknown} mods into "${name}"${unknown ? `; ${unknown} not installed (kept as missing)` : ""}`);
      setMode(null);
    } catch (e) {
      setMsg(String(e instanceof Error ? e.message : e));
    }
  };

  const verify = async () => {
    const rows = parseExport(text);
    if (!rows.length) return setMsg("Paste a profile export first");
    setBusy(true);
    setMsg(null);
    try {
      const enabled = enabledKeys();
      const needHash = rows.some((r) => r.sha256);
      const hashes = needHash ? await hashWithProgress(enabled) : [];
      const map: Record<string, string> = {};
      for (const h of hashes) map[h.key] = h.sha256;
      setResult(verifyExport(rows, mods, enabled, map));
    } catch (e) {
      setMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="rounded border border-edge bg-panel p-3 space-y-2 md:col-span-2">
      <div className="font-semibold text-[13px]">Profiles: import, export and co-op sync check</div>
      <div className="flex items-center gap-2 flex-wrap">
        <button className="btn" onClick={importCa} disabled={busy} title={paths?.usedModsFile ?? "Pick a used_mods.txt"}>
          Import CA launcher list…
        </button>
        <button className="btn" onClick={importCollection} disabled={busy}>
          Import Workshop collection…
        </button>
        <button
          className="btn"
          disabled={busy}
          onClick={() => {
            setText("");
            setResult(null);
            setMode("import");
          }}
        >
          Import from text…
        </button>
        <span className="w-px h-4 bg-edge" />
        <button className="btn" onClick={exportText} disabled={busy} title="Enabled packs in load order, with size and sha256 so a partner can verify them">
          Export "{profiles.active}" (with hashes)
        </button>
        <button
          className="btn"
          disabled={busy}
          title="Paste a partner's export and compare it with your active profile"
          onClick={() => {
            setText("");
            setResult(null);
            setMode("verify");
          }}
        >
          Verify against an export…
        </button>
      </div>
      {progress && <div className="text-textMuted">{progress}</div>}
      {mode && (
        <div className="space-y-1">
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            readOnly={mode === "export"}
            rows={7}
            className="w-full font-mono text-[11px] select-text"
            placeholder={mode === "verify" ? "Paste your partner's export here, then Compare" : "Paste a profile export here (one pack per line)"}
          />
          <div className="flex gap-2">
            {mode === "export" && (
              <button className="btn" onClick={() => void navigator.clipboard?.writeText(text)}>
                Copy to clipboard
              </button>
            )}
            {mode === "import" && (
              <button className="btn-accent" onClick={importText} disabled={busy}>
                Create profile
              </button>
            )}
            {mode === "verify" && (
              <button className="btn-accent" onClick={verify} disabled={busy}>
                {busy ? "Comparing…" : `Compare with "${profiles.active}"`}
              </button>
            )}
            <button
              className="btn"
              onClick={() => {
                setMode(null);
                setResult(null);
              }}
            >
              Close
            </button>
          </div>
        </div>
      )}
      {result && mode === "verify" && (
        <div className="space-y-1">
          <div className={result.ok ? "text-ok font-semibold" : "text-danger font-semibold"}>
            {result.ok ? "In sync: same packs, same files, same order." : "Not in sync — see below."}
          </div>
          <table className="w-full">
            <tbody>
              {result.rows.map((r) => (
                <tr key={r.row.key} className="border-t border-edge/50">
                  <td className="py-0.5 pr-2 w-44">
                    <span className={`badge ${STATUS_STYLE[r.status]}`}>{STATUS_LABEL[r.status]}</span>
                  </td>
                  <td className="py-0.5 pr-2 select-text">{r.row.file}</td>
                  <td className="py-0.5 text-right">
                    {r.row.key.startsWith("ws:") && r.status !== "match" && (
                      <button className="hover:underline text-accent" onClick={() => void openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${r.row.key.slice(3, r.row.key.indexOf("/"))}`)}>
                        Workshop page
                      </button>
                    )}
                  </td>
                </tr>
              ))}
              {result.extra.map((m) => (
                <tr key={m.key} className="border-t border-edge/50">
                  <td className="py-0.5 pr-2">
                    <span className="badge border-warn text-warn">only on your side</span>
                  </td>
                  <td className="py-0.5 select-text" colSpan={2}>
                    {m.file}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {result.orderDiffers && <div className="text-warn">The common packs load in a different order on your side.</div>}
        </div>
      )}
      {collection && collection.missing.length > 0 && (
        <div>
          <div className="text-warn mb-1">Not installed ({collection.missing.length}) — subscribe, then press Refresh in Settings → Game:</div>
          <ul className="space-y-0.5">
            {collection.missing.map((id) => (
              <li key={id}>
                <button className="hover:underline text-accent" onClick={() => void openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`)}>
                  {collection.res.items[id]?.title || id}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
      {msg && <div className="text-textMuted">{msg}</div>}
    </section>
  );
}
