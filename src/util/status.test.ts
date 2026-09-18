import { describe, expect, it } from "vitest";
import type { DllStatus, ModEntry, Profile } from "../ipc/commands";
import type { WorkshopItem } from "../ipc/workshop";
import { modStatus, seProblems, seRequirement, versionLess, type SeRequirement } from "./status";

const ws = (timeUpdated: number, fromLauncherCache = false): WorkshopItem => ({
  id: "1",
  title: "T",
  description: "",
  timeUpdated,
  timeCreated: 0,
  tags: [],
  previewUrl: "",
  requiredItems: [],
  fetchedAt: 0,
  fromLauncherCache,
});

const mod = (over: Partial<ModEntry> = {}): ModEntry => ({
  key: "ws:1/a.pack",
  file: "a.pack",
  path: "a.pack",
  dir: "",
  source: "workshop",
  workshopId: "1",
  packType: "mod",
  size: 0,
  mtime: 0,
  previewPath: null,
  installedUpdated: 1000,
  latestUpdated: 1000,
  ...over,
});

describe("modStatus", () => {
  const cutoff = 500;
  it("flags a pending update from Steam's manifest or from the API", () => {
    expect(modStatus(mod({ latestUpdated: 1500 }), undefined, cutoff).kind).toBe("pending");
    expect(modStatus(mod(), ws(2000), cutoff).kind).toBe("pending");
    // launcher-cache seeds carry no real update time
    expect(modStatus(mod(), ws(2000, true), cutoff).kind).toBe("ok");
  });
  it("separates old from up to date by the cutoff", () => {
    expect(modStatus(mod({ installedUpdated: 400, latestUpdated: 400 }), undefined, cutoff).kind).toBe("old");
    expect(modStatus(mod(), undefined, cutoff).kind).toBe("ok");
    expect(modStatus(mod({ installedUpdated: 400, latestUpdated: 400 }), undefined, 0).kind).toBe("ok");
  });
  it("handles missing data and non-Workshop packs", () => {
    expect(modStatus(mod({ installedUpdated: null, latestUpdated: null }), undefined, cutoff).kind).toBe("unknown");
    expect(modStatus(mod({ source: "data", workshopId: null }), undefined, cutoff).kind).toBe("local");
    expect(modStatus(mod({ source: "folder", workshopId: null }), undefined, cutoff).kind).toBe("local");
    expect(modStatus(undefined, undefined, cutoff).kind).toBe("unknown");
  });
});

describe("seRequirement", () => {
  const lua = { required: true, source: "lua", detail: "script/x.lua", minVersion: null };
  it("manual override wins over the scan", () => {
    expect(seRequirement(lua, { tags: [], notes: "", hidden: false, seOverride: false }).required).toBe(false);
    expect(seRequirement(undefined, { tags: [], notes: "", hidden: false, seOverride: true }).source).toBe("manual");
  });
  it("uses the scan otherwise", () => {
    expect(seRequirement(lua, undefined)).toMatchObject({ required: true, source: "lua" });
    expect(seRequirement(undefined, undefined).required).toBe(false);
  });
});

describe("seProblems", () => {
  const profile = (dll: boolean): Profile => ({
    name: "p",
    dll,
    skipIntro: false,
    entries: [
      { key: "a", enabled: true },
      { key: "b", enabled: true },
      { key: "c", enabled: false },
    ],
  });
  const req = (k: string): SeRequirement =>
    k === "a" ? { required: true, source: "marker", detail: "", minVersion: "0.24.0" } : k === "c" ? { required: true, source: "lua", detail: "", minVersion: null } : { required: false, source: "", detail: "", minVersion: null };
  const dll = (version: string | null): DllStatus => ({
    gameFingerprint: null,
    gameTimestampHex: null,
    gameSizeHex: null,
    installed: [],
    selected: version ? { version, path: "", dir: "", manifest: null, compatible: true } : null,
  });
  const title = (k: string) => k.toUpperCase();

  it("reports SE off, missing DLL, and too-old DLL; ignores disabled mods", () => {
    expect(seProblems(profile(false), req, title, dll("0.24.0"))).toEqual([{ kind: "off", mods: ["A"] }]);
    expect(seProblems(profile(true), req, title, dll(null))).toEqual([{ kind: "no-dll", mods: ["A"] }]);
    expect(seProblems(profile(true), req, title, dll("0.23.3"))).toEqual([{ kind: "too-old", mods: ["A"], need: "0.24.0", have: "0.23.3" }]);
    expect(seProblems(profile(true), req, title, dll("0.24.1"))).toEqual([]);
  });

  it("compares versions numerically", () => {
    expect(versionLess("0.9.0", "0.23.0")).toBe(true);
    expect(versionLess("0.23.0", "0.23.0")).toBe(false);
    expect(versionLess("1.0", "0.99.9")).toBe(false);
  });
});
