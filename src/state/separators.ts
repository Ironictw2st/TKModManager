// Separators double as group headers (MO2 style): a separator entry heads every pack entry
// that follows it, up to the next separator. All functions are pure over the entries array.

import type { ProfileEntry } from "../ipc/commands";

export const SEP_PREFIX = "sep:";

export const isSeparator = (e: ProfileEntry | { key: string }): boolean => e.key.startsWith(SEP_PREFIX);

export function newSeparatorKey(): string {
  const id = typeof crypto !== "undefined" && "randomUUID" in crypto ? crypto.randomUUID() : String(Date.now()) + Math.random();
  return `${SEP_PREFIX}${id}`;
}

/** Keys of the pack entries that belong to the separator `sepKey`. */
export function groupMembers(entries: ProfileEntry[], sepKey: string): string[] {
  const start = entries.findIndex((e) => e.key === sepKey);
  if (start < 0) return [];
  const out: string[] = [];
  for (let i = start + 1; i < entries.length && !isSeparator(entries[i]); i++) out.push(entries[i].key);
  return out;
}

/** The separator heading `key`, if any. */
export function groupOf(entries: ProfileEntry[], key: string): string | null {
  const idx = entries.findIndex((e) => e.key === key);
  for (let i = idx; i >= 0; i--) if (isSeparator(entries[i])) return entries[i].key;
  return null;
}

/** Insert a new separator before `beforeKey` (or at the end). Returns the new entries + key. */
export function addSeparator(entries: ProfileEntry[], label: string, beforeKey: string | null): { entries: ProfileEntry[]; key: string } {
  const key = newSeparatorKey();
  const sep: ProfileEntry = { key, enabled: false, label };
  const at = beforeKey ? entries.findIndex((e) => e.key === beforeKey) : -1;
  const next = at < 0 ? [...entries, sep] : [...entries.slice(0, at), sep, ...entries.slice(at)];
  return { entries: next, key };
}

/** Remove the separator only; its members stay where they are (joining the group above). */
export function removeSeparator(entries: ProfileEntry[], sepKey: string): ProfileEntry[] {
  return entries.filter((e) => e.key !== sepKey);
}

export function renameSeparator(entries: ProfileEntry[], sepKey: string, label: string): ProfileEntry[] {
  return entries.map((e) => (e.key === sepKey ? { ...e, label } : e));
}

export function setCollapsed(entries: ProfileEntry[], sepKey: string, collapsed: boolean): ProfileEntry[] {
  return entries.map((e) => (e.key === sepKey ? { ...e, collapsed } : e));
}

/** Enable/disable every member of a group. */
export function toggleGroup(entries: ProfileEntry[], sepKey: string, enabled: boolean): ProfileEntry[] {
  const members = new Set(groupMembers(entries, sepKey));
  return entries.map((e) => (members.has(e.key) ? { ...e, enabled } : e));
}

/** "all" | "none" | "some" | "empty" for the header checkbox. */
export function groupState(entries: ProfileEntry[], sepKey: string, exists: (key: string) => boolean): "all" | "none" | "some" | "empty" {
  const members = groupMembers(entries, sepKey).filter(exists);
  if (members.length === 0) return "empty";
  const byKey = new Map(entries.map((e) => [e.key, e]));
  const on = members.filter((k) => byKey.get(k)?.enabled).length;
  return on === 0 ? "none" : on === members.length ? "all" : "some";
}

/** Move `keys` as a block so it lands before/after `overKey`. A dragged separator takes its
 *  whole group along. */
export function moveBlock(entries: ProfileEntry[], keys: string[], overKey: string): ProfileEntry[] {
  const expanded = new Set<string>();
  for (const k of keys) {
    expanded.add(k);
    if (k.startsWith(SEP_PREFIX)) groupMembers(entries, k).forEach((m) => expanded.add(m));
  }
  if (expanded.has(overKey)) return entries;
  const from = entries.findIndex((e) => expanded.has(e.key));
  const to = entries.findIndex((e) => e.key === overKey);
  if (from < 0 || to < 0) return entries;
  const moving = entries.filter((e) => expanded.has(e.key));
  const rest = entries.filter((e) => !expanded.has(e.key));
  let at = rest.findIndex((e) => e.key === overKey);
  if (from < to) {
    at += 1;
    // Dropping onto a separator while moving down: land after that separator's whole group
    // only when a group is being moved (keeps groups from being split accidentally).
    if (overKey.startsWith(SEP_PREFIX) && keys.some((k) => k.startsWith(SEP_PREFIX))) {
      while (at < rest.length && !isSeparator(rest[at])) at++;
    }
  }
  return [...rest.slice(0, at), ...moving, ...rest.slice(at)];
}

/** Keys hidden because their separator is collapsed. */
export function collapsedKeys(entries: ProfileEntry[]): Set<string> {
  const hidden = new Set<string>();
  let collapsed = false;
  for (const e of entries) {
    if (isSeparator(e)) collapsed = !!e.collapsed;
    else if (collapsed) hidden.add(e.key);
  }
  return hidden;
}
