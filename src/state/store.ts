// Single zustand store: everything the UI shows plus the persistence calls. Profiles and
// metadata are owned here and written through the Rust side on every change (small JSON files).

import { create } from "zustand";
import {
  api,
  type DllStatus,
  type GamePaths,
  type LaunchStatus,
  type MetaDoc,
  type ModEntry,
  type ModMeta,
  type Profile,
  type ProfilesDoc,
  type RemoteDll,
  type Settings,
} from "../ipc/commands";
import { comparePackNames } from "../util/format";
import type { WorkshopItem } from "../ipc/workshop";

export type SortKey = "order" | "title" | "file" | "source" | "type" | "size" | "updated";

export interface Filters {
  search: string;
  source: "all" | "workshop" | "data";
  type: "all" | "mod" | "movie";
  enabled: "all" | "enabled" | "disabled";
  tag: string | null;
  showHidden: boolean;
}

export interface AppStore {
  // loaded data
  ready: boolean;
  error: string | null;
  version: string;
  settings: Settings;
  paths: GamePaths | null;
  mods: ModEntry[];
  modsByKey: Record<string, ModEntry>;
  profiles: ProfilesDoc;
  meta: MetaDoc;
  workshop: Record<string, WorkshopItem>;
  dll: DllStatus | null;
  dllRemote: RemoteDll | null;
  dllError: string | null;
  launch: LaunchStatus;
  gameRunning: boolean;

  // ui
  filters: Filters;
  sort: { key: SortKey; dir: 1 | -1 };
  selected: string[];
  focused: string | null;
  panel: "mods" | "conflicts" | "settings";

  // actions
  init(): Promise<void>;
  refreshMods(): Promise<void>;
  refreshDll(): Promise<void>;
  setSettings(patch: Partial<Settings>): Promise<void>;
  setFilters(patch: Partial<Filters>): void;
  setSort(key: SortKey): void;
  setPanel(panel: AppStore["panel"]): void;
  select(keys: string[], focused?: string | null): void;

  activeProfile(): Profile;
  updateProfile(name: string, fn: (p: Profile) => Profile): Promise<void>;
  setActiveProfile(name: string): Promise<void>;
  createProfile(name: string, from?: Profile): Promise<void>;
  renameProfile(oldName: string, newName: string): Promise<void>;
  deleteProfile(name: string): Promise<void>;
  toggleMods(keys: string[], enabled?: boolean): Promise<void>;
  moveMods(keys: string[], toIndex: number): Promise<void>;
  sortProfileAlpha(): Promise<void>;
  enabledToTop(): Promise<void>;

  setMeta(key: string, patch: Partial<ModMeta>): Promise<void>;
  setWorkshop(items: Record<string, WorkshopItem>): void;
  setLaunch(status: LaunchStatus): void;
}

const DEFAULT_FILTERS: Filters = {
  search: "",
  source: "all",
  type: "all",
  enabled: "all",
  tag: null,
  showHidden: false,
};

const EMPTY_PROFILE: Profile = { name: "Default", entries: [], dll: false, skipIntro: false };

/** Make sure every installed pack has an entry (new packs are appended, disabled) and keep
 *  entries for packs that are gone (shown as missing). */
export function reconcile(profile: Profile, mods: ModEntry[]): Profile {
  const have = new Set(profile.entries.map((e) => e.key));
  const missing = mods
    .filter((m) => !have.has(m.key))
    .sort((a, b) => comparePackNames(a.file, b.file))
    .map((m) => ({ key: m.key, enabled: false }));
  if (missing.length === 0) return profile;
  return { ...profile, entries: [...profile.entries, ...missing] };
}

