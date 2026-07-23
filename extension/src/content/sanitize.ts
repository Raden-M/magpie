// Sanitization + stable block ids. DOMPurify with a strict allowlist runs at
// capture time (docs/05, step 3); the backend still serves under a strict CSP
// regardless (defense in depth). Links are absolutized and marked external;
// deterministic data-mag-id ids back section anchors and the jump-to rail.

import DOMPurify from "dompurify";

const ALLOWED_TAGS = [
  "p", "br", "hr", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote",
  "ul", "ol", "li", "dl", "dt", "dd", "pre", "code", "kbd", "samp",
  "em", "strong", "i", "b", "u", "s", "mark", "small", "sub", "sup",
  "span", "a", "img", "figure", "figcaption", "picture", "source",
  "table", "thead", "tbody", "tfoot", "tr", "th", "td", "caption", "colgroup", "col",
  "article", "section", "header", "footer", "aside", "div", "main",
  "time", "abbr", "cite", "q", "del", "ins",
];
const ALLOWED_ATTR = [
  "href", "src", "srcset", "alt", "title", "colspan", "rowspan", "datetime",
  "lang", "dir", "cite", "start", "type",
];

// Mark all links external once, at module load.
let hookInstalled = false;
function installHook(): void {
  if (hookInstalled) return;
  DOMPurify.addHook("afterSanitizeAttributes", (node) => {
    if (node instanceof Element && node.tagName === "A" && node.hasAttribute("href")) {
      node.setAttribute("target", "_blank");
      node.setAttribute("rel", "noopener noreferrer");
    }
  });
  hookInstalled = true;
}

function absolutize(root: Element, baseUrl: string): void {
  const resolve = (value: string): string | null => {
    try {
      return new URL(value, baseUrl).href;
    } catch {
      return null;
    }
  };
  root.querySelectorAll("a[href]").forEach((el) => {
    const abs = resolve(el.getAttribute("href")!);
    if (abs) el.setAttribute("href", abs);
    else el.removeAttribute("href");
  });
  root.querySelectorAll("img[src]").forEach((el) => {
    const abs = resolve(el.getAttribute("src")!);
    if (abs) el.setAttribute("src", abs);
  });
}

/** Sanitize dirty HTML and return a detached container element holding it. */
export function sanitizeToRoot(dirtyHtml: string, baseUrl: string): HTMLElement {
  installHook();
  const clean = DOMPurify.sanitize(dirtyHtml, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    FORBID_ATTR: ["style", "onerror", "onload"],
  }) as string;
  const container = document.createElement("div");
  container.innerHTML = clean;
  absolutize(container, baseUrl);
  return container;
}

const BLOCK_SELECTOR =
  "h1,h2,h3,h4,h5,h6,p,li,blockquote,pre,figure,table,section,dd,figcaption";

// djb2 over the block's leading text - content-derived, so ids are stable
// across re-captures of unchanged content. A per-slug counter breaks ties.
function blockHash(text: string): string {
  const trimmed = text.trim().slice(0, 160);
  let h = 5381;
  for (let i = 0; i < trimmed.length; i++) {
    h = ((h << 5) + h + trimmed.charCodeAt(i)) >>> 0;
  }
  return "b" + h.toString(36);
}

/** Assign deterministic `data-mag-id` ids to block elements, in document order. */
export function assignBlockIds(root: Element): void {
  const seen = new Map<string, number>();
  root.querySelectorAll(BLOCK_SELECTOR).forEach((el) => {
    const base = blockHash(el.textContent ?? "");
    const n = seen.get(base) ?? 0;
    seen.set(base, n + 1);
    el.setAttribute("data-mag-id", n === 0 ? base : `${base}-${n}`);
  });
}
