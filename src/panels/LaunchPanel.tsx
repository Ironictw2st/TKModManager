import { useEffect, useMemo, useState } from "react";
import { api, type SaveGame } from "../ipc/commands";
import { useStore } from "../state/store";
import { isSeparator } from "../state/separators";
import { formatDate } from "../util/format";
import { seProblems, seRequirement } from "../util/status";

const PHASE_STYLE: Record<string, string> = {
  idle: "text-textMuted",
  writing: "text-textMuted",
  spawned: "text-warn",
  menu: "text-warn",
  injected: "text-warn",
  verified: "text-ok",
  mismatch: "text-danger",
  failed: "text-danger",
  crashed: "text-danger",
  exited: "text-textMuted",
};

/** Bottom of the right column: Play, Script Extender, Skip Intro, Load Save (mockup order),
 *  plus warnings for enabled mods that need the extender and the launch status. */
export default function LaunchPanel() {
  const profile = useStore((s) => s.activeProfile());
  const updateProfile = useStore((s) => s.updateProfile);
  const launch = useStore((s) => s.launch);
  const launchProfile = useStore((s) => s.launchProfile);
  const gameRunning = useStore((s) => s.gameRunning);
  const dll = useStore((s) => s.dll);
  const paths = useStore((s) => s.paths);
  const setPanel = useStore((s) => s.setPanel);
  const loadSave = useStore((s) => s.loadSave);
  const setLoadSave = useStore((s) => s.setLoadSave);
  const crash = useStore((s) => s.crash);
  const setCrashOpen = useStore((s) => s.setCrashOpen);
  const clearCrash = useStore((s) => s.clearCrash);
  const seScan = useStore((s) => s.seScan);
  const meta = useStore((s) => s.meta);
  const modsByKey = useStore((s) => s.modsByKey);
  const workshop = useStore((s) => s.workshop);
  const version = useStore((s) => s.version);
  const [busy, setBusy] = useState(false);
  const [saves, setSaves] = useState<SaveGame[]>([]);
  const enabledCount = profile.entries.filter((e) => e.enabled && !isSeparator(e)).length;

  useEffect(() => {
    if (gameRunning) return;
    api.listSaves().then(setSaves).catch(() => setSaves([]));
  }, [gameRunning]);

  const problems = useMemo(
    () =>
      seProblems(
        profile,
        (k) => seRequirement(seScan[k], meta.mods[k]),
        (k) => {
          const m = modsByKey[k];
          return (m?.workshopId && workshop[m.workshopId]?.title) || m?.file || k;
        },
        dll,
      ),
    [profile, seScan, meta, modsByKey, workshop, dll],
  );

  const play = async () => {
    setBusy(true);
    try {
      await launchProfile(null);
    } finally {
      setBusy(false);
    }
  };
  const turnOnSe = () => void updateProfile(profile.name, (p) => ({ ...p, dll: true }));

  return (
    <div className="border-t border-edge bg-panelHeader p-3 space-y-2 text-[12px]">
      <button
        className="w-full py-2 rounded bg-accent/30 hover:bg-accent/45 border border-accent font-semibold text-[15px] tracking-wide disabled:opacity-50"
        onClick={play}
        disabled={busy || gameRunning || !paths?.gameRoot}
        title={gameRunning ? "The game is already running" : `Launch ${profile.name} with ${enabledCount} mods`}
      >
        {gameRunning ? "Running…" : "▶  Play"}
      </button>
      <div className="text-center text-textMuted">
        {profile.name} · {enabledCount} mods enabled
      </div>

      {problems.map((p) => (
        <div key={p.kind} className={`rounded border px-2 py-1.5 ${p.kind === "off" ? "border-warn bg-warn/10" : "border-danger bg-danger/10"}`}>
          <div className={p.kind === "off" ? "text-warn" : "text-danger"}>
            {p.kind === "off" && `${p.mods.length} enabled mod${p.mods.length > 1 ? "s" : ""} need${p.mods.length > 1 ? "" : "s"} the script extender.`}
            {p.kind !== "off" && p.text}
          </div>
          <div className="text-textMuted truncate" title={p.mods.join("\n")}>
            {p.mods.slice(0, 3).join(", ")}
            {p.mods.length > 3 ? ` +${p.mods.length - 3} more` : ""}
          </div>
          {p.kind === "off" ? (
            <button className="btn mt-1" onClick={turnOnSe} disabled={gameRunning}>
              Turn on script extender
            </button>
          ) : (
            <button className="btn mt-1" onClick={() => setPanel("settings")}>
              Script extender settings
            </button>
          )}
        </div>
      ))}

      <div className="space-y-1.5">
        <label className="flex items-center gap-2 cursor-pointer" title="Inject the script extender DLL once the main menu is up">
          <input type="checkbox" checked={profile.dll} disabled={gameRunning} onChange={(e) => void updateProfile(profile.name, (p) => ({ ...p, dll: e.target.checked }))} />
          <span className="flex-1">Script extender</span>
          <button
            className={`badge ${dll?.selected ? "border-ok text-ok" : "border-danger text-danger"}`}
            onClick={() => setPanel("settings")}
            title={dll?.selected ? `DLL ${dll.selected.version} matches this game build` : "No DLL matching this game build is installed"}
          >
            {dll?.selected ? `v${dll.selected.version}` : "none"}
          </button>
        </label>
        <label className="flex items-center gap-2 cursor-pointer" title="Skip the two startup videos (generated options pack)">
          <input
            type="checkbox"
            checked={profile.skipIntro}
            disabled={gameRunning}
            onChange={(e) => void updateProfile(profile.name, (p) => ({ ...p, skipIntro: e.target.checked }))}
          />
          <span>Skip intro</span>
        </label>
        <label className="flex items-center gap-2" title="Load this campaign save straight away (game_startup_mode campaign_load)">
          <span className="w-16 shrink-0">Load save</span>
          <select value={loadSave} onChange={(e) => setLoadSave(e.target.value)} disabled={gameRunning || saves.length === 0} className="flex-1 min-w-0">
            <option value="">{saves.length ? "(main menu)" : "(no saves)"}</option>
            {saves.slice(0, 40).map((s) => (
              <option key={s.name} value={s.name}>
                {s.name} · {formatDate(s.mtime)}
              </option>
            ))}
          </select>
        </label>
      </div>

      {crash && (
        <div className="rounded border border-danger bg-danger/10 px-2 py-1.5 flex items-center gap-2">
          <span className="text-danger flex-1">Game ended abnormally</span>
          <button className="btn" onClick={() => setCrashOpen(true)}>
            Details
          </button>
          <button className="text-textMuted hover:text-text" onClick={clearCrash} title="Dismiss">
            ✕
          </button>
        </div>
      )}

      {launch.message && (
        <button
          className={`block w-full text-left truncate ${PHASE_STYLE[launch.phase] ?? ""}`}
          title={`${launch.message}${launch.pid ? ` (pid ${launch.pid})` : ""} — open the Logs tab`}
          onClick={() => setPanel("logs")}
        >
          {launch.message}
        </button>
      )}
      <div className="text-[10px] text-textMuted text-center" title={paths?.gameRoot ?? ""}>
        {paths?.gameRoot ? `Three Kingdoms ${paths.exeVersion ?? ""}` : "game not found"} · TK Mod Manager v{version}
      </div>
    </div>
  );
}
