import { useState } from "react";
import { askConfirm, askText, showMessage } from "../components/Dialogs";
import ContextMenu, { type MenuItem } from "../components/ContextMenu";
import { useStore } from "../state/store";
import { api } from "../ipc/commands";
import { isSeparator } from "../state/separators";

/** Compact profile picker; profile actions live in the "⋯" menu. */
export default function ProfileBar() {
  const profiles = useStore((s) => s.profiles);
  const setActive = useStore((s) => s.setActiveProfile);
  const create = useStore((s) => s.createProfile);
  const rename = useStore((s) => s.renameProfile);
  const remove = useStore((s) => s.deleteProfile);
  const activeProfile = useStore((s) => s.activeProfile);
  const gameRunning = useStore((s) => s.gameRunning);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);

  const run = async (fn: () => Promise<void>) => {
    try {
      await fn();
    } catch (e) {
      void showMessage(String(e instanceof Error ? e.message : e));
    }
  };

  const items: MenuItem[] = [
    {
      label: "New profile…",
      onClick: () =>
        void run(async () => {
          const n = (await askText("Name for the new profile:"))?.trim();
          if (n) await create(n);
        }),
    },
    {
      label: "Duplicate…",
      onClick: () =>
        void run(async () => {
          const n = (await askText("Name for the copy:", `${profiles.active} copy`))?.trim();
          if (n) await create(n, activeProfile());
        }),
    },
    {
      label: "Rename…",
      onClick: () =>
        void run(async () => {
          const n = (await askText("New name:", profiles.active))?.trim();
          if (n && n !== profiles.active) await rename(profiles.active, n);
        }),
    },
    {
      label: "Delete",
      disabled: profiles.profiles.length <= 1,
      onClick: () =>
        void run(async () => {
          if (await askConfirm(`Delete profile "${profiles.active}"?`, "Delete")) await remove(profiles.active);
        }),
    },
    { separator: true, label: "" },
    {
      label: "Create desktop shortcut",
      onClick: () =>
        void run(async () => {
          await showMessage(`Shortcut created:\n${await api.createProfileShortcut(profiles.active)}`);
        }),
    },
  ];

  return (
    <div className="flex items-center gap-1.5 text-[12px] min-w-0">
      <span className="text-textMuted">Profile</span>
      <select
        value={profiles.active}
        onChange={(e) => void setActive(e.target.value)}
        disabled={gameRunning}
        className="min-w-0 flex-1 max-w-72"
        title="Active profile (enabled mods + load order)"
      >
        {profiles.profiles.map((p) => (
          <option key={p.name} value={p.name}>
            {p.name} ({p.entries.filter((e) => e.enabled && !isSeparator(e)).length})
          </option>
        ))}
      </select>
      <button
        className="btn px-2"
        title="Profile actions"
        onClick={(e) => {
          const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
          setMenu({ x: r.left, y: r.bottom + 2 });
        }}
      >
        ⋯
      </button>
      {menu && <ContextMenu x={menu.x} y={menu.y} items={items} onClose={() => setMenu(null)} />}
    </div>
  );
}
