import { describe, expect, it } from "vitest";
import type { ProfileEntry } from "../ipc/commands";
import { addSeparator, collapsedKeys, groupMembers, groupOf, groupState, moveBlock, removeSeparator, toggleGroup } from "./separators";

const e = (key: string, enabled = false, extra: Partial<ProfileEntry> = {}): ProfileEntry => ({ key, enabled, ...extra });

const base = (): ProfileEntry[] => [
  e("data:top.pack", true),
  e("sep:A", false, { label: "A" }),
  e("data:a1.pack", true),
  e("data:a2.pack"),
  e("sep:B", false, { label: "B" }),
  e("data:b1.pack"),
];

describe("separators", () => {
  it("finds members and owners", () => {
    expect(groupMembers(base(), "sep:A")).toEqual(["data:a1.pack", "data:a2.pack"]);
    expect(groupMembers(base(), "sep:B")).toEqual(["data:b1.pack"]);
    expect(groupOf(base(), "data:a2.pack")).toBe("sep:A");
    expect(groupOf(base(), "data:top.pack")).toBeNull();
  });

  it("toggles a whole group and reports its state", () => {
    const all = () => true;
    expect(groupState(base(), "sep:A", all)).toBe("some");
    const on = toggleGroup(base(), "sep:A", true);
    expect(groupState(on, "sep:A", all)).toBe("all");
    expect(on.find((x) => x.key === "data:b1.pack")?.enabled).toBe(false);
    expect(groupState(toggleGroup(base(), "sep:A", false), "sep:A", all)).toBe("none");
    expect(groupState(base(), "sep:A", () => false)).toBe("empty");
  });

  it("adds before a key and removes without touching members", () => {
    const { entries, key } = addSeparator(base(), "New", "data:b1.pack");
    expect(entries.map((x) => x.key)).toEqual(["data:top.pack", "sep:A", "data:a1.pack", "data:a2.pack", "sep:B", key, "data:b1.pack"]);
    const removed = removeSeparator(base(), "sep:B");
    expect(groupMembers(removed, "sep:A")).toEqual(["data:a1.pack", "data:a2.pack", "data:b1.pack"]);
  });

  it("moves a separator with its group", () => {
    const moved = moveBlock(base(), ["sep:A"], "data:b1.pack");
    expect(moved.map((x) => x.key)).toEqual(["data:top.pack", "sep:B", "data:b1.pack", "sep:A", "data:a1.pack", "data:a2.pack"]);
    const up = moveBlock(base(), ["sep:B"], "data:top.pack");
    expect(up.map((x) => x.key)).toEqual(["sep:B", "data:b1.pack", "data:top.pack", "sep:A", "data:a1.pack", "data:a2.pack"]);
  });

  it("moves plain packs between groups and ignores drops onto itself", () => {
    const moved = moveBlock(base(), ["data:b1.pack"], "data:a1.pack");
    expect(groupMembers(moved, "sep:A")).toEqual(["data:b1.pack", "data:a1.pack", "data:a2.pack"]);
    const same = base();
    expect(moveBlock(same, ["sep:A"], "data:a1.pack")).toBe(same);
  });

  it("hides members of collapsed groups", () => {
    const entries = base().map((x) => (x.key === "sep:A" ? { ...x, collapsed: true } : x));
    expect([...collapsedKeys(entries)]).toEqual(["data:a1.pack", "data:a2.pack"]);
  });
});
