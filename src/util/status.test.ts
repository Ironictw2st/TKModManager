import { describe, expect, it } from "vitest";
import type { DllStatus, ModEntry, Profile } from "../ipc/commands";
import type { WorkshopItem } from "../ipc/workshop";
import { modStatus, seProblems, seRangeText, seRequirement, seUnmet, versionAboveMax, versionLess, type SeInfo, type SeRequirement } from "./status";

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

const info = (min: string | null, max: string | null): SeInfo => ({
  required: true,
  path: "SE/script_extender.json",
  author: "Ironic",
  minVersion: min,
  maxVersion: max,
  notes: null,
  error: null,
});

describe("seRequirement", () => {
  it("manual override wins over the manifest", () => {
    expect(seRequirement(info("0.28", "0.28"), { tags: [], notes: "", hidden: false, seOverride: false }).required).toBe(false);
    const forced = seRequirement(undefined, { tags: [], notes: "", hidden: false, seOverride: true });
    expect(forced).toMatchObject({ required: true, source: "manual" });
    // forcing keeps the manifest's range
    expect(seRequirement(info("0.28", null), { tags: [], notes: "", hidden: false, seOverride: true }).minVersion).toBe("0.28");
  });
  it("uses the manifest otherwise", () => {
    expect(seRequirement(info("0.28", "0.28"), undefined)).toMatchObject({ required: true, source: "manifest", author: "Ironic" });
    expect(seRequirement(undefined, undefined).required).toBe(false);
  });
  it("describes ranges", () => {
    expect(seRangeText(info("0.28", "0.28"))).toBe("0.28");
    expect(seRangeText(info("0.26", "0.28"))).toBe("0.26 – 0.28");
    expect(seRangeText(info("0.28", null))).toBe("0.28 or newer");
    expect(seRangeText(info(null, "0.28"))).toBe("up to 0.28");
    expect(seRangeText(info(null, null))).toBe("");
  });
});

describe("version checks", () => {
  it("compares numerically, 0.3 below 0.28", () => {
    expect(versionLess("0.3", "0.28")).toBe(true);
    expect(versionLess("0.9.0", "0.23.0")).toBe(true);
    expect(versionLess("0.28.0", "0.28")).toBe(false);
  });
  it("applies the maximum at its own precision", () => {
    expect(versionAboveMax("0.28.0", "0.28")).toBe(false);
    expect(versionAboveMax("0.28.5", "0.28")).toBe(false);
    expect(versionAboveMax("0.29.0", "0.28")).toBe(true);
    expect(versionAboveMax("0.28.6", "0.28.5")).toBe(true);
    expect(versionAboveMax("0.30.0", "0.30")).toBe(false);
  });
  it("seUnmet checks in order off, no DLL, too old, too new", () => {
    const r = seRequirement(info("0.28", "0.28"), undefined);
    expect(seUnmet(r, false, "0.28.0")).toBe("off");
    expect(seUnmet(r, true, null)).toBe("no-dll");
    expect(seUnmet(r, true, "0.27.9")).toBe("too-old");
    expect(seUnmet(r, true, "0.29.0")).toBe("too-new");
    expect(seUnmet(r, true, "0.28.3")).toBeNull();
    expect(seUnmet(seRequirement(undefined, undefined), false, null)).toBeNull();
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
      { key: "d", enabled: true },
    ],
  });
  const reqs: Record<string, SeRequirement> = {
    a: seRequirement(info("0.28", "0.28"), undefined),
    c: seRequirement(info(null, null), undefined),
    d: seRequirement(info(null, "0.24"), undefined),
  };
  const req = (k: string) => reqs[k] ?? seRequirement(undefined, undefined);
  const dll = (version: string | null): DllStatus => ({
    gameFingerprint: null,
    gameTimestampHex: null,
    gameSizeHex: null,
    installed: [],
    selected: version ? { version, path: "", dir: "", manifest: null, compatible: true } : null,
  });
  const title = (k: string) => k.toUpperCase();

  it("groups problems by reason and ignores disabled mods", () => {
    expect(seProblems(profile(false), req, title, dll("0.28.0")).map((p) => [p.kind, p.mods])).toEqual([["off", ["A", "D"]]]);
    expect(seProblems(profile(true), req, title, dll(null)).map((p) => p.kind)).toEqual(["no-dll"]);
    const mixed = seProblems(profile(true), req, title, dll("0.25.0"));
    expect(mixed.map((p) => [p.kind, p.mods])).toEqual([
      ["too-old", ["A"]],
      ["too-new", ["D"]],
    ]);
    expect(mixed[0].text).toBe("Needs script extender 0.28; v0.25.0 is installed.");
    expect(seProblems(profile(true), (k) => (k === "a" ? reqs.a : req("x")), title, dll("0.28.1"))).toEqual([]);
  });
});
