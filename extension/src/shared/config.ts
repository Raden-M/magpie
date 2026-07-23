// Settings storage. Kept in `chrome.storage.local` (not `sync`) so the bearer
// token is never uploaded to the browser vendor's sync (docs/08-deployment.md).

import { ext } from "./ext.js";
import type { Settings } from "./types.js";

export const DEFAULT_SETTINGS: Settings = {
  backendUrl: "http://127.0.0.1:8787",
  token: "",
  defaultShade: "iridescent",
};

export async function getSettings(): Promise<Settings> {
  const stored = await ext.storage.local.get(DEFAULT_SETTINGS);
  return { ...DEFAULT_SETTINGS, ...stored } as Settings;
}

export async function setSettings(patch: Partial<Settings>): Promise<void> {
  await ext.storage.local.set(patch);
}

/** Normalize a base URL: trim, drop a trailing slash. */
export function normalizeBackendUrl(url: string): string {
  return url.trim().replace(/\/+$/, "");
}
