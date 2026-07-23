// Message protocol between the content script (which builds a capture from the
// live page) and the service worker (which owns the backend connection).

import type { CapturePayload, CaptureResponse } from "./types.js";

export interface SerializedImage {
  hash: string;
  mime: string;
  base64: string;
}

export interface CaptureRequestMessage {
  type: "magpie-capture";
  payload: CapturePayload;
  images: SerializedImage[];
}

export type CaptureResultMessage =
  | { ok: true; response: CaptureResponse }
  | { ok: false; error: string };

export const CAPTURE_MESSAGE = "magpie-capture" as const;

/** Sent from the service worker to the content script to trigger a capture of
 *  the current selection (from the context menu or keyboard command). */
export interface TriggerCaptureMessage {
  type: "magpie-trigger-capture";
}
export const TRIGGER_CAPTURE = "magpie-trigger-capture" as const;
