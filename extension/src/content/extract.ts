// Reader-view extraction. Runs Readability on a *clone* of the document (it
// mutates its input), then sanitizes and assigns block ids. Falls back to a
// selection-scoped capture so a highlight is never lost on a non-article page
// (docs/05, step 2).

import { Readability } from "@mozilla/readability";

import { assignBlockIds, sanitizeToRoot } from "./sanitize.js";

export interface Extraction {
  title: string;
  byline: string | null;
  siteName: string | null;
  lang: string | null;
  excerpt: string | null;
  /** Sanitized, id-assigned reader HTML (the snapshot). */
  contentHtml: string;
  /** The same content as a live DOM root, for anchoring + normalization. */
  contentRoot: HTMLElement;
  wordCount: number;
  /** True when Readability failed and we captured a fallback block. */
  partial: boolean;
}

export function extractReader(doc: Document, fallbackBlock?: Element | null): Extraction {
  let title = doc.title || "";
  let byline: string | null = null;
  let siteName: string | null = null;
  let excerpt: string | null = null;
  let lang: string | null = doc.documentElement.getAttribute("lang");
  let dirtyHtml: string | null = null;

  try {
    const clone = doc.cloneNode(true) as Document;
    const article = new Readability(clone).parse();
    if (article && article.content) {
      dirtyHtml = article.content;
      title = article.title || title;
      byline = article.byline || null;
      siteName = article.siteName || null;
      excerpt = article.excerpt || null;
      lang = article.lang || lang;
    }
  } catch {
    // Fall through to the fallback capture below.
  }

  let partial = false;
  if (!dirtyHtml) {
    partial = true;
    const block = fallbackBlock ?? doc.body;
    dirtyHtml = block ? block.outerHTML : "<article></article>";
  }

  const root = sanitizeToRoot(dirtyHtml, doc.baseURI);
  assignBlockIds(root);

  const text = (root.textContent ?? "").trim();
  const wordCount = text ? text.split(/\s+/).length : 0;

  return {
    title: title.trim() || "Untitled",
    byline,
    siteName,
    lang,
    excerpt,
    contentHtml: root.innerHTML,
    contentRoot: root,
    wordCount,
    partial,
  };
}
