// Applies appearance settings to <html>: data-theme (dark/light/system) and the accent colour.

import type { Settings } from "./ipc/commands";

const LIGHT_QUERY = "(prefers-color-scheme: light)";

function resolveMode(mode: Settings["themeMode"]): "light" | "dark" {
  if (mode === "light" || mode === "dark") return mode;
  return window.matchMedia?.(LIGHT_QUERY).matches ? "light" : "dark";
}

function hexToChannels(hex: string): string | null {
  const m = /^#?([0-9a-fA-F]{6})$/.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return `${(n >> 16) & 255} ${(n >> 8) & 255} ${n & 255}`;
}

export function applyTheme(settings: Pick<Settings, "themeMode" | "accent">): void {
  const root = document.documentElement;
  root.dataset.theme = resolveMode(settings.themeMode);
  const accent = hexToChannels(settings.accent);
  if (accent) root.style.setProperty("--accent", accent);
}