export const useStore = create<AppStore>((set, get) => ({
  ready: false,
  error: null,
  version: __APP_VERSION__,
  settings: {
    gameRoot: null,
    workshopDir: null,
    themeMode: "dark",
    accent: "#c9a227",
    checkAppUpdates: true,
    checkDllUpdates: true,
    steamApiKey: "",
    workshopCacheHours: 24,
  },
  paths: null,
  mods: [],
  modsByKey: {},
  profiles: { schema: 1, active: "Default", profiles: [EMPTY_PROFILE] },
  meta: { mods: {} },
  workshop: {},
  dll: null,
  dllRemote: null,
  dllError: null,
  launch: { phase: "idle", message: "", pid: null },
  gameRunning: false,
  filters: DEFAULT_FILTERS,
  sort: { key: "order", dir: 1 },
  selected: [],
  focused: null,
  panel: "mods",

  async init() {
    try {
      const [settings, profiles, meta] = await Promise.all([api.getSettings(), api.loadProfiles(), api.loadMeta()]);
      set({ settings, profiles, meta });
      await get().refreshMods();
      // First run: seed the Default profile from the CA launcher's list when it exists.
      const s = get();
      const active = s.activeProfile();
      const paths = s.paths;
      if (active.entries.every((e) => !e.enabled) && paths?.usedModsFile && !localStorage.getItem("tkmm.seeded")) {
        try {
          const imported = await api.importModList(paths.usedModsFile);
          const enabledKeys = imported.map((i) => i.key).filter((k): k is string => !!k);
          if (enabledKeys.length) {
            await s.updateProfile(active.name, (p) => {
              const rest = p.entries.filter((e) => !enabledKeys.includes(e.key));
              return { ...p, entries: [...enabledKeys.map((key) => ({ key, enabled: true })), ...rest] };
            });
          }
        } catch (e) {
          console.warn("seed from used_mods.txt failed", e);
        }
        localStorage.setItem("tkmm.seeded", "1");
      }
      set({ ready: true, gameRunning: await api.gameRunning() });
      void get().refreshDll();
    } catch (e) {
      set({ error: String(e), ready: true });
    }
  },

  async refreshMods() {
    const [paths, mods] = await Promise.all([api.getPaths(), api.scanMods()]);
    const modsByKey: Record<string, ModEntry> = {};
    for (const m of mods) modsByKey[m.key] = m;
    // Reconcile every profile so new packs show up in all of them.
    const doc = get().profiles;
    const profiles = doc.profiles.map((p) => reconcile(p, mods));
    const changed = profiles.some((p, i) => p !== doc.profiles[i]);
    const next = changed ? { ...doc, profiles } : doc;
    set({ paths, mods, modsByKey, profiles: next });
    if (changed) await api.saveProfiles(next);
  },

  async refreshDll() {
    try {
      set({ dll: await api.dllStatus() });
    } catch (e) {
      set({ dllError: String(e) });
    }
  },

  async setSettings(patch) {
    const settings = { ...get().settings, ...patch };
    set({ settings });
    await api.setSettings(settings);
    if ("gameRoot" in patch || "workshopDir" in patch) {
      await get().refreshMods();
      await get().refreshDll();
    }
  },

  setFilters(patch) {
    set({ filters: { ...get().filters, ...patch } });
  },
  setSort(key) {
    const cur = get().sort;
    set({ sort: cur.key === key ? { key, dir: cur.dir === 1 ? -1 : 1 } : { key, dir: 1 } });
  },
  setPanel(panel) {
    set({ panel });
  },
  select(keys, focused) {
    set({ selected: keys, focused: focused === undefined ? (keys[keys.length - 1] ?? null) : focused });
  },

  activeProfile() {
    const doc = get().profiles;
    return doc.profiles.find((p) => p.name === doc.active) ?? doc.profiles[0] ?? EMPTY_PROFILE;
  },

  async updateProfile(name, fn) {
    const doc = get().profiles;
    const profiles = doc.profiles.map((p) => (p.name === name ? fn(p) : p));
    const next = { ...doc, profiles };
    set({ profiles: next });
    await api.saveProfiles(next);
  },

  async setActiveProfile(name) {
    const next = { ...get().profiles, active: name };
    set({ profiles: next, selected: [], focused: null });
    await api.saveProfiles(next);
  },

  async createProfile(name, from) {
    const doc = get().profiles;
    if (doc.profiles.some((p) => p.name === name)) throw new Error(`A profile named "${name}" already exists`);
    const base: Profile = from
      ? { ...from, name, entries: from.entries.map((e) => ({ ...e })) }
      : reconcile({ ...EMPTY_PROFILE, name }, get().mods);
    const next = { ...doc, active: name, profiles: [...doc.profiles, base] };
    set({ profiles: next });
    await api.saveProfiles(next);
  },

  async renameProfile(oldName, newName) {
    const doc = get().profiles;
    if (doc.profiles.some((p) => p.name === newName)) throw new Error(`A profile named "${newName}" already exists`);
    const next = {
      ...doc,
      active: doc.active === oldName ? newName : doc.active,
      profiles: doc.profiles.map((p) => (p.name === oldName ? { ...p, name: newName } : p)),
    };
    set({ profiles: next });
    await api.saveProfiles(next);
  },

  async deleteProfile(name) {
    const doc = get().profiles;
    if (doc.profiles.length <= 1) throw new Error("Cannot delete the last profile");
    const profiles = doc.profiles.filter((p) => p.name !== name);
    const next = { ...doc, profiles, active: doc.active === name ? profiles[0].name : doc.active };
    set({ profiles: next });
    await api.saveProfiles(next);
  },

  async toggleMods(keys, enabled) {
    const p = get().activeProfile();
    const ks = new Set(keys);
    await get().updateProfile(p.name, (prof) => ({
      ...prof,
      entries: prof.entries.map((e) => (ks.has(e.key) ? { ...e, enabled: enabled ?? !e.enabled } : e)),
    }));
  },

  /** Move the selected entries as a block so the first lands at `toIndex` (index in the
   *  entries array after removal). */
  async moveMods(keys, toIndex) {
    const p = get().activeProfile();
    const ks = new Set(keys);
    await get().updateProfile(p.name, (prof) => {
      const moving = prof.entries.filter((e) => ks.has(e.key));
      const rest = prof.entries.filter((e) => !ks.has(e.key));
      const at = Math.max(0, Math.min(toIndex, rest.length));
      return { ...prof, entries: [...rest.slice(0, at), ...moving, ...rest.slice(at)] };
    });
  },

  async sortProfileAlpha() {
    const p = get().activeProfile();
    const byKey = get().modsByKey;
    await get().updateProfile(p.name, (prof) => ({
      ...prof,
      entries: [...prof.entries].sort((a, b) =>
        comparePackNames(byKey[a.key]?.file ?? a.key, byKey[b.key]?.file ?? b.key),
      ),
    }));
  },

  async enabledToTop() {
    const p = get().activeProfile();
    await get().updateProfile(p.name, (prof) => ({
      ...prof,
      entries: [...prof.entries.filter((e) => e.enabled), ...prof.entries.filter((e) => !e.enabled)],
    }));
  },

  async setMeta(key, patch) {
    const doc = get().meta;
    const cur = doc.mods[key] ?? { tags: [], notes: "", hidden: false };
    const next = { mods: { ...doc.mods, [key]: { ...cur, ...patch } } };
    set({ meta: next });
    await api.saveMeta(next);
  },

  setWorkshop(items) {
    set({ workshop: { ...get().workshop, ...items } });
  },

  setLaunch(status) {
    set({ launch: status, gameRunning: status.phase !== "exited" && status.phase !== "failed" && status.phase !== "idle" });
  },
}));
