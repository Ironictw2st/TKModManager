import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { useStore } from "../state/store";
import { formatBytes, formatDate } from "../util/format";

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
  const gameBuildTime = useStore((s) => s.dll?.gameFingerprint?.timestamp ?? 0);
  const [notes, setNotes] = useState("");
  const [tagInput, setTagInput] = useState("");

  useEffect(() => {
    setNotes(meta?.notes ?? "");
    setTagInput("");
  }, [focused, meta?.notes]);

  if (!focused) {
    return (
      <aside className="w-80 shrink-0 border-l border-edge bg-panel p-3 text-textMuted text-[12px]">
        Select a mod to see its details.
      </aside>
    );
  }
  if (!mod) {
    return (
      <aside className="w-80 shrink-0 border-l border-edge bg-panel p-3 text-[12px]">
        <div className="font-semibold mb-1">Missing pack</div>
        <div className="text-textMuted break-all">{focused}</div>
        <div className="text-textMuted mt-2">
          This profile entry points at a pack that is no longer installed (unsubscribed or deleted). It is skipped at launch.
        </div>
      </aside>
    );
  }
  const ws = mod.workshopId ? workshop[mod.workshopId] : undefined;
  const title = ws?.title || mod.file.replace(/\.pack$/i, "");
  const tags = meta?.tags ?? [];
  const olderThanGame = !!ws?.timeUpdated && !ws.fromLauncherCache && gameBuildTime > 0 && ws.timeUpdated < gameBuildTime;
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
    <aside className="w-80 shrink-0 border-l border-edge bg-panel overflow-auto text-[12px]">
      {mod.previewPath && (
        <img src={convertFileSrc(mod.previewPath)} alt="" className="w-full aspect-square object-cover bg-sunken" draggable={false} />
      )}
      <div className="p-3 space-y-3">
        <div>
          <div className="font-semibold text-[13px] leading-tight select-text">{title}</div>
          <div className="text-textMuted break-all select-text">{mod.file}</div>
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

        {olderThanGame && (
          <div className="text-[11px] text-textMuted">
            Last updated on the Workshop before the current game build ({formatDate(gameBuildTime)}). Most mods keep working; worth a look if it misbehaves.
          </div>
        )}

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
    </aside>
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
