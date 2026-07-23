// Client-side URL canonicalization. Prefers <link rel="canonical">, else
// normalizes the location. The backend re-validates independently, so this only
// needs to be a good-faith guess (docs/03-data-model.md).

const TRACKING_EXACT = new Set([
  "gclid", "fbclid", "ref", "ref_src", "ref_url", "igshid", "yclid", "dclid",
  "msclkid", "_hsenc", "_hsmi", "vero_id", "spm",
]);
const TRACKING_PREFIX = ["utm_", "mc_", "mkt_", "pk_", "piwik_", "hsa_"];

function isTracking(key: string): boolean {
  const k = key.toLowerCase();
  return TRACKING_EXACT.has(k) || TRACKING_PREFIX.some((p) => k.startsWith(p));
}

export function normalizeUrl(input: string): string | null {
  let url: URL;
  try {
    url = new URL(input);
  } catch {
    return null;
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") return null;

  url.hash = "";
  if (
    (url.protocol === "https:" && url.port === "443") ||
    (url.protocol === "http:" && url.port === "80")
  ) {
    url.port = "";
  }

  const kept: [string, string][] = [];
  for (const [key, value] of url.searchParams) {
    if (!isTracking(key)) kept.push([key, value]);
  }
  kept.sort((a, b) => (a[0] === b[0] ? a[1].localeCompare(b[1]) : a[0].localeCompare(b[0])));
  url.search = "";
  for (const [key, value] of kept) url.searchParams.append(key, value);

  if (url.pathname.length > 1 && url.pathname.endsWith("/")) {
    url.pathname = url.pathname.replace(/\/+$/, "");
  }
  return url.toString();
}

export function canonicalUrl(doc: Document): string {
  const link = doc.querySelector<HTMLLinkElement>('link[rel="canonical"]');
  const href = doc.location.href;
  const candidate = link?.href && link.href.trim() ? link.href : href;
  return normalizeUrl(candidate) ?? normalizeUrl(href) ?? href;
}
