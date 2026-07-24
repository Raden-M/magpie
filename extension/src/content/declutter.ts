// Post-extraction declutter. Readability keeps the article body, but on some
// pages (Wikipedia infoboxes/taxoboxes, nav widgets) that body still carries
// page furniture: single-character link stubs, orphaned figure captions, and
// images that couldn't be stored locally. Served under a strict CSP, an image
// whose src still points off-origin renders as a broken icon, so we drop it -
// and the caption left behind with it. This pass runs after image collection,
// so it can tell a stored image (/media/<hash>) from one that will never load.

const PRUNABLE =
  "p,li,dd,dt,td,th,figure,figcaption,blockquote,section,div,ul,ol,dl,table,h1,h2,h3,h4,h5,h6";

function hasImage(el: Element): boolean {
  return el.querySelector("img") !== null;
}

function collapsedText(el: Element): string {
  return (el.textContent ?? "").replace(/\s+/g, " ").trim();
}

function isClutter(el: Element): boolean {
  if (el.tagName === "FIGURE") {
    // Figures can wrap code, tables, or quotes - keep those. Only an image-less,
    // caption-only figure is an orphan left behind by a dropped image.
    if (hasImage(el)) return false;
    return el.querySelector("pre,code,table,blockquote,ul,ol,dl") === null;
  }
  if (hasImage(el)) return false;
  const text = collapsedText(el);
  if (text === "") return true; // empty wrapper or block
  // A block whose entire content is a one/two-character link is a nav stub
  // (period bars, alphabet indices), never prose.
  if (text.length <= 2 && el.querySelector("a")) return true;
  return false;
}

/** Remove non-content residue left by extraction. Mutates `root` in place. */
export function declutter(root: Element): void {
  // Drop images that weren't stored: their original cross-origin src can't load
  // under the reader's CSP, so they'd only ever render as broken icons.
  root.querySelectorAll("img").forEach((img) => {
    const src = img.getAttribute("src") ?? "";
    if (!src.startsWith("/media/") && !src.startsWith("data:")) img.remove();
  });
  // <source> carries only off-origin srcset; a <picture> with no <img> left is
  // dead weight.
  root.querySelectorAll("source").forEach((el) => el.remove());
  root.querySelectorAll("picture").forEach((el) => {
    if (!hasImage(el)) el.remove();
  });

  // Prune clutter deepest-first, repeating until stable so a wrapper emptied by
  // one removal is caught on the next pass.
  let changed = true;
  while (changed) {
    changed = false;
    const candidates = Array.from(root.querySelectorAll(PRUNABLE)).reverse();
    for (const el of candidates) {
      if (el.isConnected && isClutter(el)) {
        el.remove();
        changed = true;
      }
    }
  }

  // Collapse runs of <br> left where inline content was stripped around them.
  root.querySelectorAll("br + br").forEach((el) => el.remove());
}
