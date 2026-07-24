// Assemble a capture from the live page: extract the reader view, store images,
// compute anchoring selectors, and package it for the service worker.

import { computeSelectors, normalizedTextOf } from "../shared/anchor.js";
import { blobToBase64, randomId, sha256Hex } from "../shared/bytes.js";
import type { CaptureRequestMessage } from "../shared/messages.js";
import type { CapturePayload } from "../shared/types.js";
import { canonicalUrl } from "../shared/url.js";
import { declutter } from "./declutter.js";
import { extractReader } from "./extract.js";
import { collectImages } from "./images.js";
import { assignBlockIds } from "./sanitize.js";
import type { SelectionSnapshot } from "./select.js";

function countWords(root: Element): number {
  const text = (root.textContent ?? "").trim();
  return text ? text.split(/\s+/).length : 0;
}

export interface CaptureInput {
  selection: SelectionSnapshot | null;
  color: string;
  note: string | null;
  maxImageBytes: number;
}

export async function buildCapture(input: CaptureInput): Promise<CaptureRequestMessage> {
  const extraction = extractReader(document, input.selection?.block ?? null);
  // Store images first (rewrites kept ones to /media/<hash>), then declutter so
  // it can drop the ones that will never load, and only then freeze the block
  // ids, text, and HTML - all against the same, final content.
  const root = extraction.contentRoot;
  const images = await collectImages(root, input.maxImageBytes);
  declutter(root);
  assignBlockIds(root);

  const contentHtml = root.innerHTML;
  const contentText = normalizedTextOf(root);
  const contentHash = await sha256Hex(contentText);

  const highlight = input.selection
    ? computeSelectors(
        root,
        {
          quote: input.selection.quote,
          prefix: input.selection.prefix,
          suffix: input.selection.suffix,
        },
        input.color,
        input.note,
      )
    : null;

  const payload: CapturePayload = {
    capture_id: randomId(),
    url: document.location.href,
    canonical_url: canonicalUrl(document),
    page: {
      title: extraction.title,
      site_name: extraction.siteName,
      lang: extraction.lang,
    },
    snapshot: {
      content_html: contentHtml,
      content_text: contentText,
      byline: extraction.byline,
      excerpt: extraction.excerpt,
      word_count: countWords(root),
      content_hash: contentHash,
      partial: extraction.partial,
    },
    highlight,
    media: images.map((img) => ({ hash: img.hash, mime: img.mime, byte_size: img.byteSize })),
  };

  const serializedImages = await Promise.all(
    images.map(async (img) => ({
      hash: img.hash,
      mime: img.mime,
      base64: await blobToBase64(img.blob),
    })),
  );

  return { type: "magpie-capture", payload, images: serializedImages };
}
