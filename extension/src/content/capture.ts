// Assemble a capture from the live page: extract the reader view, store images,
// compute anchoring selectors, and package it for the service worker.

import { computeSelectors, normalizedTextOf } from "../shared/anchor.js";
import { blobToBase64, sha256Hex } from "../shared/bytes.js";
import type { CaptureRequestMessage } from "../shared/messages.js";
import type { CapturePayload } from "../shared/types.js";
import { canonicalUrl } from "../shared/url.js";
import { extractReader } from "./extract.js";
import { collectImages } from "./images.js";
import type { SelectionSnapshot } from "./select.js";

export interface CaptureInput {
  selection: SelectionSnapshot | null;
  color: string;
  note: string | null;
  maxImageBytes: number;
}

export async function buildCapture(input: CaptureInput): Promise<CaptureRequestMessage> {
  const extraction = extractReader(document, input.selection?.block ?? null);
  const images = await collectImages(extraction.contentRoot, input.maxImageBytes);
  const contentText = normalizedTextOf(extraction.contentRoot);
  const contentHash = await sha256Hex(contentText);

  const highlight = input.selection
    ? computeSelectors(
        extraction.contentRoot,
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
    capture_id: crypto.randomUUID(),
    url: document.location.href,
    canonical_url: canonicalUrl(document),
    page: {
      title: extraction.title,
      site_name: extraction.siteName,
      lang: extraction.lang,
    },
    snapshot: {
      content_html: extraction.contentHtml,
      content_text: contentText,
      byline: extraction.byline,
      excerpt: extraction.excerpt,
      word_count: extraction.wordCount,
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
