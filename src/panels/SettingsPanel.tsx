import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { api, type InstalledDll, type RemoteDll } from "../ipc/commands";
import { useStore } from "../state/store";
import { checkForUpdateVerbose, installAndRelaunch, type CheckResult } from "../updater";
import Markdown from "../components/Markdown";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded border border-edge bg-panel p-3 space-y-2">
      <div className="font-semibold text-[13px]">{title}</div>
      {children}
    </section>
  );
}

export default function SettingsPanel() {
  return (
    <div className="flex-1 min-w-0 overflow-auto p-3 grid gap-3 md:grid-cols-2 text-[12px] content-start">
      <GamePaths />
      <Appearance />
      <ScriptExtender />
      <ProfilesTools />
      <ModListPreview />
      <AppUpdates />
    </div>
  );
}

function GamePaths() {
  const settings = useStore((s) => s.settings);
  const paths = useStore((s) => s.paths);
  const setSettings = useStore((s) => s.setSettings);
  const pick = async (key: "gameRoot" | "workshopDir") => {
    const dir = await open({ directory: true, multiple: false, title: key === "gameRoot" ? "Select the Total War THREE KINGDOMS folder" : "Select the Workshop content folder (779340)" });
    if (typeof dir === "string") await setSettings({ [key]: dir });
  };
  return (
    <Section title="Game">
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
      </Field>
      <div className="text-textMuted">
        The mod list is written to <code>tkmm_mods.txt</code> in the game folder; CA's <code>used_mods.txt</code> is only read (Import).
      </div>
    </Section>
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

function Appearance() {
  const settings = useStore((s) => s.settings);
  const setSettings = useStore((s) => s.setSettings);
  return (
    <Section title="Appearance & updates">
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
  const [remote, setRemote] = useState<RemoteDll | null>(null);
  const [remoteErr, setRemoteErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [log, setLog] = useState<{ version: string; text: string } | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const check = async () => {
    setRemoteErr(null);
    setBusy(true);
    try {
      setRemote(await api.dllCheckUpdate());
    } catch (e) {
      setRemoteErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (settings.checkDllUpdates) void check();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
    const version = prompt("Version of this DLL (semver, e.g. 0.15.0):", "0.15.0");
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
    if (!confirm(`Remove script extender ${d.version}?`)) return;
    try {
      await api.dllRemove(d.version);
      await refreshDll();
    } catch (e) {
      setMsg(String(e));
    }
  };

  const showLog = async (d: InstalledDll) => {
    try {
      setLog({ version: d.version, text: await api.dllReadLog(d.dir) });
    } catch (e) {
      setLog({ version: d.version, text: String(e) });
    }
  };

  return (
    <Section title="Script extender (DLL)">
      <div className="text-textMuted">
        Game build: {dll?.gameTimestampHex ?? "?"} / {dll?.gameSizeHex ?? "?"}. A DLL is only injected when its manifest matches these values;
        after a game update the DLL must be rebuilt for the new build.
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
                  <button className="btn" onClick={() => void showLog(d)}>
                    Log
                  </button>
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
        <button className="btn" onClick={importLocal} disabled={busy}>
          Import local DLL…
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
      {msg && <div className="text-textMuted">{msg}</div>}
      {log && (
        <div>
          <div className="flex items-center gap-2">
            <span className="text-textMuted">script_extender.log (v{log.version})</span>
            <button className="btn" onClick={() => setLog(null)}>
              Close
            </button>
          </div>
          <textarea readOnly value={log.text} rows={10} className="w-full font-mono text-[11px] select-text" />
        </div>
      )}
    </Section>
  );
}

function ProfilesTools() {
  const paths = useStore((s) => s.paths);
  const profiles = useStore((s) => s.profiles);
  const activeProfile = useStore((s) => s.activeProfile);
  const createProfile = useStore((s) => s.createProfile);
  const updateProfile = useStore((s) => s.updateProfile);
  const modsByKey = useStore((s) => s.modsByKey);
  const [text, setText] = useState("");
  const [mode, setMode] = useState<"export" | "import" | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  const importCa = async () => {
    let file = paths?.usedModsFile ?? null;
    if (!file) {
      const picked = await open({ multiple: false, filters: [{ name: "Mod list", extensions: ["txt"] }], title: "Select used_mods.txt" });
      if (typeof picked !== "string") return;
      file = picked;
    }
    const name = prompt("Name for the imported profile:", "CA launcher");
    if (!name) return;
    try {
      const imported = await api.importModList(file);
      const keys = imported.map((i) => i.key).filter((k): k is string => !!k);
      const unmatched = imported.filter((i) => !i.key).map((i) => i.file);
      await createProfile(name);
      await updateProfile(name, (p) => {
        const rest = p.entries.filter((e) => !keys.includes(e.key));
        return { ...p, entries: [...keys.map((key) => ({ key, enabled: true })), ...rest] };
      });
      setMsg(`Imported ${keys.length} mods into "${name}"${unmatched.length ? `; not installed: ${unmatched.join(", ")}` : ""}`);
    } catch (e) {
      setMsg(String(e));
    }
  };

  const exportText = () => {
    const p = activeProfile();
    const lines = [`# TK Mod Manager profile "${p.name}" ${new Date().toISOString().slice(0, 10)}`];
    for (const e of p.entries.filter((x) => x.enabled)) {
      const m = modsByKey[e.key];
      lines.push(`${e.key}${m ? `\t# ${m.file}` : ""}`);
    }
    setText(lines.join("\n"));
    setMode("export");
  };

  const importText = async () => {
    const keys = text
      .split(/\r?\n/)
      .map((l) => l.split("\t#")[0].trim())
      .filter((l) => l && !l.startsWith("#"));
    if (!keys.length) return setMsg("Nothing to import");
    const name = prompt("Name for the imported profile:", "Imported");
    if (!name) return;
    const known = keys.filter((k) => modsByKey[k]);
    const unknown = keys.filter((k) => !modsByKey[k]);
    try {
      await createProfile(name);
      await updateProfile(name, (p) => {
        const rest = p.entries.filter((e) => !keys.includes(e.key));
        return { ...p, entries: [...keys.map((key) => ({ key, enabled: true })), ...rest] };
      });
      setMsg(`Imported ${known.length} mods into "${name}"${unknown.length ? `; ${unknown.length} not installed (kept as missing)` : ""}`);
      setMode(null);
    } catch (e) {
      setMsg(String(e));
    }
  };

  return (
    <Section title="Profiles">
      <div className="flex items-center gap-2 flex-wrap">
        <button className="btn" onClick={importCa} title={paths?.usedModsFile ?? "Pick a used_mods.txt"}>
          Import CA launcher list…
        </button>
        <button className="btn" onClick={exportText}>
          Export "{profiles.active}" as text
        </button>
        <button
          className="btn"
          onClick={() => {
            setText("");
            setMode("import");
          }}
        >
          Import from text…
        </button>
      </div>
      {mode && (
        <div className="space-y-1">
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            readOnly={mode === "export"}
            rows={8}
            className="w-full font-mono text-[11px] select-text"
            placeholder="Paste a profile export here (one key per line)"
          />
          <div className="flex gap-2">
            {mode === "export" ? (
              <button className="btn" onClick={() => void navigator.clipboard?.writeText(text)}>
                Copy to clipboard
              </button>
            ) : (
              <button className="btn-accent" onClick={importText}>
                Create profile
              </button>
            )}
            <button className="btn" onClick={() => setMode(null)}>
              Close
            </button>
          </div>
        </div>
      )}
      {msg && <div className="text-textMuted">{msg}</div>}
    </Section>
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
      <textarea readOnly value={text} rows={10} className="w-full font-mono text-[11px] select-text" />
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
