// Service worker (MV3 background). Owns the backend connection: it receives an
// assembled capture from the content script, rebuilds the image blobs, and POSTs
// to the backend. The fetch runs here (extension origin) so it matches the
// backend's CORS allowlist. MV3 workers are ephemeral - no long-lived state.

import { base64ToBlob } from "../shared/bytes.js";
import { getSettings } from "../shared/config.js";
import { CaptureError, postCapture } from "../shared/http.js";
import { CAPTURE_MESSAGE } from "../shared/messages.js";
import type { CaptureRequestMessage, CaptureResultMessage } from "../shared/messages.js";

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
    const error =
      err instanceof CaptureError
        ? err.message
        : "Something went wrong keeping this. Please try again.";
    return { ok: false, error };
  }
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message && message.type === CAPTURE_MESSAGE) {
    handleCapture(message as CaptureRequestMessage).then(sendResponse);
    return true; // keep the channel open for the async response
  }
  return false;
});

console.debug("magpie: background service worker ready");
