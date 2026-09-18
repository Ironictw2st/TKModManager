import { useState } from "react";
import { api } from "../ipc/commands";
import { useStore } from "../state/store";

const PHASE_STYLE: Record<string, string> = {
  idle: "text-textMuted",
  writing: "text-textMuted",
  spawned: "text-warn",
  menu: "text-warn",
  injected: "text-warn",
  verified: "text-ok",
  mismatch: "text-danger",
  failed: "text-danger",
  exited: "text-textMuted",
};

/** Play button, per-profile launch options and live launch status. */
export default function LaunchBar() {
  const activeProfile = useStore((s) => s.activeProfile);
  const profiles = useStore((s) => s.profiles);
  const updateProfile = useStore((s) => s.updateProfile);
  const launch = useStore((s) => s.launch);
  const setLaunch = useStore((s) => s.setLaunch);
  const gameRunning = useStore((s) => s.gameRunning);
  const dll = useStore((s) => s.dll);
  const paths = useStore((s) => s.paths);
  const setPanel = useStore((s) => s.setPanel);
  const [busy, setBusy] = useState(false);
  const profile = activeProfile();
  const enabledCount = profile.entries.filter((e) => e.enabled).length;
  const dllOk = !!dll?.selected;

  const play = async () => {
    setBusy(true);
    try {
      setLaunch({ phase: "writing", message: "Starting…", pid: null });
      await api.launchGame(profile);
    } catch (e) {
      setLaunch({ phase: "failed", message: String(e), pid: null });
    } finally {
      setBusy(false);
    }
  };

  return (
    <footer className="flex items-center gap-3 px-3 py-2 border-t border-edge bg-panelHeader text-[12px]">
      <button
        className="px-5 py-1.5 rounded bg-accent/30 hover:bg-accent/45 border border-accent font-semibold text-[13px] disabled:opacity-50"
        onClick={play}
        disabled={busy || gameRunning || !paths?.gameRoot}
        title={gameRunning ? "The game is already running" : `Launch with ${enabledCount} mods`}
      >
        {gameRunning ? "Running…" : "▶ Play"}
      </button>
      <span className="text-textMuted">
        {profiles.active} · {enabledCount} enabled
      </span>
      <label className="flex items-center gap-1.5 cursor-pointer" title="Inject the script extender DLL once the main menu is up">
        <input
          type="checkbox"
          checked={profile.dll}
          disabled={gameRunning}
          onChange={(e) => void updateProfile(profile.name, (p) => ({ ...p, dll: e.target.checked }))}
        />
        Script extender
        {profile.dll && (
          <button
            className={`badge ${dllOk ? "border-ok text-ok" : "border-danger text-danger"}`}
            onClick={() => setPanel("settings")}
            title={dllOk ? `DLL ${dll?.selected?.version} matches this game build` : "No DLL matching this game build is installed"}
          >
            {dllOk ? `v${dll?.selected?.version}` : "not available"}
          </button>
        )}
      </label>
      <label className="flex items-center gap-1.5 cursor-pointer" title="Skip the two startup videos (generated options pack)">
        <input
          type="checkbox"
          checked={profile.skipIntro}
          disabled={gameRunning}
          onChange={(e) => void updateProfile(profile.name, (p) => ({ ...p, skipIntro: e.target.checked }))}
        />
        Skip intro
      </label>
      <div className="flex-1" />
      <span className={PHASE_STYLE[launch.phase] ?? ""} title={launch.pid ? `pid ${launch.pid}` : ""}>
        {launch.message}
      </span>
    </footer>
  );
}
