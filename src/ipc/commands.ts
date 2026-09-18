// Typed wrappers around every Rust command (src-tauri/src/*.rs). Field names are camelCase on
// both sides (serde rename_all), so these types mirror the Rust structs one-to-one.

import { invoke } from "@tauri-apps/api/core";
import type { WorkshopItem } from "./workshop";
import type { SeInfo } from "../util/status";

export type PackType = "boot" | "release" | "patch" | "mod" | "movie" | "unknown";
export type ModSource = "workshop" | "data" | "folder";

export interface ModEntry {
  key: string;
  file: string;
  path: string;
  dir: string;
  source: ModSource;
  workshopId: string | null;
  packType: PackType;
  size: number;
  mtime: number;
  previewPath: string | null;
  /** Workshop only: publish time of the installed version (Steam's workshop manifest). */
  installedUpdated: number | null;
  /** Workshop only: newest publish time Steam knows of. */
  latestUpdated: number | null;
}

export interface GamePaths {
  gameRoot: string | null;
  exe: string | null;
  dataDir: string | null;
  workshopDir: string | null;
  usedModsFile: string | null;
  exeVersion: string | null;
}

export interface Settings {
  gameRoot: string | null;
  workshopDir: string | null;
  themeMode: "dark" | "light" | "system";
  accent: string;
  checkAppUpdates: boolean;
  checkDllUpdates: boolean;
  steamApiKey: string;
  workshopCacheHours: number;
  extraModDirs: string[];
  autoInjectExternal: boolean;
  dllChannel: "stable" | "prerelease";
  minimizeToTray: boolean;
  /** Unix seconds; null = the game exe's build date. */
  outdatedBefore: number | null;
}

/** A pack entry, or a separator (`key` starts with "sep:") that heads a group. */
export interface ProfileEntry {
  key: string;
  enabled: boolean;
  label?: string;
  collapsed?: boolean;
}

export interface Profile {
  name: string;
  entries: ProfileEntry[];
  dll: boolean;
  skipIntro: boolean;
  lastPlayed?: number;
}

export interface ProfilesDoc {
  schema: number;
  active: string;
  profiles: Profile[];
}

export interface ModMeta {
  tags: string[];
  notes: string;
  hidden: boolean;
  /** Manual script-extender requirement; undefined/null = automatic. */
  seOverride?: boolean | null;
}

export interface MetaDoc {
  mods: Record<string, ModMeta>;
}

export interface ParsedList {
  dirs: string[];
  mods: string[];
  excludes: string[];
}

export interface ImportedEntry {
  file: string;
  key: string | null;
}

export interface ExeFingerprint {
  timestamp: number;
  sizeOfImage: number;
}

export interface DllManifest {
  version: string;
  game_exe_version: string;
  exe_timestamp: string;
  exe_size_of_image: string;
  sha256: string;
}

export interface InstalledDll {
  version: string;
  path: string;
  dir: string;
  manifest: DllManifest | null;
  compatible: boolean;
}

export interface DllStatus {
  gameFingerprint: ExeFingerprint | null;
  gameTimestampHex: string | null;
  gameSizeHex: string | null;
  installed: InstalledDll[];
  selected: InstalledDll | null;
}

export interface RemoteDll {
  version: string;
  notes: string;
  dllUrl: string;
  manifestUrl: string;
  installed: boolean;
}

export interface DllConfig {
  buildNumber: string;
  buildNumberShort: string;
  buildModified: boolean | null;
}

export interface SaveGame {
  name: string;
  mtime: number;
}

export interface LaunchStatus {
  phase: "idle" | "writing" | "spawned" | "menu" | "injected" | "verified" | "mismatch" | "failed" | "exited" | "crashed";
  message: string;
  pid: number | null;
  exitCode?: number | null;
  historyId?: number | null;
}

export interface PackSnap {
  key: string;
  file: string;
  size: number;
  mtime: number;
}

export interface LaunchRecord {
  id: number;
  profile: string;
  started: number;
  ended: number | null;
  exitCode: number | null;
  packs: PackSnap[];
}

export interface LaunchDiff {
  added: string[];
  removed: string[];
  changed: string[];
  reordered: boolean;
}

export interface CrashReport {
  record: LaunchRecord;
  baseline: LaunchRecord | null;
  diff: LaunchDiff | null;
}

export interface LogSource {
  id: string;
  label: string;
  path: string;
  size: number;
  mtime: number;
}

export interface PackHash {
  key: string;
  file: string;
  size: number;
  sha256: string;
}

export interface StartupArgs {
  profile: string | null;
  launch: boolean;
  minimized: boolean;
}

export interface CollectionResult {
  id: string;
  title: string;
  children: string[];
  items: Record<string, WorkshopItem>;
}

export const api = {
  appVersion: () => invoke<string>("app_version"),
  appDataDir: () => invoke<string>("app_data_dir"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  getPaths: () => invoke<GamePaths>("get_paths"),
  scanMods: () => invoke<ModEntry[]>("scan_mods"),
  loadProfiles: () => invoke<ProfilesDoc>("load_profiles"),
  saveProfiles: (doc: ProfilesDoc) => invoke<void>("save_profiles", { doc }),
  loadMeta: () => invoke<MetaDoc>("load_meta"),
  saveMeta: (doc: MetaDoc) => invoke<void>("save_meta", { doc }),
  readModListFile: (path: string) => invoke<ParsedList>("read_mod_list_file", { path }),
  importModList: (path: string) => invoke<ImportedEntry[]>("import_mod_list", { path }),
  previewModList: (profile: Profile) => invoke<string>("preview_mod_list", { profile }),
  gameRunning: () => invoke<boolean>("game_running"),
  launchGame: (profile: Profile, loadSave: string | null) => invoke<number>("launch_game", { profile, loadSave }),
  listSaves: () => invoke<SaveGame[]>("list_saves"),
  launchHistory: () => invoke<LaunchRecord[]>("launch_history"),
  crashReport: (id: number | null) => invoke<CrashReport | null>("crash_report", { id }),
  logSources: () => invoke<LogSource[]>("log_sources"),
  logTail: (path: string, maxBytes?: number) => invoke<string>("log_tail", { path, maxBytes }),
  hashPacks: (keys: string[]) => invoke<PackHash[]>("hash_packs", { keys }),
  startupArgs: () => invoke<StartupArgs | null>("startup_args"),
  createProfileShortcut: (profile: string) => invoke<string>("create_profile_shortcut", { profile }),
  dllStatus: () => invoke<DllStatus>("dll_status"),
  dllCheckUpdate: () => invoke<RemoteDll>("dll_check_update"),
  dllInstall: (r: RemoteDll) =>
    invoke<InstalledDll>("dll_install", { version: r.version, dllUrl: r.dllUrl, manifestUrl: r.manifestUrl }),
  dllImportLocal: (path: string, version: string) => invoke<InstalledDll>("dll_import_local", { path, version }),
  dllRemove: (version: string) => invoke<void>("dll_remove", { version }),
  dllReadLog: (dir: string) => invoke<string>("dll_read_log", { dir }),
  dllReadCfg: () => invoke<DllConfig>("dll_read_cfg"),
  dllWriteCfg: (config: DllConfig) => invoke<void>("dll_write_cfg", { config }),
  seRequirements: (keys: string[]) => invoke<Record<string, SeInfo>>("se_requirements", { keys }),
  workshopCollection: (input: string) => invoke<CollectionResult>("workshop_collection", { input }),
};
