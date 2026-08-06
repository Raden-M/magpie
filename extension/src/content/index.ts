// Content script. Shows the capture pill on a selection, and runs a capture on
// Keep (from the pill, the context menu, or the keyboard command), assembling it
// and handing it to the service worker.

import { getSettings } from "../shared/config.js";
import { errorMessage } from "../shared/errors.js";
import { isExcluded } from "../shared/exclusions.js";
import { ext } from "../shared/ext.js";
import { PAGE_STATE, TRIGGER_CAPTURE } from "../shared/messages.js";
import type { CaptureResultMessage } from "../shared/messages.js";
import type { Settings } from "../shared/types.js";
import { buildCapture } from "./capture.js";
import { createUi } from "./menu.js";
import { readSelection, selectionRect, type SelectionSnapshot } from "./select.js";

let settings: Settings | null = null;
let excluded = false;
let lastSelection: SelectionSnapshot | null = null;
let capturing = false;

const ui = createUi(onKeep, () => {
  lastSelection = null;
});

function applySettings(s: Settings): void {
  const first = settings === null;
  const was = excluded;
  settings = s;
  excluded = isExcluded(location.href, s.excludedSites);
  if (excluded) {
    lastSelection = null;
    ui.hidePill();
  }
  // Toggling from the context menu happens under the native menu with nothing
  // else to show for it, so confirm it in the page - but never on load, where
  // the state is not news.
  if (!first && excluded !== was) {
    // "ok" either way: the user asked for it and it worked. Red is for failure.
    ui.showToast(
      excluded ? "Magpie is switched off on this site." : "Magpie is back on this site.",
      "ok",
    );
  }
  announcePageState();
}

/** Tell the worker what this page is, so the context menu matches it. */
function announcePageState(): void {
  if (document.visibilityState !== "visible") return;
  void ext.runtime.sendMessage({ type: PAGE_STATE, excluded }).catch(() => {
    // The worker may be asleep with no listener yet; the next event re-announces.
  });
}

void getSettings().then(applySettings);
ext.storage.onChanged.addListener(() => {
  void getSettings().then(applySettings);
});
document.addEventListener("visibilitychange", announcePageState);
// Focusing a second window doesn't change visibility, and the menu is shared
// across windows; re-announcing as the native menu opens keeps it honest.
document.addEventListener("contextmenu", announcePageState, true);

let debounceTimer: number | undefined;
document.addEventListener("selectionchange", () => {
  window.clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(updatePill, 150);
});

function sameSelection(a: SelectionSnapshot | null, b: SelectionSnapshot): boolean {
  return !!a && a.quote === b.quote && a.prefix === b.prefix && a.suffix === b.suffix;
}

function updatePill(): void {
  if (excluded) return;
  const snapshot = readSelection();
  const rect = selectionRect();
  if (snapshot && rect) {
    // Reposition on the same selection; only a genuinely new one resets the pill
    // - otherwise opening the note (which fires selectionchange) would stomp it.
    const same = sameSelection(lastSelection, snapshot);
    lastSelection = snapshot;
    if (same) ui.repositionPill(rect);
    else ui.showPill(rect, settings?.defaultShade ?? "iridescent");
  } else {
    // Focusing the note input can momentarily collapse the page selection; don't
    // tear the pill down while the user is mid-note.
    if (ui.isEngaged()) return;
    lastSelection = null;
    ui.hidePill();
  }
}

async function onKeep(color: string, note: string | null): Promise<void> {
  if (capturing || excluded) return;
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
    const result = (await ext.runtime.sendMessage(message)) as CaptureResultMessage;
    if (result.ok) {
      const n = result.response.page.highlight_count;
      const noun = n === 1 ? "shiny thing" : "shiny things";
      ui.showToast(`Kept. ${n} ${noun} on this page.`, "ok");
      ui.hidePill();
      // Clear the selection so it visibly deselects on Keep, rather than lingering.
      window.getSelection()?.removeAllRanges();
      lastSelection = null;
    } else {
      ui.showToast(result.error, "error");
    }
  } catch (err) {
    console.error("magpie: capture failed", err);
    ui.showToast(`Couldn't keep this: ${errorMessage(err)}`, "error");
  } finally {
    capturing = false;
    ui.setBusy(false);
  }
}

ext.runtime.onMessage.addListener((message) => {
  if (message && message.type === TRIGGER_CAPTURE) {
    // The keyboard command reaches even an excluded page - the browser owns the
    // shortcut. Say why nothing happened rather than swallowing the keystroke.
    if (excluded) ui.showToast("Magpie is switched off on this site.", "error");
    else void onKeep(settings?.defaultShade ?? "iridescent", null);
  }
});
