// Options page controller: load/save the backend URL, token, and default shade,
// plus a connection test against the backend's /healthz.

import { getSettings, normalizeBackendUrl, setSettings } from "../shared/config.js";

const backendUrl = document.getElementById("backendUrl") as HTMLInputElement;
const token = document.getElementById("token") as HTMLInputElement;
const defaultShade = document.getElementById("defaultShade") as HTMLSelectElement;
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
}

async function save(): Promise<void> {
  await setSettings({
    backendUrl: normalizeBackendUrl(backendUrl.value),
    token: token.value.trim(),
    defaultShade: defaultShade.value,
  });
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
    else setStatus(`Backend returned ${res.status}.`, "error");
  } catch {
    setStatus("Couldn't reach the backend.", "error");
  }
}

saveBtn.addEventListener("click", () => void save());
testBtn.addEventListener("click", () => void test());
void load();
