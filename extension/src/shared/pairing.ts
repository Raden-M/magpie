// Pairing: turning what someone types into the options page into a backend URL,
// a token, and the host permission needed to reach it.
//
// Deliberately free of imports. These are pure string functions, and keeping
// them away from the storage layer (which needs the `browser`/`chrome` globals)
// is what lets `node --test` exercise them directly.

/** Normalize a base URL: trim, drop a trailing slash. */
export function normalizeBackendUrl(url: string): string {
  return url.trim().replace(/\/+$/, "");
}

export interface Pairing {
  backendUrl: string;
  token?: string;
}

/**
 * Split a pasted value into a backend URL and, if it carries one, a token.
 *
 * The backend prints `https://host:port#token` at startup so pairing is one
 * paste instead of two fields copied separately. A fragment is never sent to a
 * server, so the token cannot leak by pasting the string somewhere it doesn't
 * belong. A plain URL parses to itself, which is what a local user types.
 */
export function parseConnectString(value: string): Pairing {
  const trimmed = value.trim();
  const hash = trimmed.indexOf("#");
  if (hash === -1) return { backendUrl: normalizeBackendUrl(trimmed) };

  const token = trimmed.slice(hash + 1).trim();
  return {
    backendUrl: normalizeBackendUrl(trimmed.slice(0, hash)),
    ...(token ? { token } : {}),
  };
}

/**
 * The host permission covering a backend, as an origin match pattern.
 *
 * Returns null for anything unparseable, so a half-typed URL asks for nothing.
 * The pattern is per-origin rather than the wildcards the manifest declares as
 * optional: the extension only ever needs to reach the one server it is paired
 * with, and a narrow grant is what the permission prompt should say.
 *
 * Also null for an IPv6 literal: the match-pattern grammar has no bracketed
 * host, so `http://[::1]:8787/*` is not a pattern a browser will accept. Better
 * to say so than to hand `permissions.request` something it throws on.
 */
export function originPatternFor(url: string): string | null {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return null;
    if (parsed.hostname.startsWith("[")) return null;
    return `${parsed.protocol}//${parsed.host}/*`;
  } catch {
    return null;
  }
}
