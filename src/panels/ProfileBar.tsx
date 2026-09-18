import { useState } from "react";
import { askConfirm, showMessage } from "../components/Dialogs";
import { useStore } from "../state/store";
import { api } from "../ipc/commands";
import { isSeparator } from "../state/separators";

/** Profile picker with new / duplicate / rename / delete. */
export default function ProfileBar() {
  const profiles = useStore((s) => s.profiles);
  const setActive = useStore((s) => s.setActiveProfile);
  const create = useStore((s) => s.createProfile);
  const rename = useStore((s) => s.renameProfile);
  const remove = useStore((s) => s.deleteProfile);
  const activeProfile = useStore((s) => s.activeProfile);
  const gameRunning = useStore((s) => s.gameRunning);
  const [mode, setMode] = useState<null | "new" | "dup" | "rename">(null);
  const [name, setName] = useState("");
  const [err, setErr] = useState<string | null>(null);

  const start = (m: NonNullable<typeof mode>) => {
    setMode(m);
    setName(m === "rename" ? profiles.active : m === "dup" ? `${profiles.active} copy` : "");
    setErr(null);
  };

  const commit = async () => {
    const n = name.trim();
    if (!n) return;
    try {
      if (mode === "new") await create(n);
      else if (mode === "dup") await create(n, activeProfile());
      else if (mode === "rename") await rename(profiles.active, n);
      setMode(null);
    } catch (e) {
      setErr(String(e instanceof Error ? e.message : e));
    }
  };

  const del = async () => {
    if (!(await askConfirm(`Delete profile "${profiles.active}"?`, "Delete"))) return;
    try {
      await remove(profiles.active);
    } catch (e) {
      void showMessage(String(e instanceof Error ? e.message : e));
    }
  };

  return (
    <div className="flex items-center gap-1.5 text-[12px]">
      <span className="text-textMuted">Profile</span>
      {mode ? (
        <>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void commit();
              if (e.key === "Escape") setMode(null);
            }}
            className="w-44"
            placeholder="Profile name"
          />
          <button className="btn-accent" onClick={commit}>
            {mode === "rename" ? "Rename" : "Create"}
          </button>
          <button className="btn" onClick={() => setMode(null)}>
            Cancel
          </button>
          {err && <span className="text-danger">{err}</span>}
        </>
      ) : (
        <>
          <select value={profiles.active} onChange={(e) => void setActive(e.target.value)} disabled={gameRunning} className="min-w-40">
            {profiles.profiles.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name} ({p.entries.filter((e) => e.enabled && !isSeparator(e)).length})
              </option>
            ))}
          </select>
          <button className="btn" onClick={() => start("new")} title="New empty profile">
            New
          </button>
          <button className="btn" onClick={() => start("dup")} title="Duplicate the active profile">
            Duplicate
          </button>
          <button className="btn" onClick={() => start("rename")} title="Rename the active profile">
            Rename
          </button>
          <button className="btn" onClick={del} title="Delete the active profile" disabled={profiles.profiles.length <= 1}>
            Delete
          </button>
          <button
            className="btn"
            title="Create a desktop shortcut that launches the game with this profile"
            onClick={async () => {
              try {
                void showMessage(`Shortcut created:\n${await api.createProfileShortcut(profiles.active)}`);
              } catch (e) {
                void showMessage(String(e));
              }
            }}
          >
            Shortcut
          </button>
        </>
      )}
    </div>
  );
}
