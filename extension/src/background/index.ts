// Service worker (MV3 background). Owns the backend connection: it receives an
// assembled capture from the content script, rebuilds the image blobs, and POSTs
// to the backend. The fetch runs here (extension origin) so it matches the
// backend's CORS allowlist. MV3 workers are ephemeral - no long-lived state.

import { base64ToBlob } from "../shared/bytes.js";
import { getSettings } from "../shared/config.js";
import { ext } from "../shared/ext.js";
import { CaptureError, postCapture } from "../shared/http.js";
import { CAPTURE_MESSAGE, TRIGGER_CAPTURE } from "../shared/messages.js";
import type { CaptureRequestMessage, CaptureResultMessage } from "../shared/messages.js";

const CONTEXT_MENU_ID = "magpie-keep-selection";

ext.runtime.onInstalled.addListener(() => {
  ext.contextMenus.create({
    id: CONTEXT_MENU_ID,
    title: "Keep selection in Magpie",
    contexts: ["selection"],
  });
});

ext.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === CONTEXT_MENU_ID && tab?.id !== undefined) {
    void ext.tabs.sendMessage(tab.id, { type: TRIGGER_CAPTURE });
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
      err instanceof CaptureError
        ? err.message
        : `Couldn't keep this: ${err instanceof Error ? err.message : String(err)}`;
    return { ok: false, error };
  }
}

ext.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message && message.type === CAPTURE_MESSAGE) {
    handleCapture(message as CaptureRequestMessage).then(sendResponse);
    return true; // keep the channel open for the async response
  }
  return false;
});

console.debug("magpie: background service worker ready");
