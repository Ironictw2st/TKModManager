// Typed wrappers around every Rust command (src-tauri/src/*.rs). Field names are camelCase on
// both sides (serde rename_all), so these types mirror the Rust structs one-to-one.

import { invoke } from "@tauri-apps/api/core";

export type PackType = "boot" | "release" | "patch" | "mod" | "movie" | "unknown";
export type ModSource = "workshop" | "data";

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
}

export interface ProfileEntry {
  key: string;
  enabled: boolean;
}

export interface Profile {
  name: string;
  entries: ProfileEntry[];
  dll: boolean;
  skipIntro: boolean;
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

export interface SaveGame {
  name: string;
  mtime: number;
}

export interface LaunchStatus {
  phase: "idle" | "writing" | "spawned" | "menu" | "injected" | "verified" | "mismatch" | "failed" | "exited";
  message: string;
  pid: number | null;
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
  dllStatus: () => invoke<DllStatus>("dll_status"),
  dllCheckUpdate: () => invoke<RemoteDll>("dll_check_update"),
  dllInstall: (r: RemoteDll) =>
    invoke<InstalledDll>("dll_install", { version: r.version, dllUrl: r.dllUrl, manifestUrl: r.manifestUrl }),
  dllImportLocal: (path: string, version: string) => invoke<InstalledDll>("dll_import_local", { path, version }),
  dllRemove: (version: string) => invoke<void>("dll_remove", { version }),
  dllReadLog: (dir: string) => invoke<string>("dll_read_log", { dir }),
};
