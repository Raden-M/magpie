// Service worker (MV3 background). Owns the backend connection: it receives an
// assembled capture from the content script, rebuilds the image blobs, and POSTs
// to the backend. The fetch runs here (extension origin) so it matches the
// backend's CORS allowlist. MV3 workers are ephemeral - no long-lived state.

import { base64ToBlob } from "../shared/bytes.js";
import { getSettings, setSettings } from "../shared/config.js";
import { errorMessage } from "../shared/errors.js";
import { exclusionFor, isExcluded, withoutExclusionsFor } from "../shared/exclusions.js";
import { ext } from "../shared/ext.js";
import { CaptureError, postCapture } from "../shared/http.js";
import { CAPTURE_MESSAGE, PAGE_STATE, TRIGGER_CAPTURE } from "../shared/messages.js";
import type {
  CaptureRequestMessage,
  CaptureResultMessage,
  PageStateMessage,
} from "../shared/messages.js";

const CONTEXT_MENU_ID = "magpie-keep-selection";
const EXCLUDE_MENU_ID = "magpie-toggle-site";

ext.runtime.onInstalled.addListener(() => {
  ext.contextMenus.create({
    id: CONTEXT_MENU_ID,
    title: "Keep selection in Magpie",
    contexts: ["selection"],
  });
  ext.contextMenus.create({
    id: EXCLUDE_MENU_ID,
    title: "Don't offer Magpie on this site",
    contexts: ["page", "selection"],
    documentUrlPatterns: ["http://*/*", "https://*/*"],
  });
});

/**
 * Match the menu to the page the visible content script just described. Menu
 * state is persistent, so this survives the worker being torn down; a page with
 * no content script (a browser page, the PDF viewer) leaves it as it was, and
 * both items no-op there.
 */
function showMenuFor(excluded: boolean): void {
  const update = (id: string, props: chrome.contextMenus.UpdateProperties): void => {
    // Rejects if the worker woke before `onInstalled` created the items.
    void Promise.resolve(ext.contextMenus.update(id, props)).catch(() => {});
  };
  update(CONTEXT_MENU_ID, { visible: !excluded });
  update(EXCLUDE_MENU_ID, {
    title: excluded ? "Offer Magpie on this site again" : "Don't offer Magpie on this site",
  });
}

async function toggleSite(pageUrl: string): Promise<void> {
  const settings = await getSettings();
  const entry = exclusionFor(pageUrl);
  if (!entry) return;

  const excludedSites = isExcluded(pageUrl, settings.excludedSites)
    ? withoutExclusionsFor(pageUrl, settings.excludedSites)
    : [...settings.excludedSites, entry];
  await setSettings({ excludedSites });
  // The content script re-reads on the storage change and re-announces, which
  // is what puts the menu back in step.
}

// Capturing has three ways in - the pill, the context menu, the keyboard - and
// settings had none. The toolbar icon carries no popup, so without this it is a
// button that does nothing, and pairing is buried under about:addons where
// nobody would find it.
ext.action.onClicked.addListener(() => {
  void ext.runtime.openOptionsPage();
});

ext.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === CONTEXT_MENU_ID && tab?.id !== undefined) {
    void ext.tabs.sendMessage(tab.id, { type: TRIGGER_CAPTURE });
  }
  if (info.menuItemId === EXCLUDE_MENU_ID && info.pageUrl) {
    void toggleSite(info.pageUrl);
  }
});

ext.commands.onCommand.addListener(async (command) => {
  if (command !== "keep-selection") return;
  const [tab] = await ext.tabs.query({ active: true, currentWindow: true });
  if (tab?.id !== undefined) {
    void ext.tabs.sendMessage(tab.id, { type: TRIGGER_CAPTURE });
  }
});

async function handleCapture(msg: CaptureRequestMessage): Promise<CaptureResultMessage> {
  try {
    const settings = await getSettings();
    const images = msg.images.map((img) => ({
      hash: img.hash,
      blob: base64ToBlob(img.base64, img.mime),
    }));
    const response = await postCapture(settings, { payload: msg.payload, images });
    return { ok: true, response };
  } catch (err) {
    console.error("magpie: capture failed", err);
    const error =
      err instanceof CaptureError ? err.message : `Couldn't keep this: ${errorMessage(err)}`;
    return { ok: false, error };
  }
}

ext.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message && message.type === CAPTURE_MESSAGE) {
    handleCapture(message as CaptureRequestMessage).then(sendResponse);
    return true; // keep the channel open for the async response
  }
  if (message && message.type === PAGE_STATE) {
    showMenuFor((message as PageStateMessage).excluded);
  }
  return false;
});

console.debug("magpie: background service worker ready");
