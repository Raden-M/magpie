// Settings storage. Kept in `chrome.storage.local` (not `sync`) so the bearer
// token is never uploaded to the browser vendor's sync (docs/usage.md).
//
// Parsing what goes in these fields lives in `pairing.ts`.

import { ext } from "./ext.js";
import type { Settings } from "./types.js";

export const DEFAULT_SETTINGS: Settings = {
  backendUrl: "http://127.0.0.1:8787",
  token: "",
  defaultShade: "iridescent",
  excludedSites: [],
};

export async function getSettings(): Promise<Settings> {
  const stored = await ext.storage.local.get(DEFAULT_SETTINGS);
  return { ...DEFAULT_SETTINGS, ...stored } as Settings;
}

export async function setSettings(patch: Partial<Settings>): Promise<void> {
  await ext.storage.local.set(patch);
}
