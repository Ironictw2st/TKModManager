import { describe, expect, it } from "vitest";
import { mapSegments, reconcile, updatedSince } from "./store";
import type { ModEntry, Profile, ProfileEntry } from "../ipc/commands";

const mod = (key: string, file: string, mtime = 0): ModEntry => ({
  key,
  file,
  path: file,
  dir: "",
  source: "data",
  workshopId: null,
  packType: "mod",
  size: 0,
  mtime,
  previewPath: null,
  installedUpdated: null,
  latestUpdated: null,
});

describe("reconcile", () => {
  it("appends new packs disabled, in alphabetical order, keeping existing order", () => {
    const p: Profile = { name: "x", entries: [{ key: "data:z.pack", enabled: true }], dll: false, skipIntro: false };
    const out = reconcile(p, [mod("data:b.pack", "b.pack"), mod("data:!a.pack", "!a.pack"), mod("data:z.pack", "z.pack")]);
    expect(out.entries.map((e) => e.key)).toEqual(["data:z.pack", "data:!a.pack", "data:b.pack"]);
    expect(out.entries[1].enabled).toBe(false);
  });

  it("returns the same object when nothing changed", () => {
    const p: Profile = { name: "x", entries: [{ key: "data:z.pack", enabled: true }], dll: false, skipIntro: false };
    expect(reconcile(p, [mod("data:z.pack", "z.pack")])).toBe(p);
  });

  it("keeps entries for packs that are gone, and separators", () => {
    const p: Profile = {
      name: "x",
      entries: [{ key: "sep:1", enabled: false, label: "G" }, { key: "ws:1/gone.pack", enabled: true }],
      dll: false,
      skipIntro: false,
    };
    expect(reconcile(p, []).entries).toHaveLength(2);
  });
});

describe("mapSegments", () => {
  it("applies the function inside each group and keeps separators in place", () => {
    const entries: ProfileEntry[] = [
      { key: "data:b", enabled: false },
      { key: "data:a", enabled: true },
      { key: "sep:1", enabled: false, label: "G" },
      { key: "data:d", enabled: false },
      { key: "data:c", enabled: true },
    ];
    const sorted = mapSegments(entries, (seg) => [...seg].sort((x, y) => x.key.localeCompare(y.key)));
    expect(sorted.map((e) => e.key)).toEqual(["data:a", "data:b", "sep:1", "data:c", "data:d"]);
  });
});

describe("updatedSince", () => {
  it("compares the newer of file time and Workshop time with the last launch", () => {
    expect(updatedSince(undefined, mod("k", "f", 999), undefined)).toBe(false);
    expect(updatedSince(100, mod("k", "f", 50), undefined)).toBe(false);
    expect(updatedSince(100, mod("k", "f", 150), undefined)).toBe(true);
    const ws = { id: "1", title: "", description: "", timeUpdated: 200, timeCreated: 0, tags: [], previewUrl: "", requiredItems: [], fetchedAt: 0 };
    expect(updatedSince(100, mod("k", "f", 50), ws)).toBe(true);
    expect(updatedSince(100, undefined, ws)).toBe(false);
  });
});
