// Per-mod status shown as the dot in the mod list and the block at the top of the details pane:
//   pending  red    Steam has a newer version than the one installed
//   old      amber  last Workshop update predates the current game build (informational)
//   ok       green  up to date
//   unknown  grey   Workshop item without any update data yet
//   local    grey   data/ or folder pack: nothing to compare against
// Plus the script-extender requirement: manual override > the pack's SE/script_extender.json.

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

/** What the backend read from a pack's `SE/script_extender.json` (see se_scan.rs). */
export interface SeInfo {
  required: boolean;
  path: string;
  author: string | null;
  minVersion: string | null;
  maxVersion: string | null;
  notes: string | null;
  error: string | null;
}

export interface SeRequirement {
  required: boolean;
  /** "manual" | "manifest" | "" */
  source: string;
  author: string | null;
  minVersion: string | null;
  maxVersion: string | null;
  notes: string | null;
  error: string | null;
}

const NONE: SeRequirement = { required: false, source: "", author: null, minVersion: null, maxVersion: null, notes: null, error: null };

/** Manual override wins; otherwise the pack's manifest decides. The manifest's version range
 *  still applies when the user forces "requires". */
export function seRequirement(scan: SeInfo | undefined, meta: ModMeta | undefined): SeRequirement {
  const fromScan: SeRequirement = scan?.required
    ? { required: true, source: "manifest", author: scan.author, minVersion: scan.minVersion, maxVersion: scan.maxVersion, notes: scan.notes, error: scan.error }
    : NONE;
  if (meta?.seOverride === true) return { ...fromScan, required: true, source: "manual" };
  if (meta?.seOverride === false) return { ...NONE, source: "manual" };
  return fromScan;
}

/** "0.28", "0.28 or newer", "up to 0.28", "0.26 – 0.28", or "" */
export function seRangeText(r: Pick<SeRequirement, "minVersion" | "maxVersion">): string {
  const { minVersion: lo, maxVersion: hi } = r;
  if (lo && hi) return lo === hi ? lo : `${lo} – ${hi}`;
  if (lo) return `${lo} or newer`;
  if (hi) return `up to ${hi}`;
  return "";
}

export function seSourceText(r: SeRequirement): string {
  if (!r.required) return r.source === "manual" ? "Marked by you as not needing the script extender" : "Does not use the script extender";
  const range = seRangeText(r);
  const bits = [r.source === "manual" ? "Marked by you as requiring it" : "Declared in SE/script_extender.json"];
  if (r.author) bits.push(`by ${r.author}`);
  if (range) bits.push(`version ${range}`);
  return bits.join(" · ");
}

function parts(v: string): number[] {
  return v
    .trim()
    .replace(/^v/i, "")
    .split(/[.-]/)
    .map((x) => parseInt(x, 10) || 0);
}

/** Dotted numeric compare; missing parts count as 0 (0.28 == 0.28.0). */
export function compareVersions(a: string, b: string): number {
  const pa = parts(a);
  const pb = parts(b);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d < 0 ? -1 : 1;
  }
  return 0;
}

export function versionLess(a: string, b: string): boolean {
  return compareVersions(a, b) < 0;
}

/** Is `have` newer than `max`, compared at the maximum's own precision (max 0.28 allows 0.28.x)? */
export function versionAboveMax(have: string, max: string): boolean {
  const n = parts(max).length;
  const h = parts(have).slice(0, n).join(".");
  return compareVersions(h, max) > 0;
}

export type SeUnmet = "off" | "no-dll" | "too-old" | "too-new" | null;

/** Why this launch would not satisfy the requirement (null = satisfied or not required). */
export function seUnmet(r: SeRequirement, profileDll: boolean, dllVersion: string | null | undefined): SeUnmet {
  if (!r.required) return null;
  if (!profileDll) return "off";
  if (!dllVersion) return "no-dll";
  if (r.minVersion && versionLess(dllVersion, r.minVersion)) return "too-old";
  if (r.maxVersion && versionAboveMax(dllVersion, r.maxVersion)) return "too-new";
  return null;
}

export function seUnmetText(u: SeUnmet, r: SeRequirement, have: string | null | undefined): string {
  switch (u) {
    case "off":
      return "The script extender is off for this profile.";
    case "no-dll":
      return "No script extender DLL matches this game build.";
    case "too-old":
      return `Needs script extender ${seRangeText(r)}; v${have} is installed.`;
    case "too-new":
      return `Supports script extender ${seRangeText(r)} only; v${have} is installed.`;
    default:
      return "";
  }
}

export type SeProblem = { kind: Exclude<SeUnmet, null>; mods: string[]; text: string };

/** What stops the enabled mods that need the extender, grouped by reason, for the launch panel. */
export function seProblems(
  profile: Profile,
  requirement: (key: string) => SeRequirement,
  title: (key: string) => string,
  dll: DllStatus | null,
): SeProblem[] {
  const have = dll?.selected?.version ?? null;
  const groups = new Map<Exclude<SeUnmet, null>, { mods: string[]; text: string }>();
  for (const e of profile.entries) {
    if (!e.enabled || e.key.startsWith("sep:")) continue;
    const r = requirement(e.key);
    const u = seUnmet(r, profile.dll, have);
    if (!u) continue;
    const g = groups.get(u) ?? { mods: [], text: "" };
    g.mods.push(title(e.key));
    // For version problems name the range when every mod agrees, else a generic line.
    const t = seUnmetText(u, r, have);
    g.text = g.text && g.text !== t ? (u === "too-old" ? `Some mods need a newer script extender than v${have}.` : `Some mods do not support script extender v${have}.`) : t;
    groups.set(u, g);
  }
  const order: Exclude<SeUnmet, null>[] = ["off", "no-dll", "too-old", "too-new"];
  return order.filter((k) => groups.has(k)).map((k) => ({ kind: k, ...groups.get(k)! }));
}
