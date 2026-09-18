import { useEffect } from "react";
import { DialogHost } from "./components/Dialogs";
import { listen } from "@tauri-apps/api/event";
import { useStore, type Panel } from "./state/store";
import { applyTheme } from "./theme";
import { workshopApi } from "./ipc/workshop";
import { api, type LaunchStatus, type StartupArgs } from "./ipc/commands";
import ProfileBar from "./panels/ProfileBar";
import ModList from "./panels/ModList";
import ModDetails from "./panels/ModDetails";
import LaunchPanel from "./panels/LaunchPanel";
import SettingsPanel from "./panels/SettingsPanel";
import ConflictsPanel from "./panels/ConflictsPanel";
import LogsPanel from "./panels/LogsPanel";
import CrashDialog from "./panels/CrashDialog";
import UpdateBanner from "./panels/UpdateBanner";

/** Apply command-line style arguments (from this process, a second instance, or a shortcut). */
async function applyArgs(args: StartupArgs | null) {
  if (!args) return;
  const s = useStore.getState();
  if (args.launch) await s.launchProfile(args.profile);
  else if (args.profile && s.profiles.profiles.some((p) => p.name === args.profile)) await s.setActiveProfile(args.profile);
}

const TABS: { id: Exclude<Panel, "settings">; label: string }[] = [
  { id: "mods", label: "Mods" },
  { id: "conflicts", label: "Conflicts" },
  { id: "logs", label: "Logs" },
];

export default function App() {
  const ready = useStore((s) => s.ready);
  const error = useStore((s) => s.error);
  const settings = useStore((s) => s.settings);
  const panel = useStore((s) => s.panel);
  const setPanel = useStore((s) => s.setPanel);
  const paths = useStore((s) => s.paths);
  const mods = useStore((s) => s.mods);
  const gameRunning = useStore((s) => s.gameRunning);

  useEffect(() => {
    const unLaunch = listen<LaunchStatus>("launch-status", (e) => useStore.getState().setLaunch(e.payload));
    const unCli = listen<StartupArgs>("cli-args", (e) => void applyArgs(e.payload));
    const unTray = listen<string | null>("tray-play", (e) => void useStore.getState().launchProfile(e.payload));
    void useStore
      .getState()
      .init()
      .then(() => api.startupArgs())
      .then(applyArgs)
      .catch((e) => console.warn("startup args", e));
    return () => {
      for (const un of [unLaunch, unCli, unTray]) void un.then((f) => f());
    };
  }, []);

  useEffect(() => applyTheme(settings), [settings.themeMode, settings.accent]);

  // The launch thread reports exits for games it follows; poll as a safety net so the Play
  // button always comes back once the game closes.
  useEffect(() => {
    if (!ready) return;
    const id = window.setInterval(async () => {
      const running = await api.gameRunning().catch(() => gameRunning);
      if (running !== useStore.getState().gameRunning) {
        const cur = useStore.getState().launch;
        useStore.setState({ gameRunning: running, launch: running || cur.phase === "crashed" ? cur : { phase: "idle", message: "", pid: null } });
      }
    }, 5000);
    return () => window.clearInterval(id);
  }, [ready, gameRunning]);

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

  return (
    <div className="h-full flex">
      {/* Left: header (tabs · profile · settings) over the main content. */}
      <div className="flex-1 min-w-0 flex flex-col">
        <header className="flex items-center gap-3 px-3 py-2 border-b border-edge bg-panelHeader">
          <div className="font-semibold text-[14px] tracking-wide whitespace-nowrap">
            TK <span className="text-accent">Mod Manager</span>
          </div>
          <nav className="flex gap-1">
            {TABS.map((t) => (
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
          <div className="flex-1 min-w-0 flex justify-center">
            <ProfileBar />
          </div>
          <button
            onClick={() => setPanel(panel === "settings" ? "mods" : "settings")}
            className={`px-2.5 py-1 rounded text-[12px] whitespace-nowrap ${
              panel === "settings" ? "bg-accent/25 border border-accent text-text" : "border border-edge text-textMuted hover:text-text hover:bg-hover"
            }`}
            title={panel === "settings" ? "Back to the mod list" : "Settings"}
          >
            ⚙ Settings
          </button>
        </header>

        {error && <div className="px-3 py-1 text-[12px] bg-danger/20 border-b border-danger">{error}</div>}
        {!paths?.gameRoot && (
          <div className="px-3 py-1 text-[12px] bg-warn/20 border-b border-warn">
            Three Kingdoms was not found through Steam. Set the game folder in Settings.
          </div>
        )}

        <main className="flex-1 min-h-0 flex">
          {panel === "mods" && <ModList />}
          {panel === "conflicts" && <ConflictsPanel />}
          {panel === "logs" && <LogsPanel />}
          {panel === "settings" && <SettingsPanel />}
        </main>
      </div>

      {/* Right: mod details above the launch controls, full height. */}
      <aside className="w-80 shrink-0 flex flex-col border-l border-edge bg-panel">
        <div className="flex-1 min-h-0 overflow-auto">
          <ModDetails />
        </div>
        <LaunchPanel />
      </aside>

      <CrashDialog />
      <DialogHost />
      <UpdateBanner enabled={settings.checkAppUpdates} />
    </div>
  );
}
