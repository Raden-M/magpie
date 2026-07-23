// Content script. Shows the capture pill on a selection, and runs a capture on
// Keep (from the pill, the context menu, or the keyboard command), assembling it
// and handing it to the service worker.

import { getSettings } from "../shared/config.js";
import { TRIGGER_CAPTURE } from "../shared/messages.js";
import type { CaptureResultMessage } from "../shared/messages.js";
import type { Settings } from "../shared/types.js";
import { buildCapture } from "./capture.js";
import { createUi } from "./menu.js";
import { readSelection, selectionRect, type SelectionSnapshot } from "./select.js";

let settings: Settings | null = null;
let lastSelection: SelectionSnapshot | null = null;
let capturing = false;

const ui = createUi(onKeep);

void getSettings().then((s) => (settings = s));
chrome.storage.onChanged.addListener(() => {
  void getSettings().then((s) => (settings = s));
});

let debounceTimer: number | undefined;
document.addEventListener("selectionchange", () => {
  window.clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(updatePill, 150);
});

function updatePill(): void {
  const snapshot = readSelection();
  const rect = selectionRect();
  if (snapshot && rect) {
    lastSelection = snapshot;
    ui.showPill(rect, settings?.defaultShade ?? "iridescent");
  } else {
    lastSelection = null;
    ui.hidePill();
  }
}

async function onKeep(color: string, note: string | null): Promise<void> {
  if (capturing) return;
  const selection = lastSelection ?? readSelection();
  capturing = true;
  ui.setBusy(true);
  try {
    if (!settings) settings = await getSettings();
    const message = await buildCapture({
      selection,
      color,
      note,
      maxImageBytes: 5_000_000,
    });
    const result = (await chrome.runtime.sendMessage(message)) as CaptureResultMessage;
    if (result.ok) {
      const n = result.response.page.highlight_count;
      const noun = n === 1 ? "shiny thing" : "shiny things";
      ui.showToast(`Kept. ${n} ${noun} on this page.`, "ok");
      ui.hidePill();
    } else {
      ui.showToast(result.error, "error");
    }
  } catch (err) {
    ui.showToast(
      err instanceof Error ? err.message : "Something went wrong keeping this.",
      "error",
    );
  } finally {
    capturing = false;
    ui.setBusy(false);
  }
}

chrome.runtime.onMessage.addListener((message) => {
  if (message && message.type === TRIGGER_CAPTURE) {
    void onKeep(settings?.defaultShade ?? "iridescent", null);
  }
});
