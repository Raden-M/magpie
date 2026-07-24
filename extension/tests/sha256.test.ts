// The pure-JS SHA-256 fallback (used on non-secure-context http:// pages, where
// crypto.subtle is unavailable) must produce identical digests to the real
// thing. Checked against FIPS vectors and Node's crypto.

import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { sha256HexPure } from "../src/shared/bytes.ts";

test("matches known FIPS-180-4 vectors", () => {
  assert.equal(
    sha256HexPure(""),
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  );
  assert.equal(
    sha256HexPure("abc"),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
  assert.equal(
    sha256HexPure("The quick brown fox jumps over the lazy dog"),
    "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592",
  );
});

test("matches Node crypto across lengths (block-boundary padding)", () => {
  for (const len of [0, 1, 55, 56, 63, 64, 65, 200, 1000]) {
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) bytes[i] = (i * 31 + 7) & 0xff;
    const expected = createHash("sha256").update(bytes).digest("hex");
    assert.equal(sha256HexPure(bytes.buffer), expected, `length ${len}`);
  }
});
