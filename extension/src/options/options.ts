// Options page controller: load/save the backend URL, token, and default shade,
// request access to whichever backend was named, and test the connection.

import { getSettings, setSettings } from "../shared/config.js";
import { formatExclusions, parseExclusions } from "../shared/exclusions.js";
import { ext } from "../shared/ext.js";
import {
  normalizeBackendUrl,
  originPatternFor,
  parseConnectString,
} from "../shared/pairing.js";

const backendUrl = document.getElementById("backendUrl") as HTMLInputElement;
const token = document.getElementById("token") as HTMLInputElement;
const defaultShade = document.getElementById("defaultShade") as HTMLSelectElement;
const excludedSites = document.getElementById("excludedSites") as HTMLTextAreaElement;
const saveBtn = document.getElementById("save") as HTMLButtonElement;
const testBtn = document.getElementById("test") as HTMLButtonElement;
const status = document.getElementById("status") as HTMLSpanElement;

function setStatus(text: string, tone: "" | "ok" | "error" = ""): void {
  status.textContent = text;
  status.className = `status${tone ? " " + tone : ""}`;
}

async function load(): Promise<void> {
  const settings = await getSettings();
  backendUrl.value = settings.backendUrl;
  token.value = settings.token;
  defaultShade.value = settings.defaultShade;
  excludedSites.value = formatExclusions(settings.excludedSites);
}

// The context menu edits the same list, so reflect it while the page is open
// rather than letting a stale textarea overwrite it on the next Save.
ext.storage.onChanged.addListener((changes) => {
  if (changes.excludedSites && document.activeElement !== excludedSites) {
    excludedSites.value = formatExclusions((changes.excludedSites.newValue ?? []) as string[]);
  }
});

/**
 * Ask for access to this backend, if we don't already have it.
 *
 * Magpie can be hosted anywhere, so the manifest declares the origins as
 * *optional* and we request the single one in use. That keeps the install
 * prompt quiet, and it is the only way a remote backend works at all - without
 * a host permission the request is subject to CORS, and on Firefox the origin
 * to allowlist is a per-profile random UUID nobody can predict.
 *
 * Called before anything is awaited: Firefox only honours a permission request
 * still tied to the click that started it.
 *
 * Rejects rather than resolving false when the browser won't take the pattern
 * at all - an IPv6 literal has no match-pattern spelling, for one - so the
 * caller can say which of the two happened instead of blaming the user for
 * declining a prompt they were never shown.
 */
function requestAccess(url: string): Promise<boolean> {
  const origin = originPatternFor(url);
  if (!origin) return Promise.reject(new Error(`unusable origin: ${url}`));
  // Resolves true immediately when the permission is already held.
  return ext.permissions.request({ origins: [origin] });
}

async function save(): Promise<void> {
  const pairing = parseConnectString(backendUrl.value);
  if (!pairing.backendUrl) {
    setStatus("Enter a backend URL first.", "error");
    return;
  }
  if (!originPatternFor(pairing.backendUrl)) {
    setStatus("That doesn't look like an http:// or https:// URL.", "error");
    return;
  }

  // A pasted connect string carries the token; reflect both fields before the
  // await so what is saved is what the page shows.
  backendUrl.value = pairing.backendUrl;
  if (pairing.token) token.value = pairing.token;

  let granted: boolean;
  try {
    granted = await requestAccess(pairing.backendUrl);
  } catch {
    setStatus(
      "This browser can't grant access to that address. Use a hostname or an IPv4 address.",
      "error",
    );
    return;
  }
  if (!granted) {
    setStatus("Magpie needs permission to reach that backend. Nothing saved.", "error");
    return;
  }

  const exclusions = parseExclusions(excludedSites.value);
  await setSettings({
    backendUrl: pairing.backendUrl,
    token: token.value.trim(),
    defaultShade: defaultShade.value,
    excludedSites: exclusions,
  });
  // Show what was actually stored: entries are tidied, and a line that parses
  // to nothing disappearing is the only signal that it was never a rule.
  excludedSites.value = formatExclusions(exclusions);
  setStatus("Saved.", "ok");
  window.setTimeout(() => setStatus(""), 2500);
}

async function test(): Promise<void> {
  const base = normalizeBackendUrl(backendUrl.value);
  if (!base) {
    setStatus("Enter a backend URL first.", "error");
    return;
  }
  setStatus("Testing…");
  const headers: Record<string, string> = {};
  if (token.value.trim()) headers["Authorization"] = `Bearer ${token.value.trim()}`;
  try {
    const res = await fetch(`${base}/healthz`, { headers });
    if (res.ok) setStatus("Connected.", "ok");
    else if (res.status === 401) setStatus("Reachable, but the token was rejected.", "error");
    else if (res.status === 421)
      setStatus("Reached a Magpie that doesn't answer to that hostname.", "error");
    else setStatus(`Backend returned ${res.status}.`, "error");
  } catch {
    setStatus("Couldn't reach the backend. Save first, so access is granted.", "error");
  }
}

saveBtn.addEventListener("click", () => void save());
testBtn.addEventListener("click", () => void test());
void load();
