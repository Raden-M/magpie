// Image capture. To survive link rot, images are stored, not hot-linked: we
// collect them from the sanitized content, fetch each as a blob, content-hash
// it, and rewrite the src to /media/<hash> (docs/05, step 5). Failures degrade
// to a broken/blocked image, never a failed capture.
//
// Fetches run in the page's context, so cross-origin images without permissive
// CORS may not be retrievable in the MVP; those are simply left unstored.

export interface CollectedImage {
  hash: string;
  blob: Blob;
  mime: string;
  byteSize: number;
}

const DEFAULT_MAX_COUNT = 60;

async function sha256Hex(buffer: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Choose a single URL for an image, preferring an explicit src over srcset. */
function pickSource(img: HTMLImageElement): string | null {
  const src = img.getAttribute("src");
  if (src && src.trim()) return src.trim();

  const srcset = img.getAttribute("srcset");
  if (srcset) {
    const candidates = srcset
      .split(",")
      .map((part) => part.trim().split(/\s+/)[0])
      .filter(Boolean);
    if (candidates.length > 0) return candidates[candidates.length - 1];
  }
  return null;
}

async function fetchImage(url: string, maxBytes: number): Promise<CollectedImage | null> {
  let response: Response;
  try {
    response = await fetch(url);
  } catch {
    return null;
  }
  if (!response.ok) return null;

  const blob = await response.blob();
  if (blob.size === 0 || blob.size > maxBytes) return null;

  // Skip tracking pixels.
  try {
    const bitmap = await createImageBitmap(blob);
    const tiny = bitmap.width <= 1 && bitmap.height <= 1;
    bitmap.close();
    if (tiny) return null;
  } catch {
    // Undecodable here (e.g. SVG in some engines) - keep it; the backend serves
    // it verbatim.
  }

  const hash = await sha256Hex(await blob.arrayBuffer());
  return { hash, blob, mime: blob.type || "application/octet-stream", byteSize: blob.size };
}

/**
 * Collect, fetch, hash, and rewrite every image under `root`. Mutates `root`:
 * stored images point at /media/<hash> with srcset removed. Returns the unique
 * blobs to upload.
 */
export async function collectImages(
  root: HTMLElement,
  maxImageBytes: number,
  maxCount: number = DEFAULT_MAX_COUNT,
): Promise<CollectedImage[]> {
  const images = Array.from(root.querySelectorAll("img"));
  const byHash = new Map<string, CollectedImage>();
  const urlToHash = new Map<string, string>();
  let stored = 0;

  for (const img of images) {
    img.removeAttribute("srcset");
    const url = pickSource(img);
    if (!url) continue;

    let hash = urlToHash.get(url) ?? null;
    if (hash === null) {
      if (stored >= maxCount) continue;
      const collected = await fetchImage(url, maxImageBytes);
      if (!collected) continue;
      hash = collected.hash;
      urlToHash.set(url, hash);
      if (!byHash.has(hash)) {
        byHash.set(hash, collected);
        stored++;
      }
    }
    img.setAttribute("src", `/media/${hash}`);
  }

  return Array.from(byHash.values());
}
