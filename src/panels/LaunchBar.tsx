import { useEffect, useState } from "react";
import { api, type SaveGame } from "../ipc/commands";
import { useStore } from "../state/store";
import { formatDate } from "../util/format";

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

/** Play button, per-profile launch options, optional save to load, and live launch status. */
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
  const [saves, setSaves] = useState<SaveGame[]>([]);
  const [loadSave, setLoadSave] = useState<string>("");
  const profile = activeProfile();
  const enabledCount = profile.entries.filter((e) => e.enabled).length;
  const dllOk = !!dll?.selected;

  useEffect(() => {
    if (gameRunning) return;
    api.listSaves().then(setSaves).catch(() => setSaves([]));
  }, [gameRunning]);

  const play = async () => {
    setBusy(true);
    try {
      setLaunch({ phase: "writing", message: "Starting…", pid: null });
      await api.launchGame(profile, loadSave || null);
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
      <span className="text-textMuted whitespace-nowrap">
        {profiles.active} · {enabledCount} enabled
      </span>
      <label className="flex items-center gap-1.5 cursor-pointer whitespace-nowrap" title="Inject the script extender DLL once the main menu is up">
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
      <label className="flex items-center gap-1.5 cursor-pointer whitespace-nowrap" title="Skip the two startup videos (generated options pack)">
        <input
          type="checkbox"
          checked={profile.skipIntro}
          disabled={gameRunning}
          onChange={(e) => void updateProfile(profile.name, (p) => ({ ...p, skipIntro: e.target.checked }))}
        />
        Skip intro
      </label>
      {saves.length > 0 && (
        <label className="flex items-center gap-1.5 whitespace-nowrap" title="Load this campaign save straight away (game_startup_mode campaign_load)">
          <span className="text-textMuted">Load save</span>
          <select value={loadSave} onChange={(e) => setLoadSave(e.target.value)} disabled={gameRunning} className="max-w-56">
            <option value="">(main menu)</option>
            {saves.slice(0, 40).map((s) => (
              <option key={s.name} value={s.name}>
                {s.name} · {formatDate(s.mtime)}
              </option>
            ))}
          </select>
        </label>
      )}
      <div className="flex-1" />
      <span className={`truncate ${PHASE_STYLE[launch.phase] ?? ""}`} title={launch.pid ? `pid ${launch.pid}` : launch.message}>
        {launch.message}
      </span>
    </footer>
  );
}
