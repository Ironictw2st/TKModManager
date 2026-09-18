// Per-mod status shown as the dot in the mod list and the block at the top of the details pane:
//   pending  red    Steam has a newer version than the one installed
//   old      amber  last Workshop update predates the current game build (informational)
//   ok       green  up to date
//   unknown  grey   Workshop item without any update data yet
//   local    grey   data/ or folder pack: nothing to compare against
// Plus the script-extender requirement (manual override > marker file > Lua scan).

import type { DllStatus, ModEntry, ModMeta, Profile } from "../ipc/commands";
import type { WorkshopItem } from "../ipc/workshop";
import { formatDate } from "./format";

export type ModStatusKind = "pending" | "old" | "ok" | "unknown" | "local";

export interface ModStatus {
  kind: ModStatusKind;
  /** One-line explanation (tooltip / details). */
  text: string;
  /** Newest known publish time (unix seconds), if any. */
  latest: number | null;
  installed: number | null;
}

export const STATUS_LABEL: Record<ModStatusKind, string> = {
  pending: "Update pending",
  old: "Older than game patch",
  ok: "Up to date",
  unknown: "Unknown",
  local: "Local file",
};

/** Sort weight: problems first. */
export const STATUS_RANK: Record<ModStatusKind, number> = { pending: 0, old: 1, unknown: 2, local: 3, ok: 4 };

export function modStatus(mod: ModEntry | undefined, ws: WorkshopItem | undefined, cutoff: number): ModStatus {
  if (!mod) return { kind: "unknown", text: "Not installed", latest: null, installed: null };
  if (mod.source !== "workshop") {
    return { kind: "local", text: mod.source === "data" ? "Local pack in data/: no Workshop version to compare" : "Pack from an extra folder: no Workshop version to compare", latest: null, installed: null };
  }
  const apiLatest = ws && !ws.fromLauncherCache && ws.timeUpdated ? ws.timeUpdated : 0;
  const latest = Math.max(mod.latestUpdated ?? 0, apiLatest) || null;
  const installed = mod.installedUpdated ?? null;
  if (latest && installed && latest > installed) {
    return {
      kind: "pending",
      text: `Update pending: Steam has the ${formatDate(latest)} version, ${formatDate(installed)} is installed. Steam downloads it the next time it updates Workshop items (restart Steam or open the Downloads page).`,
      latest,
      installed,
    };
  }
  const published = latest ?? installed;
  if (!published) return { kind: "unknown", text: "No update information yet (Steam has not reported this item)", latest: null, installed };
  if (cutoff > 0 && published < cutoff) {
    return {
      kind: "old",
      text: `Last updated ${formatDate(published)}, before the current game build (${formatDate(cutoff)}). It may still work fine.`,
      latest,
      installed,
    };
  }
  return { kind: "ok", text: `Up to date (updated ${formatDate(published)})`, latest, installed };
}

export interface SeInfo {
  required: boolean;
  source: string;
  detail: string;
  minVersion: string | null;
}

export interface SeRequirement {
  required: boolean;
  /** "manual" | "marker" | "lua" | "" */
  source: string;
  detail: string;
  minVersion: string | null;
}

export function seRequirement(scan: SeInfo | undefined, meta: ModMeta | undefined): SeRequirement {
  if (meta?.seOverride !== undefined && meta?.seOverride !== null) {
    return { required: meta.seOverride, source: "manual", detail: "", minVersion: scan?.minVersion ?? null };
  }
  if (scan?.required) return { required: true, source: scan.source, detail: scan.detail, minVersion: scan.minVersion };
  return { required: false, source: "", detail: "", minVersion: null };
}

export function seSourceText(r: SeRequirement): string {
  if (!r.required) return r.source === "manual" ? "Marked by you as not needing the script extender" : "Does not use the script extender";
  switch (r.source) {
    case "manual":
      return "Marked by you as requiring the script extender";
    case "marker":
      return `Declares it in ${r.detail}${r.minVersion ? ` (needs v${r.minVersion} or newer)` : ""}`;
    case "lua":
      return `Calls the script extender API in ${r.detail}`;
    default:
      return "Requires the script extender";
  }
}

/** Compare dotted versions ("0.23.2" vs "0.24.0"); non-numeric parts count as 0. */
export function versionLess(a: string, b: string): boolean {
  const pa = a.split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  const pb = b.split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d < 0;
  }
  return false;
}

export type SeProblem =
  | { kind: "off"; mods: string[] }
  | { kind: "no-dll"; mods: string[] }
  | { kind: "too-old"; mods: string[]; need: string; have: string };

/** What stops the enabled mods that need the extender from working with this launch. */
export function seProblems(
  profile: Profile,
  requirement: (key: string) => SeRequirement,
  title: (key: string) => string,
  dll: DllStatus | null,
): SeProblem[] {
  const needing = profile.entries.filter((e) => e.enabled && !e.key.startsWith("sep:") && requirement(e.key).required).map((e) => e.key);
  if (needing.length === 0) return [];
  const names = needing.map(title);
  if (!profile.dll) return [{ kind: "off", mods: names }];
  const have = dll?.selected?.version;
  if (!have) return [{ kind: "no-dll", mods: names }];
  let need: string | null = null;
  const tooOld: string[] = [];
  for (const k of needing) {
    const min = requirement(k).minVersion;
    if (min && versionLess(have, min)) {
      tooOld.push(title(k));
      if (!need || versionLess(need, min)) need = min;
    }
  }
  return need ? [{ kind: "too-old", mods: tooOld, need, have }] : [];
}
