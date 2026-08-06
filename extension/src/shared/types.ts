// Wire types shared across the extension, mirroring the backend API.
// Field names match the JSON exactly.

export interface HighlightSelectors {
  quote_exact: string;
  quote_prefix: string;
  quote_suffix: string;
  pos_start: number | null;
  pos_end: number | null;
  section_anchor: string | null;
  color: string;
  note: string | null;
}

export interface MediaManifestEntry {
  hash: string;
  mime: string;
  byte_size: number;
}

export interface CapturePayload {
  capture_id: string;
  url: string;
  canonical_url: string;
  page: {
    title: string;
    site_name?: string | null;
    lang?: string | null;
    favicon_hash?: string | null;
  };
  snapshot: {
    content_html: string;
    content_text: string;
    byline?: string | null;
    excerpt?: string | null;
    word_count?: number | null;
    content_hash: string;
    partial: boolean;
  };
  highlight?: HighlightSelectors | null;
  media: MediaManifestEntry[];
}

export interface CaptureResponse {
  page: { id: string; canonical_url: string; title: string; highlight_count: number };
  snapshot: { id: string; version: number };
  highlight: { id: string } | null;
}

/** A ready-to-send capture: the payload plus the image blobs it references. */
export interface AssembledCapture {
  payload: CapturePayload;
  images: { hash: string; blob: Blob }[];
}

export interface Settings {
  backendUrl: string;
  token: string;
  defaultShade: string;
  /** Sites Magpie stays out of entirely. Syntax in `shared/exclusions.ts`. */
  excludedSites: string[];
}
