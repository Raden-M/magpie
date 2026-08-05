// Backend HTTP client. Only the service worker calls this (it holds the base URL
// + token and is allowed the cross-origin request).

import { normalizeBackendUrl } from "./pairing.js";
import type { AssembledCapture, CaptureResponse, Settings } from "./types.js";

export class CaptureError extends Error {
  constructor(
    message: string,
    readonly status?: number,
  ) {
    super(message);
    this.name = "CaptureError";
  }
}

/** POST an assembled capture as multipart/form-data and return the result. */
export async function postCapture(
  settings: Settings,
  capture: AssembledCapture,
): Promise<CaptureResponse> {
  const base = normalizeBackendUrl(settings.backendUrl);
  if (!base) {
    throw new CaptureError("No backend URL configured. Open Magpie's options to set one.");
  }

  const form = new FormData();
  form.append(
    "payload",
    new Blob([JSON.stringify(capture.payload)], { type: "application/json" }),
  );
  for (const image of capture.images) {
    form.append(`image/${image.hash}`, image.blob);
  }

  const headers: Record<string, string> = {};
  if (settings.token) {
    headers["Authorization"] = `Bearer ${settings.token}`;
  }

  let response: Response;
  try {
    response = await fetch(`${base}/api/captures`, { method: "POST", headers, body: form });
  } catch (err) {
    // Indistinguishable from here: the backend is down, or the host permission
    // for it was never granted. Name both, since re-saving the options is the
    // fix for the second and isn't an obvious thing to try.
    throw new CaptureError(
      `Couldn't reach Magpie at ${base}. Check the backend is running, and open ` +
        `Magpie's options and hit Save to grant access to it.`,
    );
  }

  if (!response.ok) {
    let message = `Magpie returned ${response.status}.`;
    try {
      const body = (await response.json()) as { error?: { message?: string } };
      if (body.error?.message) message = body.error.message;
    } catch {
      // non-JSON error body; keep the status message
    }
    if (response.status === 401) {
      message = "Magpie rejected the token. Check it in the extension options.";
    }
    if (response.status === 421) {
      message =
        "Magpie doesn't answer to that hostname. Set MAGPIE_PUBLIC_URL on the server to the " +
        "address you reach it at.";
    }
    throw new CaptureError(message, response.status);
  }

  return (await response.json()) as CaptureResponse;
}
