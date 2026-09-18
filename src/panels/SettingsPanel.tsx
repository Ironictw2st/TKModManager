import { useEffect, useState } from "react";
import { askConfirm, askText } from "../components/Dialogs";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { api, type DllConfig, type InstalledDll, type RemoteDll } from "../ipc/commands";
import { useStore } from "../state/store";
import { checkForUpdateVerbose, installAndRelaunch, type CheckResult } from "../updater";
import Markdown from "../components/Markdown";
import ProfileTools from "./ProfileTools";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded border border-edge bg-panel p-3 space-y-2">
      <div className="font-semibold text-[13px]">{title}</div>
      {children}
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2">
      <span className="w-32 shrink-0 text-textMuted">{label}</span>
      {children}
    </div>
  );
}

export default function SettingsPanel() {
  return (
    <div className="flex-1 min-w-0 overflow-auto p-3 grid gap-3 md:grid-cols-2 text-[12px] content-start">
      <GamePaths />
      <Appearance />
      <ScriptExtender />
      <ModListPreview />
      <ProfileTools />
      <AppUpdates />
    </div>
  );
}

function GamePaths() {
  const settings = useStore((s) => s.settings);
  const paths = useStore((s) => s.paths);
  const mods = useStore((s) => s.mods);
  const setSettings = useStore((s) => s.setSettings);
  const refreshMods = useStore((s) => s.refreshMods);
  const refreshSeScan = useStore((s) => s.refreshSeScan);
  const cutoff = useStore((s) => s.outdatedCutoff());
  const pick = async (key: "gameRoot" | "workshopDir") => {
    const dir = await open({
      directory: true,
      multiple: false,
      title: key === "gameRoot" ? "Select the Total War THREE KINGDOMS folder" : "Select the Workshop content folder (779340)",
    });
    if (typeof dir === "string") await setSettings({ [key]: dir });
  };
  const addFolder = async () => {
    const dir = await open({ directory: true, multiple: false, title: "Select a folder that holds .pack files" });
    if (typeof dir === "string" && !settings.extraModDirs.some((d) => d.toLowerCase() === dir.toLowerCase())) {
      await setSettings({ extraModDirs: [...settings.extraModDirs, dir] });
    }
  };
  return (
    <Section title="Game and mod folders">
      <Field label="Game folder">
        <span className="flex-1 truncate select-text" title={paths?.gameRoot ?? ""}>
          {paths?.gameRoot ?? <span className="text-danger">not found</span>}
          {settings.gameRoot ? " (manual)" : " (auto)"}
        </span>
        <button className="btn" onClick={() => void pick("gameRoot")}>
          Change…
        </button>
        {settings.gameRoot && (
          <button className="btn" onClick={() => void setSettings({ gameRoot: null })}>
            Auto
          </button>
        )}
      </Field>
      <Field label="Workshop folder">
        <span className="flex-1 truncate select-text" title={paths?.workshopDir ?? ""}>
          {paths?.workshopDir ?? <span className="text-danger">not found</span>}
        </span>
        <button className="btn" onClick={() => void pick("workshopDir")}>
          Change…
        </button>
        {settings.workshopDir && (
          <button className="btn" onClick={() => void setSettings({ workshopDir: null })}>
            Auto
          </button>
        )}
      </Field>
      <Field label="Game version">
        <span>{paths?.exeVersion ?? "?"}</span>
        {paths?.gameRoot && (
          <button className="btn" onClick={() => void openPath(paths.gameRoot!)}>
            Open folder
          </button>
        )}
        <button
          className="btn"
          onClick={() => void refreshMods().then(() => refreshSeScan())}
          title="Rescan data/, the Workshop and the extra folders, and re-check which mods need the script extender"
        >
          Refresh mods
        </button>
      </Field>
      <Field label="Older than patch">
        <input
          type="date"
          value={cutoff ? new Date(cutoff * 1000).toISOString().slice(0, 10) : ""}
          onChange={(e) => {
            const t = e.target.value ? Math.floor(Date.parse(`${e.target.value}T00:00:00Z`) / 1000) : null;
            void setSettings({ outdatedBefore: t });
          }}
          title="Workshop mods last updated before this date get the amber dot"
        />
        {settings.outdatedBefore !== null ? (
          <button className="btn" onClick={() => void setSettings({ outdatedBefore: null })} title="Use the game exe's build date">
            Reset
          </button>
        ) : (
          <span className="text-textMuted">(game build date)</span>
        )}
      </Field>
      <div>
        <div className="flex items-center gap-2 mb-1">
          <span className="font-semibold">Extra mod folders</span>
          <button className="btn" onClick={addFolder}>
            Add folder…
          </button>
        </div>
        {settings.extraModDirs.length === 0 ? (
          <div className="text-textMuted">
            None. Add a folder (for example your RPFM MyMods folder) and its packs load straight from there; nothing is copied into data/.
          </div>
        ) : (
          <ul className="space-y-0.5">
            {settings.extraModDirs.map((d) => (
              <li key={d} className="flex items-center gap-2">
                <span className="flex-1 truncate select-text" title={d}>
                  {d}
                </span>
                <span className="text-textMuted">{mods.filter((m) => m.source === "folder" && m.dir.toLowerCase() === d.toLowerCase()).length} packs</span>
                <button className="btn" onClick={() => void setSettings({ extraModDirs: settings.extraModDirs.filter((x) => x !== d) })}>
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="text-textMuted">
        The mod list is written to <code>tkmm_mods.txt</code> in the game folder; CA's <code>used_mods.txt</code> is only read (Import).
      </div>
    </Section>
  );
}

function Appearance() {
  const settings = useStore((s) => s.settings);
  const setSettings = useStore((s) => s.setSettings);
  return (
    <Section title="Appearance and behaviour">
      <Field label="Theme">
        <select value={settings.themeMode} onChange={(e) => void setSettings({ themeMode: e.target.value as typeof settings.themeMode })}>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
          <option value="system">System</option>
        </select>
      </Field>
      <Field label="Accent">
        <input type="color" value={settings.accent} onChange={(e) => void setSettings({ accent: e.target.value })} className="w-12 h-6 p-0" />
        <button className="btn" onClick={() => void setSettings({ accent: "#c9a227" })}>
          Reset
        </button>
      </Field>
      <label className="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" checked={settings.minimizeToTray} onChange={(e) => void setSettings({ minimizeToTray: e.target.checked })} />
        Keep running in the tray when the window is closed (tray menu: Play, profiles, Quit)
      </label>
      <label className="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" checked={settings.checkAppUpdates} onChange={(e) => void setSettings({ checkAppUpdates: e.target.checked })} />
        Check for app updates on startup
      </label>
      <label className="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" checked={settings.checkDllUpdates} onChange={(e) => void setSettings({ checkDllUpdates: e.target.checked })} />
        Check for script-extender updates on startup
      </label>
      <Field label="Workshop cache">
        <input
          type="number"
          min={1}
          value={settings.workshopCacheHours}
          onChange={(e) => void setSettings({ workshopCacheHours: Math.max(1, Number(e.target.value) || 24) })}
          className="w-20"
        />
        <span className="text-textMuted">hours before re-fetching titles / dates from Steam</span>
      </Field>
    </Section>
  );
}

function ScriptExtender() {
  const dll = useStore((s) => s.dll);
  const refreshDll = useStore((s) => s.refreshDll);
  const settings = useStore((s) => s.settings);
  const setSettings = useStore((s) => s.setSettings);
  const setPanel = useStore((s) => s.setPanel);
  const [remote, setRemote] = useState<RemoteDll | null>(null);
  const [remoteErr, setRemoteErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const check = async () => {
    setRemoteErr(null);
    setBusy(true);
    try {
      setRemote(await api.dllCheckUpdate());
    } catch (e) {
      setRemote(null);
      setRemoteErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (settings.checkDllUpdates) void check();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.dllChannel]);

  const install = async () => {
    if (!remote) return;
    setBusy(true);
    setProgress(0);
    const un = await listen<number>("dll-progress", (e) => setProgress(e.payload));
    try {
      const d = await api.dllInstall(remote);
      setMsg(`Installed ${d.version}${d.compatible ? " (matches this game build)" : " (built for a different game build)"}`);
      setRemote({ ...remote, installed: true });
      await refreshDll();
    } catch (e) {
      setMsg(String(e));
    } finally {
      un();
      setProgress(null);
      setBusy(false);
    }
  };

  const importLocal = async () => {
    const file = await open({ multiple: false, filters: [{ name: "DLL", extensions: ["dll"] }], title: "Select script_extender.dll" });
    if (typeof file !== "string") return;
    const version = await askText("Version of this DLL (semver, e.g. 0.23.2):", "");
    if (!version) return;
    try {
      const d = await api.dllImportLocal(file, version);
      setMsg(`Imported ${d.version}; manifest pinned to the current game build`);
      await refreshDll();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const remove = async (d: InstalledDll) => {
    if (!(await askConfirm(`Remove script extender ${d.version}?`, "Remove"))) return;
    try {
      await api.dllRemove(d.version);
      await refreshDll();
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <Section title="Script extender (DLL)">
      <div className="text-textMuted">
        Game build: {dll?.gameTimestampHex ?? "?"} / {dll?.gameSizeHex ?? "?"}. A DLL is only injected when its manifest matches these values, and
        never into a game that already has it loaded.
      </div>
      {dll?.installed.length ? (
        <table className="w-full">
          <tbody>
            {dll.installed.map((d) => (
              <tr key={d.version} className="border-t border-edge/50">
                <td className="py-1 pr-2 font-semibold">v{d.version}</td>
                <td className="py-1 pr-2">
                  {d.compatible ? (
                    <span className="badge border-ok text-ok">matches game</span>
                  ) : (
                    <span className="badge border-danger text-danger" title={d.manifest ? `built for ${d.manifest.game_exe_version || d.manifest.exe_timestamp}` : "no manifest"}>
                      other build
                    </span>
                  )}
                  {dll.selected?.version === d.version && <span className="badge border-accent text-accent ml-1">will inject</span>}
                </td>
                <td className="py-1 text-right space-x-1">
                  <button className="btn" onClick={() => void openPath(d.dir)}>
                    Folder
                  </button>
                  <button className="btn" onClick={() => void remove(d)}>
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div className="text-warn">No DLL installed. Check for updates or import a local build.</div>
      )}
      <div className="flex items-center gap-2 flex-wrap">
        <button className="btn" onClick={check} disabled={busy}>
          Check for DLL updates
        </button>
        <select
          value={settings.dllChannel}
          onChange={(e) => void setSettings({ dllChannel: e.target.value as typeof settings.dllChannel })}
          title="Pre-release also offers beta builds (tags like v0.24.0-beta.1)"
        >
          <option value="stable">Stable releases</option>
          <option value="prerelease">Include pre-releases</option>
        </select>
        <button className="btn" onClick={importLocal} disabled={busy}>
          Import local DLL…
        </button>
        <button className="btn" onClick={() => setPanel("logs")}>
          View log
        </button>
        {remote && (
          <>
            <span>
              Latest: <b>v{remote.version}</b> {remote.installed ? "(installed)" : ""}
            </span>
            {!remote.installed && (
              <button className="btn-accent" onClick={install} disabled={busy}>
                Install
              </button>
            )}
          </>
        )}
        {remoteErr && <span className="text-danger">{remoteErr}</span>}
      </div>
      {progress !== null && (
        <div className="h-1.5 rounded bg-button overflow-hidden">
          <div className="h-full bg-accent transition-[width]" style={{ width: `${Math.round(progress * 100)}%` }} />
        </div>
      )}
      {remote?.notes && !remote.installed && (
        <div className="text-textMuted max-h-24 overflow-auto">
          <Markdown text={remote.notes} />
        </div>
      )}
      <label className="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" checked={settings.autoInjectExternal} onChange={(e) => void setSettings({ autoInjectExternal: e.target.checked })} />
        Also inject when the game was started outside the manager (while this app is running)
      </label>
      <DllConfigEditor />
      {msg && <div className="text-textMuted">{msg}</div>}
    </Section>
  );
}

/** `dll\script_extender.cfg`: the menu build-number text the DLL applies at injection. */
function DllConfigEditor() {
  const [cfg, setCfg] = useState<DllConfig>({ buildNumber: "", buildNumberShort: "", buildModified: null });
  const [saved, setSaved] = useState<string | null>(null);
  useEffect(() => {
    api.dllReadCfg().then(setCfg).catch(() => {});
  }, []);
  const save = async () => {
    try {
      await api.dllWriteCfg(cfg);
      setSaved("Saved; applies at the next injection");
    } catch (e) {
      setSaved(String(e));
    }
  };
  return (
    <div className="border-t border-edge/60 pt-2 space-y-1">
      <div className="font-semibold">Main-menu build text (script_extender.cfg)</div>
      <Field label="Build number">
        <input value={cfg.buildNumber} onChange={(e) => setCfg({ ...cfg, buildNumber: e.target.value })} placeholder="leave empty to keep the game's own text" className="flex-1" />
      </Field>
      <Field label="Short form">
        <input value={cfg.buildNumberShort} onChange={(e) => setCfg({ ...cfg, buildNumberShort: e.target.value })} className="flex-1" />
      </Field>
      <Field label={'"Modified" flag'}>
        <select
          value={cfg.buildModified === null ? "" : cfg.buildModified ? "1" : "0"}
          onChange={(e) => setCfg({ ...cfg, buildModified: e.target.value === "" ? null : e.target.value === "1" })}
        >
          <option value="">Leave alone</option>
          <option value="0">Show as unmodified</option>
          <option value="1">Show as modified</option>
        </select>
        <button className="btn" onClick={save}>
          Save
        </button>
        {saved && <span className="text-textMuted">{saved}</span>}
      </Field>
    </div>
  );
}

function ModListPreview() {
  const profile = useStore((s) => s.activeProfile());
  const [text, setText] = useState("");
  useEffect(() => {
    let cancelled = false;
    api.previewModList(profile).then((t) => !cancelled && setText(t)).catch((e) => setText(String(e)));
    return () => {
      cancelled = true;
    };
  }, [profile]);
  return (
    <Section title={`Generated mod list (${profile.name})`}>
      <div className="text-textMuted">Exactly what is written to tkmm_mods.txt at launch (before the optional skip-intro pack).</div>
      <textarea readOnly value={text} rows={12} className="w-full font-mono text-[11px] select-text" />
    </Section>
  );
}

function AppUpdates() {
  const version = useStore((s) => s.version);
  const [result, setResult] = useState<CheckResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [dataDir, setDataDir] = useState("");
  useEffect(() => {
    api.appDataDir().then(setDataDir).catch(() => {});
  }, []);
  const check = async () => {
    setBusy(true);
    setResult(await checkForUpdateVerbose());
    setBusy(false);
  };
  const install = async () => {
    if (result?.status !== "available") return;
    setBusy(true);
    setProgress(0);
    try {
      await installAndRelaunch(result.info, setProgress);
    } catch (e) {
      setResult({ status: "error", message: String(e) });
      setBusy(false);
      setProgress(null);
    }
  };
  return (
    <Section title="About">
      <div>
        TK Mod Manager v{version} · data in{" "}
        <button className="underline" onClick={() => void openPath(dataDir)} title={dataDir}>
          {dataDir || "…"}
        </button>
      </div>
      <div className="text-textMuted">
        Shortcuts: <code>--profile "Name" --launch</code> starts a profile directly; <code>--minimized</code> starts in the tray.
      </div>
      <div className="flex items-center gap-2">
        <button className="btn" onClick={check} disabled={busy}>
          Check for app updates
        </button>
        {result?.status === "current" && <span className="text-ok">Up to date</span>}
        {result?.status === "error" && <span className="text-danger">{result.message}</span>}
        {result?.status === "available" && (
          <>
            <span>v{result.info.version} available</span>
            <button className="btn-accent" onClick={install} disabled={busy}>
              Install &amp; restart
            </button>
          </>
        )}
      </div>
      {progress !== null && (
        <div className="h-1.5 rounded bg-button overflow-hidden">
          <div className="h-full bg-accent transition-[width]" style={{ width: `${Math.round(progress * 100)}%` }} />
        </div>
      )}
      {result?.status === "available" && result.info.notes && (
        <div className="text-textMuted max-h-32 overflow-auto">
          <Markdown text={result.info.notes} />
        </div>
      )}
    </Section>
  );
}
