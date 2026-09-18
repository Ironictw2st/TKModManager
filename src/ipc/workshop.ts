// Workshop metadata (title, description, updated time, required items) fetched through the
// Rust side and cached on disk there. Shape mirrors src-tauri/src/workshop.rs.

import { invoke } from "@tauri-apps/api/core";

export interface WorkshopItem {
  id: string;
  title: string;
  description: string;
  timeUpdated: number;
  timeCreated: number;
  tags: string[];
  previewUrl: string;
  /** Required item ids (Workshop dependencies), when known. */
  requiredItems: string[];
  /** When this record was fetched (unix seconds). */
  fetchedAt: number;
  /** Seeded from the CA launcher cache rather than Steam. */
  fromLauncherCache?: boolean;
}

export const workshopApi = {
  /** Cached items for the given ids (no network). */
  cached: () => invoke<Record<string, WorkshopItem>>("workshop_cached"),
  /** Fetch/refresh items; `force` ignores the cache TTL. */
  fetch: (ids: string[], force: boolean) => invoke<Record<string, WorkshopItem>>("workshop_fetch", { ids, force }),
};
