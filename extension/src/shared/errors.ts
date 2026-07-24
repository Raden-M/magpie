// Reading the message off a thrown value.
//
// `instanceof Error` is unreliable in a Firefox content script: the script runs
// in its own sandbox realm, so exceptions raised by DOM and WebExtension APIs
// carry that realm's Error/DOMException constructor and the check is false even
// for a perfectly good error. Chromium's isolated world shares our constructors,
// which is why the same code reports a real message there and a generic one on
// Firefox. Duck-typing the shape works in both.

export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object") {
    const { name, message } = err as { name?: unknown; message?: unknown };
    if (typeof message === "string" && message !== "") {
      // DOMExceptions carry the useful part in the name ("NetworkError"), so
      // keep it; a plain "Error" prefix would only be noise.
      return typeof name === "string" && name !== "" && name !== "Error"
        ? `${name}: ${message}`
        : message;
    }
  }
  return String(err);
}
