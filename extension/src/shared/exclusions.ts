// Excluded sites: where Magpie stays out of the way entirely - no pill, no
// context menu, no keyboard capture.
//
// An entry is a host, optionally `*.`-prefixed to cover subdomains, optionally
// carrying a port and a path prefix:
//
//   mail.google.com        that host, every page on it
//   *.example.com          example.com and any subdomain
//   example.com/app        that host, but only under /app - for a PWA that
//                          shares its origin with a site worth keeping
//   localhost:3000         a port only matters when the entry names one
//
// Deliberately free of imports, like `pairing.ts`, so `node --test` can run
// these against the source without the `browser`/`chrome` globals.

interface Rule {
  /** Lowercase host, with a port only when the entry named one. */
  host: string;
  matchesSubdomains: boolean;
  hasPort: boolean;
  /** Path prefix with no trailing slash, or null for the whole host. */
  path: string | null;
}

/**
 * Clean up one line of user input, or return null if nothing is left.
 *
 * Accepts a pasted URL as readily as a bare host: people copy from the address
 * bar, and `https://example.com/app?q=1` clearly means `example.com/app`.
 */
export function normalizeExclusion(entry: string): string | null {
  let value = entry.trim();
  if (!value || value.startsWith("#")) return null;

  value = value.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "").replace(/^\/\//, "");
  value = value.split(/[?#]/)[0];

  const slash = value.indexOf("/");
  const host = (slash === -1 ? value : value.slice(0, slash)).toLowerCase();
  const path = slash === -1 ? "" : value.slice(slash).replace(/\/+$/, "");
  if (!host || host === "*" || host.startsWith(".")) return null;
  if (host.includes("*") && !host.startsWith("*.")) return null;

  return host + path;
}

function parseRule(entry: string): Rule | null {
  const normalized = normalizeExclusion(entry);
  if (!normalized) return null;

  const slash = normalized.indexOf("/");
  let host = slash === -1 ? normalized : normalized.slice(0, slash);
  const path = slash === -1 ? null : normalized.slice(slash);

  const matchesSubdomains = host.startsWith("*.");
  if (matchesSubdomains) host = host.slice(2);
  if (!host) return null;

  // An IPv6 literal keeps its brackets, so only look for a port after them.
  const afterHost = host.startsWith("[") ? host.indexOf("]") + 1 : 0;
  const hasPort = afterHost < host.length && host.indexOf(":", afterHost) !== -1;

  return { host, matchesSubdomains, hasPort, path };
}

/** Parse the options page's textarea into a stored list: one entry per line, deduped. */
export function parseExclusions(text: string): string[] {
  const seen = new Set<string>();
  for (const line of text.split("\n")) {
    const entry = normalizeExclusion(line);
    if (entry) seen.add(entry);
  }
  return [...seen];
}

/** Render a stored list back into the textarea. */
export function formatExclusions(entries: readonly string[]): string {
  return entries.join("\n");
}

function hostMatches(rule: Rule, url: URL): boolean {
  const host = (rule.hasPort ? url.host : url.hostname).toLowerCase();
  if (host === rule.host) return true;
  return rule.matchesSubdomains && host.endsWith(`.${rule.host}`);
}

function pathMatches(rule: Rule, url: URL): boolean {
  if (!rule.path) return true;
  return url.pathname === rule.path || url.pathname.startsWith(`${rule.path}/`);
}

function parseUrl(url: string): URL | null {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:" ? parsed : null;
  } catch {
    return null;
  }
}

/** True when Magpie should stay silent on this page. */
export function isExcluded(url: string, entries: readonly string[]): boolean {
  const parsed = parseUrl(url);
  if (!parsed) return false;
  return entries.some((entry) => {
    const rule = parseRule(entry);
    return !!rule && hostMatches(rule, parsed) && pathMatches(rule, parsed);
  });
}

/**
 * The entry that excluding this page from the context menu should add: its
 * host, port included only when it isn't the default one.
 *
 * Host-wide rather than path-scoped on purpose - "not on this site" is the
 * common case, and narrowing it to a path is a deliberate edit in the options
 * page. Null for anything Magpie doesn't run on anyway.
 */
export function exclusionFor(url: string): string | null {
  const parsed = parseUrl(url);
  return parsed ? parsed.host.toLowerCase() : null;
}

/**
 * Drop every entry covering this page, so "offer Magpie here again" undoes a
 * wildcard entry too - not just the host the context menu would have added.
 */
export function withoutExclusionsFor(url: string, entries: readonly string[]): string[] {
  return entries.filter((entry) => !isExcluded(url, [entry]));
}
