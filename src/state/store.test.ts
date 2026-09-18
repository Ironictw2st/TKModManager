import { describe, expect, it } from "vitest";
import { reconcile } from "./store";
import type { ModEntry, Profile } from "../ipc/commands";

const mod = (key: string, file: string): ModEntry => ({
  key,
  file,
  path: file,
  dir: "",
  source: "data",
  workshopId: null,
  packType: "mod",
  size: 0,
  mtime: 0,
  previewPath: null,
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

  it("keeps entries for packs that are gone", () => {
    const p: Profile = { name: "x", entries: [{ key: "ws:1/gone.pack", enabled: true }], dll: false, skipIntro: false };
    expect(reconcile(p, []).entries).toHaveLength(1);
  });
});
