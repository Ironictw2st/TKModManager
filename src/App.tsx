import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./state/store";
import { applyTheme } from "./theme";
import { workshopApi } from "./ipc/workshop";
import type { LaunchStatus } from "./ipc/commands";
import ProfileBar from "./panels/ProfileBar";
import ModList from "./panels/ModList";
import ModDetails from "./panels/ModDetails";
import LaunchBar from "./panels/LaunchBar";
import SettingsPanel from "./panels/SettingsPanel";
import ConflictsPanel from "./panels/ConflictsPanel";
import UpdateBanner from "./panels/UpdateBanner";

export default function App() {
  const ready = useStore((s) => s.ready);
  const error = useStore((s) => s.error);
  const settings = useStore((s) => s.settings);
  const panel = useStore((s) => s.panel);
  const setPanel = useStore((s) => s.setPanel);
  const paths = useStore((s) => s.paths);
  const version = useStore((s) => s.version);
  const mods = useStore((s) => s.mods);

  useEffect(() => {
    void useStore.getState().init();
    const un = listen<LaunchStatus>("launch-status", (e) => useStore.getState().setLaunch(e.payload));
    return () => {
      void un.then((f) => f());
    };
  }, []);

  useEffect(() => applyTheme(settings), [settings.themeMode, settings.accent]);

  // Workshop metadata: cached first, then a background refresh for installed items.
  useEffect(() => {
    if (!ready || mods.length === 0) return;
    const ids = Array.from(new Set(mods.map((m) => m.workshopId).filter((x): x is string => !!x)));
    let cancelled = false;
    (async () => {
      try {
        const cached = await workshopApi.cached();
        if (!cancelled) useStore.getState().setWorkshop(cached);
        const fresh = await workshopApi.fetch(ids, false);
        if (!cancelled) useStore.getState().setWorkshop(fresh);
      } catch (e) {
        console.warn("workshop metadata", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [ready, mods]);

  if (!ready) {
    return <div className="h-full flex items-center justify-center text-textMuted">Loading…</div>;
  }

  const tabs: { id: typeof panel; label: string }[] = [
    { id: "mods", label: "Mods" },
    { id: "conflicts", label: "Conflicts" },
    { id: "settings", label: "Settings" },
  ];

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-3 px-3 py-2 border-b border-edge bg-panelHeader">
        <div className="font-semibold text-[14px] tracking-wide">
          TK <span className="text-accent">Mod Manager</span>
        </div>
        <nav className="flex gap-1 ml-2">
          {tabs.map((t) => (
            <button
              key={t.id}
              onClick={() => setPanel(t.id)}
              className={`px-2.5 py-1 rounded text-[12px] ${
                panel === t.id ? "bg-accent/25 border border-accent text-text" : "border border-transparent text-textMuted hover:text-text hover:bg-hover"
              }`}
            >
              {t.label}
            </button>
          ))}
        </nav>
        <div className="flex-1" />
        <ProfileBar />
        <div className="text-[10px] text-textMuted ml-2" title={paths?.gameRoot ?? ""}>
          {paths?.gameRoot ? `game ${paths.exeVersion ?? "found"}` : "game not found"} · v{version}
        </div>
      </header>

      {error && <div className="px-3 py-1 text-[12px] bg-danger/20 border-b border-danger">{error}</div>}
      {!paths?.gameRoot && (
        <div className="px-3 py-1 text-[12px] bg-warn/20 border-b border-warn">
          Three Kingdoms was not found through Steam. Set the game folder in Settings.
        </div>
      )}

      <main className="flex-1 min-h-0 flex">
        {panel === "mods" && (
          <>
            <ModList />
            <ModDetails />
          </>
        )}
        {panel === "conflicts" && <ConflictsPanel />}
        {panel === "settings" && <SettingsPanel />}
      </main>

      <LaunchBar />
      <UpdateBanner enabled={settings.checkAppUpdates} />
    </div>
  );
}
