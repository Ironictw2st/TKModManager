// Profile export / import text and the multiplayer sync check.
//
// Export v2 (tab separated): `key<TAB>file<TAB>size<TAB>sha256` per enabled pack, in load order.
// v1 lines (`key` or `key<TAB># file`) are still accepted by the parser.

import type { ModEntry, PackHash } from "../ipc/commands";

export interface ExportRow {
  key: string;
  file: string;
  size: number | null;
  sha256: string | null;
}

export function buildExport(profileName: string, rows: PackHash[], date = new Date()): string {
  const head = `# TK Mod Manager profile "${profileName}" ${date.toISOString().slice(0, 10)} v2`;
  return [head, ...rows.map((r) => [r.key, r.file, r.size, r.sha256].join("\t"))].join("\n");
}

export function parseExport(text: string): ExportRow[] {
  const out: ExportRow[] = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const cols = raw.split("\t").map((c) => c.trim());
    const key = cols[0];
    if (!/^(ws|data|ext):/.test(key)) continue;
    const second = cols[1] ?? "";
    const file = second.startsWith("#") ? second.replace(/^#\s*/, "") : second || key.slice(key.lastIndexOf("/") + 1).replace(/^(data|ext):/, "");
    const size = cols[2] && /^\d+$/.test(cols[2]) ? Number(cols[2]) : null;
    const sha256 = cols[3] && /^[0-9a-f]{64}$/i.test(cols[3]) ? cols[3].toLowerCase() : null;
    out.push({ key, file, size, sha256 });
  }
  return out;
}

/** Find the local pack an exported row refers to: Workshop rows by id + file, the rest by
 *  file name (a partner's data/ or folder path differs from ours). */
export function matchLocal(row: ExportRow, mods: ModEntry[]): ModEntry | undefined {
  if (row.key.startsWith("ws:")) return mods.find((m) => m.key === row.key);
  const file = row.file.toLowerCase();
  return (
    mods.find((m) => m.key === row.key) ??
    mods.find((m) => m.source !== "workshop" && m.file.toLowerCase() === file) ??
    mods.find((m) => m.file.toLowerCase() === file)
  );
}

export type SyncStatus = "match" | "missing" | "disabled" | "different" | "unverified";

export interface SyncRow {
  row: ExportRow;
  local: ModEntry | undefined;
  status: SyncStatus;
}

export interface SyncResult {
  rows: SyncRow[];
  /** Enabled locally but absent from the export. */
  extra: ModEntry[];
  /** Packs present on both sides load in a different order. */
  orderDiffers: boolean;
  ok: boolean;
}

export function verifyExport(
  rows: ExportRow[],
  mods: ModEntry[],
  enabledKeysInOrder: string[],
  localHashes: Record<string, string>,
): SyncResult {
  const enabled = new Set(enabledKeysInOrder);
  const out: SyncRow[] = rows.map((row) => {
    const local = matchLocal(row, mods);
    let status: SyncStatus;
    if (!local) status = "missing";
    else if (!enabled.has(local.key)) status = "disabled";
    else if (row.size !== null && row.size !== local.size) status = "different";
    else if (row.sha256 && localHashes[local.key]) status = row.sha256 === localHashes[local.key] ? "match" : "different";
    else status = "unverified";
    return { row, local, status };
  });
  const matchedKeys = out.filter((r) => r.local && enabled.has(r.local.key)).map((r) => r.local!.key);
  const matched = new Set(matchedKeys);
  const extra = enabledKeysInOrder.filter((k) => !matched.has(k)).map((k) => mods.find((m) => m.key === k)).filter((m): m is ModEntry => !!m);
  const localOrder = enabledKeysInOrder.filter((k) => matched.has(k));
  const orderDiffers = localOrder.join("\n") !== matchedKeys.join("\n");
  const ok = out.every((r) => r.status === "match") && extra.length === 0 && !orderDiffers;
  return { rows: out, extra, orderDiffers, ok };
}
