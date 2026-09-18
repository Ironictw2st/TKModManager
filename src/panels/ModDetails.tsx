import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useStore } from "../state/store";
import { formatBytes, formatDate } from "../util/format";
import { modStatus, seRequirement, seSourceText, seUnmet as seUnmetOf, seUnmetText, STATUS_LABEL, type ModStatusKind } from "../util/status";

const STATUS_BOX: Record<ModStatusKind, string> = {
  pending: "border-danger bg-danger/10 text-danger",
  old: "border-warn bg-warn/10 text-warn",
  ok: "border-ok bg-ok/10 text-ok",
  unknown: "border-edge text-textMuted",
  local: "border-edge text-textMuted",
};

/** Right-hand pane: preview, description, Workshop link, dependencies, tags and notes. */
export default function ModDetails() {
  const focused = useStore((s) => s.focused);
  const mod = useStore((s) => (focused ? s.modsByKey[focused] : undefined));
  const workshop = useStore((s) => s.workshop);
  const meta = useStore((s) => (focused ? s.meta.mods[focused] : undefined));
  const setMeta = useStore((s) => s.setMeta);
  const profile = useStore((s) => s.activeProfile());
  const mods = useStore((s) => s.mods);
  const toggleMods = useStore((s) => s.toggleMods);
  const cutoff = useStore((s) => s.outdatedCutoff());
  const seScan = useStore((s) => (focused ? s.seScan[focused] : undefined));
  const setSeOverride = useStore((s) => s.setSeOverride);
  const dll = useStore((s) => s.dll);
  const [notes, setNotes] = useState("");
  const [tagInput, setTagInput] = useState("");

  useEffect(() => {
    setNotes(meta?.notes ?? "");
    setTagInput("");
  }, [focused, meta?.notes]);

  if (!focused) {
    return (
      <div className="p-3 text-textMuted text-[12px] space-y-2">
        <div>Select a mod to see its details.</div>
        <div className="text-[11px]">
          The dot next to each mod shows its status: green up to date, amber last updated before the current game build, red update pending
          on Steam, grey local file. <b>SE</b> marks mods that need the script extender.
        </div>
      </div>
    );
  }
  if (!mod) {
    return (
      <div className="p-3 text-[12px]">
        <div className="font-semibold mb-1">Missing pack</div>
        <div className="text-textMuted break-all">{focused}</div>
        <div className="text-textMuted mt-2">
          This profile entry points at a pack that is no longer installed (unsubscribed or deleted). It is skipped at launch.
        </div>
      </div>
    );
  }
  const ws = mod.workshopId ? workshop[mod.workshopId] : undefined;
  const title = ws?.title || mod.file.replace(/\.pack$/i, "");
  const tags = meta?.tags ?? [];
  const status = modStatus(mod, ws, cutoff);
  const se = seRequirement(seScan, meta);
  const entry = profile.entries.find((e) => e.key === mod.key);
  const have = dll?.selected?.version;
  const unmet = seUnmetOf(se, profile.dll, have);
  const seUnmet = !!entry?.enabled && unmet !== null;
  const overrideValue = meta?.seOverride === true ? "yes" : meta?.seOverride === false ? "no" : "auto";
  const enabledKeys = new Set(profile.entries.filter((e) => e.enabled).map((e) => e.key));
  const required = (ws?.requiredItems ?? []).map((id) => {
    const installed = mods.find((m) => m.workshopId === id);
    return { id, installed, enabled: installed ? enabledKeys.has(installed.key) : false, title: workshop[id]?.title };
  });

  const addTag = async () => {
    const t = tagInput.trim();
    if (!t || tags.includes(t)) return;
    await setMeta(mod.key, { tags: [...tags, t] });
    setTagInput("");
  };

  return (
    <div className="text-[12px]">
      {mod.previewPath && (
        <img src={convertFileSrc(mod.previewPath)} alt="" className="w-full h-40 object-cover bg-sunken" draggable={false} />
      )}
      <div className="p-3 space-y-3">
        <div>
          <div className="font-semibold text-[13px] leading-tight select-text">{title}</div>
          <div className="text-textMuted break-all select-text">{mod.file}</div>
        </div>

        <div className={`rounded border px-2 py-1.5 ${STATUS_BOX[status.kind]}`}>
          <div className="font-semibold">{STATUS_LABEL[status.kind]}</div>
          <div className="text-[11px] text-textMuted">{status.text}</div>
        </div>

        <div className={`rounded border px-2 py-1.5 space-y-1 ${se.required ? (seUnmet ? "border-danger bg-danger/10" : "border-se/60 bg-se/10") : "border-edge"}`}>
          <div className="flex items-center gap-2">
            <span className={`font-semibold ${se.required ? (seUnmet ? "text-danger" : "text-se") : "text-textMuted"}`}>
              {se.required ? "Needs the script extender" : "No script extender needed"}
            </span>
          </div>
          <div className="text-[11px] text-textMuted break-words">{seSourceText(se)}</div>
          {se.notes && <div className="text-[11px] break-words whitespace-pre-wrap select-text">{se.notes}</div>}
          {se.error && <div className="text-[11px] text-warn break-words">{se.error}</div>}
          {unmet && (
            <div className={`text-[11px] ${seUnmet ? "text-danger" : "text-textMuted"}`}>
              {seUnmetText(unmet, se, have)}
              {!seUnmet && " (applies once this mod is enabled)"}
            </div>
          )}
          <label className="flex items-center gap-2 text-[11px]">
            <span className="text-textMuted">Requirement</span>
            <select
              value={overrideValue}
              onChange={(e) => void setSeOverride(mod.key, e.target.value === "auto" ? null : e.target.value === "yes")}
              className="flex-1"
            >
              <option value="auto">Automatic{seScan?.required ? " (detected)" : ""}</option>
              <option value="yes">Requires the script extender</option>
              <option value="no">Does not require it</option>
            </select>
          </label>
        </div>
        <div className="flex flex-wrap gap-1">
          <span className={`badge ${mod.source === "workshop" ? "border-accent/60 text-accent" : "border-edge text-textMuted"}`} title={mod.dir}>
            {mod.source === "workshop" ? "Workshop" : mod.source === "folder" ? "Folder" : "data/"}
          </span>
          <span className={`badge ${mod.packType === "movie" ? "border-warn text-warn" : "border-edge text-textMuted"}`}>
            {mod.packType === "movie" ? "MOVIE" : "mod"}
          </span>
          <span className="badge border-edge text-textMuted">{formatBytes(mod.size)}</span>
          <span className="badge border-edge text-textMuted" title="File modified">
            {formatDate(mod.mtime)}
          </span>
          {ws?.timeUpdated ? (
            <span className="badge border-edge text-textMuted" title="Workshop updated">
              ws {formatDate(ws.timeUpdated)}
            </span>
          ) : null}
        </div>
        {mod.packType === "movie" && (
          <div className="text-[11px] text-warn">
            Movie packs load after every mod pack and cannot be ordered. Enabled = loaded; disabled = excluded via the mod list. Files are never
            renamed or moved.
          </div>
        )}
        <div className="flex gap-2">
          {mod.workshopId && (
            <button className="btn" onClick={() => void openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${mod.workshopId}`)}>
              Open in Workshop
            </button>
          )}
          <button className="btn" onClick={() => void revealItemInDir(mod.path)}>
            Show file
          </button>
        </div>

        {required.length > 0 && (
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="font-semibold">Required items</span>
              {required.some((r) => r.installed && !r.enabled) && (
                <button
                  className="btn"
                  onClick={() => void toggleMods(required.filter((r) => r.installed && !r.enabled).map((r) => r.installed!.key), true)}
                >
                  Enable all
                </button>
              )}
            </div>
            <ul className="space-y-0.5">
              {required.map((r) => (
                <li key={r.id} className="flex items-center gap-1.5">
                  <span className={`badge ${!r.installed ? "border-danger text-danger" : r.enabled ? "border-ok text-ok" : "border-warn text-warn"}`}>
                    {!r.installed ? "missing" : r.enabled ? "on" : "off"}
                  </span>
                  <button className="hover:underline text-left" onClick={() => void openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${r.id}`)}>
                    {r.title ?? r.installed?.file ?? r.id}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}

        <div>
          <div className="font-semibold mb-1">Tags</div>
          <div className="flex flex-wrap gap-1 mb-1">
            {tags.map((t) => (
              <span key={t} className="badge border-accent/60 text-accent flex items-center gap-1">
                {t}
                <button className="text-textMuted hover:text-text" onClick={() => void setMeta(mod.key, { tags: tags.filter((x) => x !== t) })}>
                  ✕
                </button>
              </span>
            ))}
          </div>
          <div className="flex gap-1">
            <input
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && void addTag()}
              placeholder="Add tag…"
              className="flex-1"
            />
            <button className="btn" onClick={addTag}>
              Add
            </button>
          </div>
          <label className="flex items-center gap-1.5 mt-2 cursor-pointer">
            <input type="checkbox" checked={!!meta?.hidden} onChange={(e) => void setMeta(mod.key, { hidden: e.target.checked })} />
            Hidden (only shown with "show hidden")
          </label>
        </div>

        <div>
          <div className="font-semibold mb-1">Notes</div>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            onBlur={() => notes !== (meta?.notes ?? "") && void setMeta(mod.key, { notes })}
            rows={3}
            className="w-full resize-y"
            placeholder="Your notes for this mod"
          />
        </div>

        {ws?.description && (
          <div>
            <div className="font-semibold mb-1">Description</div>
            <div className="text-textMuted whitespace-pre-wrap break-words select-text max-h-64 overflow-auto">{stripBb(ws.description)}</div>
          </div>
        )}
      </div>
    </div>
  );
}

/** Steam descriptions are BBCode; strip tags for a readable plain-text view. */
function stripBb(s: string): string {
  return s
    .replace(/\[img\][^[]*\[\/img\]/gi, "")
    .replace(/\[url=[^\]]*\]/gi, "")
    .replace(/\[\/?[a-z0-9*]+(=[^\]]*)?\]/gi, "")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}
