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

/**
 * Sent from the content script of the *visible* page to the service worker, so
 * the context menu can name what clicking it will do.
 *
 * The page reports its own state rather than the worker reading `tab.url`:
 * that would need the `tabs` permission, whose install warning ("read your
 * browsing history") is a steep price for a menu title.
 */
export interface PageStateMessage {
  type: "magpie-page-state";
  excluded: boolean;
}
export const PAGE_STATE = "magpie-page-state" as const;
